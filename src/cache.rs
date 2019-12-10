use std::{
    fs::{create_dir_all, read_to_string, File},
    io::{BufWriter, Write},
    path::PathBuf,
};

use anyhow::Result;
use chrono::{DateTime, Utc};
use regex::Regex;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_json;
use thiserror::Error;
use url::Url;

use crate::{
    constants::{AVAILABLE_TOOLCHAIN_CACHE, PYTHON_BASE_URL},
    download::download_to_string,
    utils::directory::{PycorsPaths, PycorsPathsFromEnv},
};

// FIXME: Pre-releases are available inside 'https://www.python.org/ftp/python/MAJOR.MINOR.PATCH'
//          This means that seeing 'MAJOR.MINOR.PATCH' in the index.html does not mean a
//          release is available; a pre-release might have created the directory.
// FIXME: Cache is re-created from scratch every time it is created. Save it to disk instead.

// FIXME: Use https://www.python.org/downloads/source/ instead to get all links of releases!

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("No compatible version found")]
    NoCompatibleVersionFound,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct AvailableToolchain {
    pub version: Version,
    pub base_url: Url,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AvailableToolchainsCache {
    last_updated: DateTime<Utc>,
    available: Vec<AvailableToolchain>,
}

fn cache_file() -> PathBuf {
    PycorsPathsFromEnv::new()
        .cache()
        .join(AVAILABLE_TOOLCHAIN_CACHE)
}

impl AvailableToolchainsCache {
    pub fn new() -> Result<AvailableToolchainsCache> {
        log::debug!("Initializing cache...");

        let cache_dir = PycorsPathsFromEnv::new().cache();
        if !cache_dir.exists() {
            create_dir_all(&cache_dir)?
        }

        let cache_file = cache_file();
        let cache: AvailableToolchainsCache = if cache_file.exists() {
            let cache_json = read_to_string(&cache_file)?;
            let mut cache: AvailableToolchainsCache = serde_json::from_str(&cache_json)?;
            let cache_age = Utc::now() - cache.last_updated;
            let cache_age_days = cache_age.num_days();
            if cache_age_days > 10 {
                log::info!(
                    "Cache is older than 10 days (age: {} days). Updating...",
                    cache_age_days
                );
                cache.update()?;
            } else {
                log::info!("Using cache ({} days old)", cache_age_days);
            }
            cache
        } else {
            AvailableToolchainsCache::create()?
        };

        Ok(cache)
    }

    fn create() -> Result<AvailableToolchainsCache> {
        let mut cache = AvailableToolchainsCache {
            last_updated: Utc::now(),
            available: Vec::new(),
        };
        cache.update()?;
        Ok(cache)
    }

    pub fn update(&mut self) -> Result<()> {
        self.last_updated = Utc::now();
        let rt = tokio::runtime::Runtime::new()?;
        let index_html: String = rt.block_on(download_to_string(PYTHON_BASE_URL))?;

        self.available = parse_index_html(&index_html)?;

        let cache_json = serde_json::to_string(&self)?;
        let cache_file = cache_file();
        let mut output = BufWriter::new(File::create(&cache_file)?);
        output.write_all(cache_json.as_bytes())?;

        Ok(())
    }

    pub fn query(&self, version_req: &VersionReq) -> Result<&AvailableToolchain> {
        // Find all compatible versions from the cached list
        let compatible_toolchains: Vec<&AvailableToolchain> = self
            .available
            .iter()
            .filter(|available| version_req.matches(&available.version))
            .collect();

        log::debug!("Compatible versions found: {:?}", compatible_toolchains);

        compatible_toolchains
            .get(0)
            .copied()
            .ok_or_else(|| CacheError::NoCompatibleVersionFound.into())
    }
}

fn parse_index_html(index_html: &str) -> Result<Vec<AvailableToolchain>> {
    let re = Regex::new(r#"(?x)<a \s+ href="(?P<version>\d+[\d\.]+)/">"#)?;

    let base_url =
        Url::parse(PYTHON_BASE_URL).expect("Constant 'PYTHON_BASE_URL' should be parsable");

    let mut toolchains: Vec<AvailableToolchain> = re
        .captures_iter(index_html)
        .filter_map(|caps| {
            let v = &caps["version"];
            let url = base_url.join(&v);
            match url {
                Ok(url) => Some((v.to_string(), url)),
                Err(e) => {
                    log::error!(
                        "Failed to construct a url from version ({:?}), skipping: {:?}",
                        v,
                        e
                    );
                    None
                }
            }
        })
        .filter_map(|(v, url)| {
            // Add a `.0` for versions missing a patch number (f.e. `2.7`)
            let dots = v.chars().filter(|c| *c == '.').count();
            let v = if dots == 1 { format!("{}.0", v) } else { v };
            match Version::parse(&v) {
                Ok(version) => Some(AvailableToolchain {
                    version,
                    base_url: url,
                }),
                Err(e) => {
                    log::error!("Failed to parse version ({:?}), skipping: {:?}", v, e);
                    None
                }
            }
        })
        .collect();

    // Sort the versions vector (in reverse order)
    toolchains.sort_unstable_by(|a, b| b.version.cmp(&a.version));
    Ok(toolchains)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_html() {
        let index_html = include_str!("../tests/fixtures/index.html");

        let parsed: Vec<AvailableToolchain> = parse_index_html(index_html).unwrap();

        let url =
            Url::parse(PYTHON_BASE_URL).expect("Constant 'PYTHON_BASE_URL' should be parsable");

        #[rustfmt::skip]
        let expected: Vec<AvailableToolchain> = vec![
            AvailableToolchain{version: "3.8.0".parse().unwrap(), base_url: url.join("3.8.0").unwrap()},
            AvailableToolchain{version: "3.7.5".parse().unwrap(), base_url: url.join("3.7.5").unwrap()},
            AvailableToolchain{version: "3.7.4".parse().unwrap(), base_url: url.join("3.7.4").unwrap()},
            AvailableToolchain{version: "3.7.3".parse().unwrap(), base_url: url.join("3.7.3").unwrap()},
            AvailableToolchain{version: "3.7.2".parse().unwrap(), base_url: url.join("3.7.2").unwrap()},
            AvailableToolchain{version: "3.7.1".parse().unwrap(), base_url: url.join("3.7.1").unwrap()},
            AvailableToolchain{version: "3.7.0".parse().unwrap(), base_url: url.join("3.7.0").unwrap()},
            AvailableToolchain{version: "3.6.9".parse().unwrap(), base_url: url.join("3.6.9").unwrap()},
            AvailableToolchain{version: "3.6.8".parse().unwrap(), base_url: url.join("3.6.8").unwrap()},
            AvailableToolchain{version: "3.6.7".parse().unwrap(), base_url: url.join("3.6.7").unwrap()},
            AvailableToolchain{version: "3.6.6".parse().unwrap(), base_url: url.join("3.6.6").unwrap()},
            AvailableToolchain{version: "3.6.5".parse().unwrap(), base_url: url.join("3.6.5").unwrap()},
            AvailableToolchain{version: "3.6.4".parse().unwrap(), base_url: url.join("3.6.4").unwrap()},
            AvailableToolchain{version: "3.6.3".parse().unwrap(), base_url: url.join("3.6.3").unwrap()},
            AvailableToolchain{version: "3.6.2".parse().unwrap(), base_url: url.join("3.6.2").unwrap()},
            AvailableToolchain{version: "3.6.1".parse().unwrap(), base_url: url.join("3.6.1").unwrap()},
            AvailableToolchain{version: "3.6.0".parse().unwrap(), base_url: url.join("3.6.0").unwrap()},
            AvailableToolchain{version: "3.5.9".parse().unwrap(), base_url: url.join("3.5.9").unwrap()},
            AvailableToolchain{version: "3.5.8".parse().unwrap(), base_url: url.join("3.5.8").unwrap()},
            AvailableToolchain{version: "3.5.7".parse().unwrap(), base_url: url.join("3.5.7").unwrap()},
            AvailableToolchain{version: "3.5.6".parse().unwrap(), base_url: url.join("3.5.6").unwrap()},
            AvailableToolchain{version: "3.5.5".parse().unwrap(), base_url: url.join("3.5.5").unwrap()},
            AvailableToolchain{version: "3.5.4".parse().unwrap(), base_url: url.join("3.5.4").unwrap()},
            AvailableToolchain{version: "3.5.3".parse().unwrap(), base_url: url.join("3.5.3").unwrap()},
            AvailableToolchain{version: "3.5.2".parse().unwrap(), base_url: url.join("3.5.2").unwrap()},
            AvailableToolchain{version: "3.5.1".parse().unwrap(), base_url: url.join("3.5.1").unwrap()},
            AvailableToolchain{version: "3.5.0".parse().unwrap(), base_url: url.join("3.5.0").unwrap()},
            AvailableToolchain{version: "3.4.10".parse().unwrap(), base_url: url.join("3.4.10").unwrap()},
            AvailableToolchain{version: "3.4.9".parse().unwrap(), base_url: url.join("3.4.9").unwrap()},
            AvailableToolchain{version: "3.4.8".parse().unwrap(), base_url: url.join("3.4.8").unwrap()},
            AvailableToolchain{version: "3.4.7".parse().unwrap(), base_url: url.join("3.4.7").unwrap()},
            AvailableToolchain{version: "3.4.6".parse().unwrap(), base_url: url.join("3.4.6").unwrap()},
            AvailableToolchain{version: "3.4.5".parse().unwrap(), base_url: url.join("3.4.5").unwrap()},
            AvailableToolchain{version: "3.4.4".parse().unwrap(), base_url: url.join("3.4.4").unwrap()},
            AvailableToolchain{version: "3.4.3".parse().unwrap(), base_url: url.join("3.4.3").unwrap()},
            AvailableToolchain{version: "3.4.2".parse().unwrap(), base_url: url.join("3.4.2").unwrap()},
            AvailableToolchain{version: "3.4.1".parse().unwrap(), base_url: url.join("3.4.1").unwrap()},
            AvailableToolchain{version: "3.4.0".parse().unwrap(), base_url: url.join("3.4.0").unwrap()},
            AvailableToolchain{version: "3.3.7".parse().unwrap(), base_url: url.join("3.3.7").unwrap()},
            AvailableToolchain{version: "3.3.6".parse().unwrap(), base_url: url.join("3.3.6").unwrap()},
            AvailableToolchain{version: "3.3.5".parse().unwrap(), base_url: url.join("3.3.5").unwrap()},
            AvailableToolchain{version: "3.3.4".parse().unwrap(), base_url: url.join("3.3.4").unwrap()},
            AvailableToolchain{version: "3.3.3".parse().unwrap(), base_url: url.join("3.3.3").unwrap()},
            AvailableToolchain{version: "3.3.2".parse().unwrap(), base_url: url.join("3.3.2").unwrap()},
            AvailableToolchain{version: "3.3.1".parse().unwrap(), base_url: url.join("3.3.1").unwrap()},
            AvailableToolchain{version: "3.3.0".parse().unwrap(), base_url: url.join("3.3.0").unwrap()},
            AvailableToolchain{version: "3.2.6".parse().unwrap(), base_url: url.join("3.2.6").unwrap()},
            AvailableToolchain{version: "3.2.5".parse().unwrap(), base_url: url.join("3.2.5").unwrap()},
            AvailableToolchain{version: "3.2.4".parse().unwrap(), base_url: url.join("3.2.4").unwrap()},
            AvailableToolchain{version: "3.2.3".parse().unwrap(), base_url: url.join("3.2.3").unwrap()},
            AvailableToolchain{version: "3.2.2".parse().unwrap(), base_url: url.join("3.2.2").unwrap()},
            AvailableToolchain{version: "3.2.1".parse().unwrap(), base_url: url.join("3.2.1").unwrap()},
            AvailableToolchain{version: "3.2.0".parse().unwrap(), base_url: url.join("3.2").unwrap()},
            AvailableToolchain{version: "3.1.5".parse().unwrap(), base_url: url.join("3.1.5").unwrap()},
            AvailableToolchain{version: "3.1.4".parse().unwrap(), base_url: url.join("3.1.4").unwrap()},
            AvailableToolchain{version: "3.1.3".parse().unwrap(), base_url: url.join("3.1.3").unwrap()},
            AvailableToolchain{version: "3.1.2".parse().unwrap(), base_url: url.join("3.1.2").unwrap()},
            AvailableToolchain{version: "3.1.1".parse().unwrap(), base_url: url.join("3.1.1").unwrap()},
            AvailableToolchain{version: "3.1.0".parse().unwrap(), base_url: url.join("3.1").unwrap()},
            AvailableToolchain{version: "3.0.1".parse().unwrap(), base_url: url.join("3.0.1").unwrap()},
            AvailableToolchain{version: "3.0.0".parse().unwrap(), base_url: url.join("3.0").unwrap()},
            AvailableToolchain{version: "2.7.17".parse().unwrap(), base_url: url.join("2.7.17").unwrap()},
            AvailableToolchain{version: "2.7.16".parse().unwrap(), base_url: url.join("2.7.16").unwrap()},
            AvailableToolchain{version: "2.7.15".parse().unwrap(), base_url: url.join("2.7.15").unwrap()},
            AvailableToolchain{version: "2.7.14".parse().unwrap(), base_url: url.join("2.7.14").unwrap()},
            AvailableToolchain{version: "2.7.13".parse().unwrap(), base_url: url.join("2.7.13").unwrap()},
            AvailableToolchain{version: "2.7.12".parse().unwrap(), base_url: url.join("2.7.12").unwrap()},
            AvailableToolchain{version: "2.7.11".parse().unwrap(), base_url: url.join("2.7.11").unwrap()},
            AvailableToolchain{version: "2.7.10".parse().unwrap(), base_url: url.join("2.7.10").unwrap()},
            AvailableToolchain{version: "2.7.9".parse().unwrap(), base_url: url.join("2.7.9").unwrap()},
            AvailableToolchain{version: "2.7.8".parse().unwrap(), base_url: url.join("2.7.8").unwrap()},
            AvailableToolchain{version: "2.7.7".parse().unwrap(), base_url: url.join("2.7.7").unwrap()},
            AvailableToolchain{version: "2.7.6".parse().unwrap(), base_url: url.join("2.7.6").unwrap()},
            AvailableToolchain{version: "2.7.5".parse().unwrap(), base_url: url.join("2.7.5").unwrap()},
            AvailableToolchain{version: "2.7.4".parse().unwrap(), base_url: url.join("2.7.4").unwrap()},
            AvailableToolchain{version: "2.7.3".parse().unwrap(), base_url: url.join("2.7.3").unwrap()},
            AvailableToolchain{version: "2.7.2".parse().unwrap(), base_url: url.join("2.7.2").unwrap()},
            AvailableToolchain{version: "2.7.1".parse().unwrap(), base_url: url.join("2.7.1").unwrap()},
            AvailableToolchain{version: "2.7.0".parse().unwrap(), base_url: url.join("2.7").unwrap()},
            AvailableToolchain{version: "2.6.9".parse().unwrap(), base_url: url.join("2.6.9").unwrap()},
            AvailableToolchain{version: "2.6.8".parse().unwrap(), base_url: url.join("2.6.8").unwrap()},
            AvailableToolchain{version: "2.6.7".parse().unwrap(), base_url: url.join("2.6.7").unwrap()},
            AvailableToolchain{version: "2.6.6".parse().unwrap(), base_url: url.join("2.6.6").unwrap()},
            AvailableToolchain{version: "2.6.5".parse().unwrap(), base_url: url.join("2.6.5").unwrap()},
            AvailableToolchain{version: "2.6.4".parse().unwrap(), base_url: url.join("2.6.4").unwrap()},
            AvailableToolchain{version: "2.6.3".parse().unwrap(), base_url: url.join("2.6.3").unwrap()},
            AvailableToolchain{version: "2.6.2".parse().unwrap(), base_url: url.join("2.6.2").unwrap()},
            AvailableToolchain{version: "2.6.1".parse().unwrap(), base_url: url.join("2.6.1").unwrap()},
            AvailableToolchain{version: "2.6.0".parse().unwrap(), base_url: url.join("2.6").unwrap()},
            AvailableToolchain{version: "2.5.6".parse().unwrap(), base_url: url.join("2.5.6").unwrap()},
            AvailableToolchain{version: "2.5.5".parse().unwrap(), base_url: url.join("2.5.5").unwrap()},
            AvailableToolchain{version: "2.5.4".parse().unwrap(), base_url: url.join("2.5.4").unwrap()},
            AvailableToolchain{version: "2.5.3".parse().unwrap(), base_url: url.join("2.5.3").unwrap()},
            AvailableToolchain{version: "2.5.2".parse().unwrap(), base_url: url.join("2.5.2").unwrap()},
            AvailableToolchain{version: "2.5.1".parse().unwrap(), base_url: url.join("2.5.1").unwrap()},
            AvailableToolchain{version: "2.5.0".parse().unwrap(), base_url: url.join("2.5").unwrap()},
            AvailableToolchain{version: "2.4.6".parse().unwrap(), base_url: url.join("2.4.6").unwrap()},
            AvailableToolchain{version: "2.4.5".parse().unwrap(), base_url: url.join("2.4.5").unwrap()},
            AvailableToolchain{version: "2.4.4".parse().unwrap(), base_url: url.join("2.4.4").unwrap()},
            AvailableToolchain{version: "2.4.3".parse().unwrap(), base_url: url.join("2.4.3").unwrap()},
            AvailableToolchain{version: "2.4.2".parse().unwrap(), base_url: url.join("2.4.2").unwrap()},
            AvailableToolchain{version: "2.4.1".parse().unwrap(), base_url: url.join("2.4.1").unwrap()},
            AvailableToolchain{version: "2.4.0".parse().unwrap(), base_url: url.join("2.4").unwrap()},
            AvailableToolchain{version: "2.3.7".parse().unwrap(), base_url: url.join("2.3.7").unwrap()},
            AvailableToolchain{version: "2.3.6".parse().unwrap(), base_url: url.join("2.3.6").unwrap()},
            AvailableToolchain{version: "2.3.5".parse().unwrap(), base_url: url.join("2.3.5").unwrap()},
            AvailableToolchain{version: "2.3.4".parse().unwrap(), base_url: url.join("2.3.4").unwrap()},
            AvailableToolchain{version: "2.3.3".parse().unwrap(), base_url: url.join("2.3.3").unwrap()},
            AvailableToolchain{version: "2.3.2".parse().unwrap(), base_url: url.join("2.3.2").unwrap()},
            AvailableToolchain{version: "2.3.1".parse().unwrap(), base_url: url.join("2.3.1").unwrap()},
            AvailableToolchain{version: "2.3.0".parse().unwrap(), base_url: url.join("2.3").unwrap()},
            AvailableToolchain{version: "2.2.3".parse().unwrap(), base_url: url.join("2.2.3").unwrap()},
            AvailableToolchain{version: "2.2.2".parse().unwrap(), base_url: url.join("2.2.2").unwrap()},
            AvailableToolchain{version: "2.2.1".parse().unwrap(), base_url: url.join("2.2.1").unwrap()},
            AvailableToolchain{version: "2.2.0".parse().unwrap(), base_url: url.join("2.2").unwrap()},
            AvailableToolchain{version: "2.1.3".parse().unwrap(), base_url: url.join("2.1.3").unwrap()},
            AvailableToolchain{version: "2.1.2".parse().unwrap(), base_url: url.join("2.1.2").unwrap()},
            AvailableToolchain{version: "2.1.1".parse().unwrap(), base_url: url.join("2.1.1").unwrap()},
            AvailableToolchain{version: "2.1.0".parse().unwrap(), base_url: url.join("2.1").unwrap()},
            AvailableToolchain{version: "2.0.1".parse().unwrap(), base_url: url.join("2.0.1").unwrap()},
            AvailableToolchain{version: "2.0.0".parse().unwrap(), base_url: url.join("2.0").unwrap()},
        ];
        assert_eq!(parsed, expected);
    }
}
