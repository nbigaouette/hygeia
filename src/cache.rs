use chrono::{DateTime, Utc};
use regex::Regex;
use semver::{SemVerError, Version, VersionReq};
use url::Url;

use crate::{constants::PYTHON_BASE_URL, Result};

// FIXME: Pre-releases are available inside 'https://www.python.org/ftp/python/MAJOR.MINOR.PATCH'
//          This means that seeing 'MAJOR.MINOR.PATCH' in the index.html does not mean a
//          release is available; a pre-release might have created the directory.

#[derive(Debug, failure::Fail)]
pub enum CacheError {
    #[fail(display = "Failed to parse version: {:?}", _0)]
    SemVerError(#[fail(cause)] SemVerError),
    #[fail(display = "No compatible version found")]
    NoCompatibleVersionFound,
}

#[derive(Debug)]
pub struct AvailableToolchain {
    version: Version,
    url: Url,
}

#[derive(Debug)]
pub struct AvailableToolchainsCache {
    last_updated: DateTime<Utc>,
    available: Vec<AvailableToolchain>,
}

impl AvailableToolchainsCache {
    pub fn new() -> Result<AvailableToolchainsCache> {
        // Load from file, etc.
        log::debug!("Initializing cache...");
        let mut cache = AvailableToolchainsCache {
            last_updated: Utc::now(),
            available: Vec::new(),
        };
        cache.update()?;
        Ok(cache)
    }

    pub fn update(&mut self) -> Result<()> {
        let index_html = reqwest::get(PYTHON_BASE_URL)?.text()?;

        self.available = parse_index_html(&index_html)?;

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
            .last()
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
                Ok(version) => Some(AvailableToolchain { version, url }),
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
