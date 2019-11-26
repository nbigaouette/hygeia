use std::{
    fs::{read_to_string, File},
    io::{BufWriter, Write},
    path::PathBuf,
};

use chrono::{DateTime, Utc};
use regex::Regex;
use semver::{SemVerError, Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_json;
use url::Url;

use crate::{
    constants::{AVAILABLE_TOOLCHAIN_CACHE, PYTHON_BASE_URL},
    utils, Result,
};

// FIXME: Pre-releases are available inside 'https://www.python.org/ftp/python/MAJOR.MINOR.PATCH'
//          This means that seeing 'MAJOR.MINOR.PATCH' in the index.html does not mean a
//          release is available; a pre-release might have created the directory.
// FIXME: Cache is re-created from scratch every time it is created. Save it to disk instead.

#[derive(Debug, failure::Fail)]
pub enum CacheError {
    #[fail(display = "Failed to parse version: {:?}", _0)]
    SemVerError(#[fail(cause)] SemVerError),
    #[fail(display = "No compatible version found")]
    NoCompatibleVersionFound,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AvailableToolchain {
    pub version: Version,
    pub base_url: Url,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AvailableToolchainsCache {
    last_updated: DateTime<Utc>,
    available: Vec<AvailableToolchain>,
}

fn cache_file() -> Result<PathBuf> {
    Ok(utils::directory::cache()?.join(AVAILABLE_TOOLCHAIN_CACHE))
}

impl AvailableToolchainsCache {
    pub fn new() -> Result<AvailableToolchainsCache> {
        log::debug!("Initializing cache...");

        let cache_file = cache_file()?;
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
        let index_html = reqwest::get(PYTHON_BASE_URL)?.text()?;

        self.last_updated = Utc::now();

        self.available = parse_index_html(&index_html)?;

        let cache_json = serde_json::to_string(&self)?;
        let cache_file = cache_file()?;
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
            .map(|a: &&AvailableToolchain| *a) // Deref once
            .ok_or(CacheError::NoCompatibleVersionFound.into())
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
