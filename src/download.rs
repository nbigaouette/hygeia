use std::{
    fs::{create_dir_all, File},
    io::{BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use failure::format_err;
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, error, info, warn};
use regex::Regex;
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
                    debug!("Downloading {} bytes...", ct_len);
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
    let download_dir = utils::pycors_download()?;
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

pub fn find_all_python_versions() -> Result<Vec<Version>> {
    let index_html = reqwest::get("https://www.python.org/ftp/python/")?.text()?;

    parse_index_html(&index_html)
}

fn parse_index_html(index_html: &str) -> Result<Vec<Version>> {
    let re = Regex::new(r#"(?x)<a \s+ href="(?P<version>\d+[\d\.]+)/">"#)?;

    let r: Result<Vec<Version>> = re
        .captures_iter(index_html)
        .map(|caps| {
            let v = &caps["version"];
            // Add a `.0` for versions missing a patch number (f.e. `2.7`)
            let dots = v.chars().filter(|c| *c == '.').count();
            let v = if dots == 1 {
                format!("{}.0", v)
            } else {
                v.to_string()
            };
            Version::parse(&v).map_err(|e| format_err!("Failed to parse version: {}", e))
        })
        .collect();

    // Sort the versions vector (in reverse order)
    r.map(|mut versions| {
        versions.sort_unstable();
        versions.into_iter().rev().collect()
    })
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

    #[test]
    fn parse_html() {
        let index_html = include_str!("../tests/fixtures/index.html");

        let parsed = parse_index_html(index_html).unwrap();

        let expected: Vec<Version> = vec![
            "3.7.2".parse().unwrap(),
            "3.7.1".parse().unwrap(),
            "3.7.0".parse().unwrap(),
            "3.6.8".parse().unwrap(),
            "3.6.7".parse().unwrap(),
            "3.6.6".parse().unwrap(),
            "3.6.5".parse().unwrap(),
            "3.6.4".parse().unwrap(),
            "3.6.3".parse().unwrap(),
            "3.6.2".parse().unwrap(),
            "3.6.1".parse().unwrap(),
            "3.6.0".parse().unwrap(),
            "3.5.6".parse().unwrap(),
            "3.5.5".parse().unwrap(),
            "3.5.4".parse().unwrap(),
            "3.5.3".parse().unwrap(),
            "3.5.2".parse().unwrap(),
            "3.5.1".parse().unwrap(),
            "3.5.0".parse().unwrap(),
            "3.4.9".parse().unwrap(),
            "3.4.8".parse().unwrap(),
            "3.4.7".parse().unwrap(),
            "3.4.6".parse().unwrap(),
            "3.4.5".parse().unwrap(),
            "3.4.4".parse().unwrap(),
            "3.4.3".parse().unwrap(),
            "3.4.2".parse().unwrap(),
            "3.4.1".parse().unwrap(),
            "3.4.0".parse().unwrap(),
            "3.3.7".parse().unwrap(),
            "3.3.6".parse().unwrap(),
            "3.3.5".parse().unwrap(),
            "3.3.4".parse().unwrap(),
            "3.3.3".parse().unwrap(),
            "3.3.2".parse().unwrap(),
            "3.3.1".parse().unwrap(),
            "3.3.0".parse().unwrap(),
            "3.2.6".parse().unwrap(),
            "3.2.5".parse().unwrap(),
            "3.2.4".parse().unwrap(),
            "3.2.3".parse().unwrap(),
            "3.2.2".parse().unwrap(),
            "3.2.1".parse().unwrap(),
            "3.2.0".parse().unwrap(),
            "3.1.5".parse().unwrap(),
            "3.1.4".parse().unwrap(),
            "3.1.3".parse().unwrap(),
            "3.1.2".parse().unwrap(),
            "3.1.1".parse().unwrap(),
            "3.1.0".parse().unwrap(),
            "3.0.1".parse().unwrap(),
            "3.0.0".parse().unwrap(),
            "2.7.15".parse().unwrap(),
            "2.7.14".parse().unwrap(),
            "2.7.13".parse().unwrap(),
            "2.7.12".parse().unwrap(),
            "2.7.11".parse().unwrap(),
            "2.7.10".parse().unwrap(),
            "2.7.9".parse().unwrap(),
            "2.7.8".parse().unwrap(),
            "2.7.7".parse().unwrap(),
            "2.7.6".parse().unwrap(),
            "2.7.5".parse().unwrap(),
            "2.7.4".parse().unwrap(),
            "2.7.3".parse().unwrap(),
            "2.7.2".parse().unwrap(),
            "2.7.1".parse().unwrap(),
            "2.7.0".parse().unwrap(),
            "2.6.9".parse().unwrap(),
            "2.6.8".parse().unwrap(),
            "2.6.7".parse().unwrap(),
            "2.6.6".parse().unwrap(),
            "2.6.5".parse().unwrap(),
            "2.6.4".parse().unwrap(),
            "2.6.3".parse().unwrap(),
            "2.6.2".parse().unwrap(),
            "2.6.1".parse().unwrap(),
            "2.6.0".parse().unwrap(),
            "2.5.6".parse().unwrap(),
            "2.5.5".parse().unwrap(),
            "2.5.4".parse().unwrap(),
            "2.5.3".parse().unwrap(),
            "2.5.2".parse().unwrap(),
            "2.5.1".parse().unwrap(),
            "2.5.0".parse().unwrap(),
            "2.4.6".parse().unwrap(),
            "2.4.5".parse().unwrap(),
            "2.4.4".parse().unwrap(),
            "2.4.3".parse().unwrap(),
            "2.4.2".parse().unwrap(),
            "2.4.1".parse().unwrap(),
            "2.4.0".parse().unwrap(),
            "2.3.7".parse().unwrap(),
            "2.3.6".parse().unwrap(),
            "2.3.5".parse().unwrap(),
            "2.3.4".parse().unwrap(),
            "2.3.3".parse().unwrap(),
            "2.3.2".parse().unwrap(),
            "2.3.1".parse().unwrap(),
            "2.3.0".parse().unwrap(),
            "2.2.3".parse().unwrap(),
            "2.2.2".parse().unwrap(),
            "2.2.1".parse().unwrap(),
            "2.2.0".parse().unwrap(),
            "2.1.3".parse().unwrap(),
            "2.1.2".parse().unwrap(),
            "2.1.1".parse().unwrap(),
            "2.1.0".parse().unwrap(),
            "2.0.1".parse().unwrap(),
            "2.0.0".parse().unwrap(),
        ];
        assert_eq!(parsed, expected);
    }
}
