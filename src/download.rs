// https://www.python.org/ftp/python/
// https://www.python.org/ftp/python/3.7.2/Python-3.7.2.tgz

use std::{fs::File, io};

use failure::format_err;
use semver::Version;
use url::Url;

use crate::{utils, Result};

pub fn download_source(version: &Version) -> Result<()> {
    let url = build_url(&version).unwrap();

    let mut resp = reqwest::get(url.as_str())?;
    let filename = url
        .path_segments()
        .ok_or_else(|| format_err!("Could not extract filename from url"))?
        .last()
        .ok_or_else(|| format_err!("Could not get last segment from url path"))?;
    let mut out = File::create(filename)?;
    io::copy(&mut resp, &mut out)?;

    Ok(())
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
