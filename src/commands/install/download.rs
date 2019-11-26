use std::{
    fs::{create_dir_all, File},
    io::{BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use failure::format_err;
use indicatif::{ProgressBar, ProgressStyle};
use semver::Version;
use url::Url;

use crate::{os::build_filename, utils, Result};

pub fn download_from_url<P: AsRef<Path>>(url: &Url, download_to: P) -> Result<()> {
    let line_header = "[1/15] Download";

    let download_to = download_to.as_ref();

    if !utils::path_exists(&download_to) {
        log::debug!("Directory {:?} does not exists. Creating.", download_to);
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
        println!(
            "  {} skipped: file {} already downloaded.",
            line_header, filename
        );
        Ok(())
    } else {
        log::info!("Downloading {}...", url);

        let mut resp = reqwest::get(url.as_str())?;

        if resp.status().is_success() {
            let headers = resp.headers().clone();
            let ct_len = match headers.get(reqwest::header::CONTENT_LENGTH).cloned() {
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

            let chunk_size = match ct_len {
                Some(x) => x / 99_u64,
                None => 1024_u64, // default chunk size
            } as usize;

            let message = format!("{}ing {:?}...", line_header, filename);

            let pb = create_progress_bar(&message, ct_len);

            let mut out = BufWriter::new(File::create(file_path)?);

            loop {
                let mut buffer = vec![0; chunk_size];
                let bcount = resp.read(&mut buffer[..])?;
                buffer.truncate(bcount);
                if buffer.is_empty() {
                    break;
                } else {
                    out.write_all(&buffer)?;
                    pb.inc(bcount as u64);
                }
            }

            pb.finish();

            Ok(())
        } else {
            log::error!("Failed to download {}: {:?}", resp.url(), resp.status());
            let res = resp.error_for_status();
            res.map(|_| ())
                .map_err(|e| format_err!("Failed to download file: {:?}", e))
        }
    }
}

pub fn download_source(version: &Version) -> Result<()> {
    let url = build_url(&version)?;
    let download_dir = utils::directory::downloaded()?;
    download_from_url(&url, download_dir)
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

fn create_progress_bar(msg: &str, length: Option<u64>) -> ProgressBar {
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

    #[test]
    fn build_url_from_version_372() {
        let version = Version::parse("3.7.2").unwrap();

        let url = build_url(&version).unwrap();

        #[cfg(not(target_os = "windows"))]
        {
            assert_eq!(
                url,
                Url::parse("https://www.python.org/ftp/python/3.7.2/Python-3.7.2.tgz").unwrap()
            );
        }
        #[cfg(target_os = "windows")]
        {
            assert_eq!(
                url,
                Url::parse("https://www.python.org/ftp/python/3.7.2/python-3.7.2-amd64.exe")
                    .unwrap()
            );
        }
    }

    #[test]
    fn build_url_from_version_372rc1() {
        let version = Version::parse("3.7.2-rc1").unwrap();

        let url = build_url(&version).unwrap();

        #[cfg(not(target_os = "windows"))]
        {
            assert_eq!(
                url,
                Url::parse("https://www.python.org/ftp/python/3.7.2/Python-3.7.2rc1.tgz").unwrap()
            );
        }
        #[cfg(target_os = "windows")]
        {
            assert_eq!(
                url,
                Url::parse("https://www.python.org/ftp/python/3.7.2/python-3.7.2rc1-amd64.exe")
                    .unwrap()
            );
        }
    }
}
