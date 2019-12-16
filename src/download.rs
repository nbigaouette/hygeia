use std::{
    fs::{create_dir_all, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use anyhow::Context;
use async_trait::async_trait;
use hyper::{body::HttpBody as _, Client, Uri};
use hyper_tls::HttpsConnector;
use indicatif::{ProgressBar, ProgressStyle};
use url::Url;

use crate::{
    utils::{self, directory::PycorsPathsProviderFromEnv},
    Result,
};

#[async_trait]
pub trait Downloader {
    async fn get(&mut self, url: &Url) -> Result<()>;
    async fn next_chunk(&mut self) -> Option<Result<bytes::Bytes>>;
}
#[cfg(test)]
use std::future::Future;
#[cfg(test)]
use std::pin::Pin;

#[cfg(test)]
mockall::mock! {
    Downloader {
        fn get(&mut self, url: &Url) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>;
        fn next_chunk(&mut self) -> Pin<Box<dyn Future<Output = Option<Result<bytes::Bytes>>> + Send>>;
    }
}

pub struct DownloaderOnline {
    client: hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>,
    response: Option<hyper::Response<hyper::Body>>,
    content_length: Option<u64>,
}

impl DownloaderOnline {
    pub fn new() -> DownloaderOnline {
        let https = {
            let mut connector = HttpsConnector::new();
            connector.https_only(true);
            connector
        };
        let client = Client::builder().build::<_, hyper::Body>(https);
        DownloaderOnline {
            client,
            response: None,
            content_length: None,
        }
    }
}

#[async_trait]
impl Downloader for DownloaderOnline {
    async fn get(&mut self, url: &Url) -> Result<()> {
        let uri = Uri::builder()
            .scheme(url.scheme())
            .authority(url.host_str().unwrap())
            .path_and_query(url.path())
            .build()?;
        let response = self.client.get(uri).await?;

        let headers = response.headers().clone();
        let content_length = match headers.get(hyper::header::CONTENT_LENGTH).cloned() {
            Some(ct_len) => {
                let ct_len: u64 = ct_len.to_str()?.parse()?;
                log::debug!("Downloading {} bytes...", ct_len);
                Some(ct_len)
            }
            None => {
                log::warn!("Could not find out file size");
                None
            }
        };

        self.response = Some(response);
        self.content_length = content_length;

        Ok(())
    }

    async fn next_chunk(&mut self) -> Option<Result<bytes::Bytes>> {
        match &mut self.response {
            None => None,
            Some(response) => response
                .data()
                .await
                .map(|v| v.with_context(|| "Failed to get next chunk")),
        }
    }
}

pub async fn download_to_string<S>(url: S, with_progress_bar: bool) -> Result<String>
where
    S: AsRef<str>,
{
    let url: Url = url.as_ref().parse()?;
    Ok(String::from_utf8(download(&url, with_progress_bar).await?)?)
}

pub async fn download_source(url: &Url, with_progress_bar: bool) -> Result<()> {
    let download_dir = PycorsPathsProviderFromEnv::new().downloaded();
    download_to_path(url, download_dir, with_progress_bar).await
}

pub async fn download_to_path<S, P>(url: S, download_to: P, with_progress_bar: bool) -> Result<()>
where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    _download_to_path(url.as_ref(), download_to.as_ref(), with_progress_bar).await
}

async fn _download_to_path(url: &str, download_to: &Path, with_progress_bar: bool) -> Result<()> {
    if !utils::path_exists(&download_to) {
        log::debug!("Directory {:?} does not exists. Creating.", download_to);
        create_dir_all(&download_to)?;
    }

    let url: Url = url.parse()?;

    let filename = url
        .path_segments()
        .ok_or_else(|| anyhow::anyhow!("Could not extract filename from url"))?
        .last()
        .ok_or_else(|| anyhow::anyhow!("Could not get last segment from url path"))?
        .to_string();

    let mut file_path = PathBuf::new();
    file_path.push(download_to);
    file_path.push(&filename);

    if file_path.exists() {
        println!("skipped: file {} already downloaded.", filename);
        Ok(())
    } else {
        let downloaded_data = download(&url, with_progress_bar).await?;

        let mut output = BufWriter::new(File::create(&file_path)?);
        output.write_all(&downloaded_data)?;

        Ok(())
    }
}

async fn download(url: &Url, with_progress_bar: bool) -> Result<Vec<u8>> {
    // Based on: https://users.rust-lang.org/t/using-async-std-was-reqwest/32735/16
    log::info!("Downloading {}...", url);

    let https = {
        let mut connector = HttpsConnector::new();
        connector.https_only(true);
        connector
    };
    let client = Client::builder().build::<_, hyper::Body>(https);

    let uri = Uri::builder()
        .scheme(url.scheme())
        .authority(url.host_str().unwrap())
        .path_and_query(url.path())
        .build()?;
    let filename: &str = match url
        .path_segments()
        .ok_or_else(|| anyhow::anyhow!("cannot extract path segments from {:?}", url))?
        .last()
        .ok_or_else(|| anyhow::anyhow!("cannot extract filename from {:?}", url))?
    {
        "" => url.as_str(),
        filename => filename,
    };

    let mut response = client.get(uri).await?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to query {:?}: {:?}", url.host_str(), response);
    }

    // Create a progress bar
    let headers = response.headers().clone();
    let ct_len = match headers.get(hyper::header::CONTENT_LENGTH).cloned() {
        Some(ct_len) => {
            let ct_len: u64 = ct_len.to_str()?.parse()?;
            log::debug!("Downloading {} bytes...", ct_len);
            Some(ct_len)
        }
        None => {
            log::warn!("Could not find out file size");
            None
        }
    };

    let message = format!("Downloading {:?}...", filename);
    let pb = if with_progress_bar {
        Some(create_download_progress_bar(&message, ct_len))
    } else {
        None
    };

    let mut output: Vec<u8> = Vec::with_capacity(1024);
    while let Some(next) = response.data().await {
        let chunk = next?;
        if let Some(pb) = pb.as_ref() {
            pb.inc(chunk.len() as u64)
        }
        output.write_all(&chunk[..])?;
    }

    if let Some(pb) = pb {
        pb.finish()
    }

    Ok(output)
}

