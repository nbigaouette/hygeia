use std::{
    fs::{create_dir_all, read_to_string, File},
    io::{BufWriter, Write},
};

use anyhow::Result;
use chrono::{DateTime, Utc};
use regex::{Regex, RegexBuilder};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_json;
use thiserror::Error;
use url::Url;

use crate::{
    constants::PYTHON_SOURCE_INDEX_URL,
    download::{download_to_string, HyperDownloader},
    utils::directory::{PycorsHomeProviderTrait, PycorsPathsProvider},
};

#[cfg(test)]
mod tests;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("No compatible version found")]
    NoCompatibleVersionFound,
}

#[cfg_attr(test, mockall::automock)]
pub trait ToolchainsCacheFetch {
    fn get(&self) -> Result<String>;
}

pub struct ToolchainsCacheFetchOnline;

impl ToolchainsCacheFetch for ToolchainsCacheFetchOnline {
    fn get(&self) -> Result<String> {
        let mut downloader = HyperDownloader::new(PYTHON_SOURCE_INDEX_URL)?;
        // HTML file is too small to bother with a prog
        let with_progress_bar = false;
        let mut rt = tokio::runtime::Runtime::new()?;
        let index_html: String =
            rt.block_on(download_to_string(&mut downloader, with_progress_bar))?;

        Ok(index_html)
    }
}

pub type AvailableToolchain = AvailableToolchainFromSource;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct AvailableToolchainFromSource {
    pub version: Version,
    pub base_url: Url,
    pub source_tar_gz: String,
}

impl AvailableToolchainFromSource {
    #[cfg_attr(windows, allow(dead_code))]
    pub fn source_url(&self) -> Url {
        let mut new_url = self.base_url.clone();
        new_url
            .path_segments_mut()
            .unwrap()
            .extend(&[&self.source_tar_gz]);
        new_url
    }

    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn windows_pre_built_url(&self) -> Url {
        let mut new_url = self.base_url.clone();
        new_url
            .path_segments_mut()
            .unwrap()
            .extend(&[&format!("python-{}-embed-amd64.zip", self.version)]);
        new_url
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AvailableToolchainsCache {
    last_updated: DateTime<Utc>,
    available: Vec<AvailableToolchain>,
}

impl AvailableToolchainsCache {
    pub fn new<P, D>(
        paths_provider: &PycorsPathsProvider<P>,
        downloader: &D,
    ) -> Result<AvailableToolchainsCache>
    where
        P: PycorsHomeProviderTrait,
        D: ToolchainsCacheFetch,
    {
        log::debug!("Initializing cache...");

        let cache_dir = paths_provider.cache();
        if !cache_dir.exists() {
            create_dir_all(&cache_dir)?
        }

        let cache_file = paths_provider.available_toolchains_cache_file();
        let cache: AvailableToolchainsCache = if cache_file.exists() {
            let cache_json = read_to_string(&cache_file)?;
            match serde_json::from_str::<AvailableToolchainsCache>(&cache_json) {
                Ok(mut cache) => {
                    let cache_age = Utc::now() - cache.last_updated;
                    let cache_age_days = cache_age.num_days();
                    if cache_age_days > 10 {
                        log::info!(
                            "Cache is older than 10 days (age: {} days). Updating...",
                            cache_age_days
                        );
                        cache.update(paths_provider, downloader)?;
                    } else {
                        log::info!("Using cache ({} days old)", cache_age_days);
                    }
                    cache
                }
                Err(e) => {
                    log::error!(
                        "Corrupted cache on disk ({:?}), recreating: {:?}",
                        cache_file,
                        e
                    );
                    AvailableToolchainsCache::create(paths_provider, downloader).map(
                        |available_toolchains| {
                            log::error!("Cache successfully recreated, moving on.");
                            available_toolchains
                        },
                    )?
                }
            }
        } else {
            AvailableToolchainsCache::create(paths_provider, downloader)?
        };

        Ok(cache)
    }

    fn create<P, D>(
        paths_provider: &PycorsPathsProvider<P>,
        downloader: &D,
    ) -> Result<AvailableToolchainsCache>
    where
        P: PycorsHomeProviderTrait,
        D: ToolchainsCacheFetch,
    {
        let mut cache = AvailableToolchainsCache {
            last_updated: Utc::now(),
            available: Vec::new(),
        };
        cache.update(paths_provider, downloader)?;
        Ok(cache)
    }

    pub fn update<P, D>(
        &mut self,
        paths_provider: &PycorsPathsProvider<P>,
        downloader: &D,
    ) -> Result<()>
    where
        P: PycorsHomeProviderTrait,
        D: ToolchainsCacheFetch,
    {
        self.last_updated = Utc::now();
        let index_html: String = downloader.get()?;

        self.available = parse_source_index_html(&index_html)?;

        let cache_json = serde_json::to_string(&self)?;
        let cache_file = paths_provider.available_toolchains_cache_file();
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

fn parse_source_index_html(index_html: &str) -> Result<Vec<AvailableToolchain>> {
    let re: Regex = RegexBuilder::new(
        r#"<a href="/downloads/release/python-[^/]*/">Python (?P<version>[2-3].[0-9]+.[0-9]+[^ ]*)? - .*<a href="(?P<url>[^"]*)">Gzipped source tar"#
    )
    .dot_matches_new_line(true)  // (s)
    .swap_greed(true)  // (U)
        .build()?;

    let mut toolchains: Vec<AvailableToolchain> = re
        .captures_iter(index_html)
        .filter_map(|caps| match (caps.name("version"), caps.name("url")) {
            (Some(version), Some(url)) => Some((version.as_str(), url.as_str())),
            (Some(version), None) => {
                log::error!("Failed to extract url for version {}", version.as_str());
                None
            }
            (None, Some(url)) => {
                log::error!("Failed to extract version for url {}", url.as_str());
                None
            }
            (None, None) => None,
        })
        .map(|(version, url)| {
            let version = Version::parse(
                &version
                    .replace("rc", "-rc") // release candidates
                    .replace("a", "-a") // alpha
                    .replace("b", "-b"), // beta
            )
            .unwrap();
            let mut url = Url::parse(url).unwrap();
            let source_tar_gz = url.path_segments().unwrap().last().unwrap().to_string();
            url.path_segments_mut().unwrap().pop();
            url.set_scheme("https").unwrap(); // 3.3.4, 3.3.5 has "http" instead of "https"
            AvailableToolchain {
                version,
                base_url: url,
                source_tar_gz,
            }
        })
        .collect();

    // Sort the versions vector (in reverse order)
    toolchains.sort_unstable_by(|a, b| b.version.cmp(&a.version));
    Ok(toolchains)
}
