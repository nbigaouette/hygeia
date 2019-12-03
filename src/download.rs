use std::{
    fs::{create_dir_all, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use hyper::{Client, Uri};
use hyper_tls::HttpsConnector;
use indicatif::{ProgressBar, ProgressStyle};
use semver::Version;
use url::Url;

use crate::{os::build_filename, utils};

pub async fn download_to_string<S>(url: S) -> Result<String>
where
    S: AsRef<str>,
{
    let url: Url = url.as_ref().parse()?;
    Ok(String::from_utf8(download(&url).await?)?)
}

pub async fn download_source(version: &Version) -> Result<()> {
    let url = build_url(&version)?;
    let download_dir = utils::directory::downloaded()?;
    download_to_path(&url, download_dir).await
}

pub async fn download_to_path<S, P>(url: S, download_to: P) -> Result<()>
where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    _download_to_path(url.as_ref(), download_to.as_ref()).await
}

async fn _download_to_path(url: &str, download_to: &Path) -> Result<()> {
    if !utils::path_exists(&download_to) {
        log::debug!("Directory {:?} does not exists. Creating.", download_to);
        create_dir_all(&download_to)?;
    }

    let url: Url = url.parse()?;

    let filename = url
        .path_segments()
        .ok_or_else(|| anyhow!("Could not extract filename from url"))?
        .last()
        .ok_or_else(|| anyhow!("Could not get last segment from url path"))?
        .to_string();

    let mut file_path = PathBuf::new();
    file_path.push(download_to);
    file_path.push(&filename);

    if file_path.exists() {
        println!("skipped: file {} already downloaded.", filename);
        Ok(())
    } else {
        log::info!("Downloading {}...", url);

        let downloaded_data = download(&url).await?;

        let mut output = BufWriter::new(File::create(&file_path)?);
        output.write_all(&downloaded_data)?;

        Ok(())
    }
}

async fn download(url: &Url) -> Result<Vec<u8>> {
    // Based on: https://users.rust-lang.org/t/using-async-std-was-reqwest/32735/16
    log::info!("Downloading {}...", url);

    let https = {
        let mut connector = HttpsConnector::new().expect("TLS initialization failed");
        connector.https_only(true);
        connector
    };
    let client = Client::builder().build::<_, hyper::Body>(https);

    let uri = Uri::builder()
        .scheme(url.scheme())
        .authority(url.host_str().unwrap())
        .path_and_query(url.path())
        .build()?;
    log::debug!("uri: {}...", uri);

    let mut response = client.get(uri).await?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to query python.org: {:?}", response);
    }
    let body = response.body_mut();

    // FIXME: Send updates to indicatif (progress bar)
    let mut output: Vec<u8> = Vec::with_capacity(1024);
    while let Some(chunk) = body.next().await {
        let bytes = chunk?.into_bytes();
        output.write_all(&bytes[..])?;
    }

    Ok(output)
}

fn build_url(version: &Version) -> Result<Url> {
    // Starting with 3.3, the Url contains the full MAJOR.MINOR.PATCH (f.e. "3.3.0").
    // Before that, the Url only contained MAJOR.MINOR (without the patch, for example "3.2")
    // See directory listing in https://www.python.org/ftp/python/
    let version_path = if *version >= Version::new(3, 3, 0) {
        format!("{}.{}.{}", version.major, version.minor, version.patch)
    } else {
        format!("{}.{}", version.major, version.minor)
    };

    let filename = build_filename(&version)?;

    let to_download = Url::parse(&format!(
        "https://www.python.org/ftp/python/{}/{}",
        version_path, filename
    ))?;

    Ok(to_download)
}