fn create_download_progress_bar(msg: &str, length: Option<u64>) -> ProgressBar {
    let pb = match length {
        Some(len) => ProgressBar::new(len),
        None => ProgressBar::new_spinner(),
    };

    pb.set_message(msg);
    if length.is_some() {
        pb
            .set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} {msg} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} eta: {eta}")
                .progress_chars("=> "));
    } else {
        pb.set_style(ProgressStyle::default_spinner());
    }

    pb
}

#[cfg(test)]
mod tests {
    use super::*;

    use bytes::Bytes;
    use mockall::Sequence;

    #[test]
    fn download_success_manual_next_chunk() {
        let mut mock_downloader = MockDownloader::default();
        let expected_downloaded_data: Vec<Bytes> = vec![
            Bytes::from_static(&[1, 2, 3, 4]),
            Bytes::from_static(&[5, 6, 7, 8]),
        ];
        let mut seq = Sequence::new();
        mock_downloader
            .expect_get()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Box::pin(futures::future::ready(Ok(()))));
        mock_downloader
            .expect_next_chunk()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|| {
                Box::pin(futures::future::ready(Some(Ok(Bytes::from_static(&[
                    1, 2, 3, 4,
                ])))))
            });
        mock_downloader
            .expect_next_chunk()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|| {
                Box::pin(futures::future::ready(Some(Ok(Bytes::from_static(&[
                    5, 6, 7, 8,
                ])))))
            });
        let url = Url::parse("https://example.com/dummmy.tar.gz").unwrap();
        let data: Vec<Bytes> = futures::executor::block_on(async {
            mock_downloader.get(&url).await.unwrap();
            vec![
                mock_downloader.next_chunk().await.unwrap().unwrap(),
                mock_downloader.next_chunk().await.unwrap().unwrap(),
            ]
        });
        assert_eq!(data, expected_downloaded_data);
    }
}
