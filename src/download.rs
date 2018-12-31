// https://www.python.org/ftp/python/
// https://www.python.org/ftp/python/3.7.2/Python-3.7.2.tgz

use std::{
    fs::{create_dir_all, File},
    io::{BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use failure::format_err;
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, error, info, warn};
use semver::Version;
use url::Url;

use crate::{utils, Result};

pub fn download_from_url<P: AsRef<Path>>(url: &Url, download_to: P) -> Result<()> {
    let download_to = download_to.as_ref();

    if !utils::path_exists(&download_to) {
        debug!("Directory {:?} does not exists. Creating.", download_to);
        create_dir_all(&download_to)?;
    }

    let filename = url
        .path_segments()
        .ok_or_else(|| format_err!("Could not extract filename from url"))?
        .last()
        .ok_or_else(|| format_err!("Could not get last segment from url path"))?
        .to_string();

    let mut file_path = PathBuf::new();
    file_path.push(download_to);
    file_path.push(&filename);

    if file_path.exists() {
        info!("File {} already downloaded. Skipping.", filename);
        Ok(())
    } else {
        info!("Downloading {}...", url);

        let mut resp = reqwest::get(url.as_str())?;

        if resp.status().is_success() {
            let headers = resp.headers().clone();
            let ct_len = match headers
                .get(reqwest::header::CONTENT_LENGTH)
                .map(|ct_len| ct_len.clone())
            {
                Some(ct_len) => {
                    let ct_len: u64 = ct_len.to_str()?.parse()?;
                    info!("Downloading {} bytes...", ct_len);
                    Some(ct_len)
                }
                None => {
                    warn!("Could not find out file size");
                    None
                }
            };

            let chunk_size = match ct_len {
                Some(x) => x / 99_u64,
                None => 1024_u64, // default chunk size
            } as usize;

            let bar = create_progress_bar(&filename, ct_len);

            let mut out = BufWriter::new(File::create(file_path)?);

            loop {
                let mut buffer = vec![0; chunk_size];
                let bcount = resp.read(&mut buffer[..])?;
                buffer.truncate(bcount);
                if buffer.is_empty() {
                    break;
                } else {
                    out.write_all(&mut buffer)?;
                    bar.inc(bcount as u64);
                }
            }

            bar.finish();

            Ok(())
        } else {
            error!("Failed to download {}: {:?}", resp.url(), resp.status());
            let res = resp.error_for_status();
            res.map(|_| ())
                .map_err(|e| format_err!("Failed to download file: {:?}", e))
        }
    }
}

pub fn download_source(version: &Version) -> Result<()> {
    let url = build_url(&version)?;
    let download_dir = utils::pycors_home()?.join("downloads");
    download_from_url(&url, download_dir)
}

fn build_url(version: &Version) -> Result<Url> {
    let main_version = format!("{}.{}", version.major, version.minor);
    let version_path = if version.patch == 0 {
        main_version.clone()
    } else {
        format!("{}.{}", main_version, version.patch)
    };
    let version_file = format!("{}", version).replace("-", "");

    let to_download = Url::parse(&format!(
        "https://www.python.org/ftp/python/{}/Python-{}.tgz",
        version_path, version_file
    ))?;

    Ok(to_download)
}

fn create_progress_bar(msg: &str, length: Option<u64>) -> ProgressBar {
    let bar = match length {
        Some(len) => ProgressBar::new(len),
        None => ProgressBar::new_spinner(),
    };

    bar.set_message(msg);
    match length.is_some() {
        true => bar
            .set_style(ProgressStyle::default_bar()
                .template("{msg} {spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} eta: {eta}")
                .progress_chars("=> ")),
        false => bar.set_style(ProgressStyle::default_spinner()),
    };

    bar
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_url_from_version_372() {
        let version = Version::parse("3.7.2").unwrap();

        let url = build_url(&version).unwrap();

        assert_eq!(
            url,
            Url::parse("https://www.python.org/ftp/python/3.7.2/Python-3.7.2.tgz").unwrap()
        );
    }

    #[test]
    fn build_url_from_version_372rc1() {
        let version = Version::parse("3.7.2-rc1").unwrap();

        let url = build_url(&version).unwrap();

        assert_eq!(
            url,
            Url::parse("https://www.python.org/ftp/python/3.7.2/Python-3.7.2rc1.tgz").unwrap()
        );
    }
}
