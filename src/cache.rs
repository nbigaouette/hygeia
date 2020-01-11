use std::{
    fs::{create_dir_all, read_to_string, File},
    io::{BufWriter, Write},
};

use anyhow::Result;
use chrono::{DateTime, Utc};
use select::{
    document::Document,
    predicate::{Class, Name, Predicate},
};
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

trait AvailableToolchainTrait {
    fn new(version: Version, base_url: Url, filename: String) -> Self;
    fn version(&self) -> &Version;
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct AvailableToolchainFromSource {
    pub version: Version,
    pub base_url: Url,
    pub source_tar_gz: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct AvailableToolchainWindowsPreBuilt {
    pub version: Version,
    pub base_url: Url,
    pub win_pre_built: String,
}

impl AvailableToolchainTrait for AvailableToolchainFromSource {
    fn new(version: Version, base_url: Url, filename: String) -> Self {
        AvailableToolchainFromSource {
            version,
            base_url,
            source_tar_gz: filename,
        }
    }
    fn version(&self) -> &Version {
        &self.version
    }
}

impl AvailableToolchainTrait for AvailableToolchainWindowsPreBuilt {
    fn new(version: Version, base_url: Url, filename: String) -> Self {
        AvailableToolchainWindowsPreBuilt {
            version,
            base_url,
            win_pre_built: filename,
        }
    }
    fn version(&self) -> &Version {
        &self.version
    }
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

impl AvailableToolchainWindowsPreBuilt {
    #[cfg_attr(not(windows), allow(dead_code))]
    pub fn windows_pre_built_url(&self) -> Url {
        let mut new_url = self.base_url.clone();
        new_url
            .path_segments_mut()
            .unwrap()
            .extend(&[&self.win_pre_built]);
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

fn parse_index_html<A>(index_html: &str, end_of_file: &str) -> Result<Vec<A>>
where
    A: AvailableToolchainTrait,
{
    let mut toolchains: Vec<A> = Vec::new();

    let document = Document::from(index_html);
    let node = document.find(Class("text")).next().unwrap();
    // Iterate over columns (Release, Pre-Release)
    for column in node.find(Class("column")) {
        for version_found in column.find(Name("ul").descendant(Name("li").descendant(Name("ul")))) {
            let version_string = version_found
                .parent()
                .unwrap()
                .text()
                .trim()
                .split(' ')
                .nth(1)
                .unwrap()
                .to_string();

            for links in version_found.find(Name("li").descendant(Name("a"))) {
                if let Some(url) = links.attr("href") {
                    if url.ends_with(end_of_file) {
                        let version = Version::parse(
                            &version_string
                                .replace("rc", "-rc") // release candidates
                                .replace("a", "-a") // alpha
                                .replace("b", "-b"), // beta
                        )
                        .unwrap();
                        let mut url = Url::parse(url).unwrap();
                        let filename = url.path_segments().unwrap().last().unwrap().to_string();
                        url.path_segments_mut().unwrap().pop();
                        url.set_scheme("https").unwrap(); // 3.3.4, 3.3.5 has "http" instead of "https"
                        toolchains.push(A::new(version, url, filename))
                    }
                }
            }
        }
    }

    // Sort the versions vector (in reverse order)
    toolchains.sort_unstable_by(|a, b| b.version().cmp(&a.version()));
    Ok(toolchains)
}

fn parse_source_index_html(index_html: &str) -> Result<Vec<AvailableToolchainFromSource>> {
    parse_index_html::<AvailableToolchainFromSource>(index_html, ".tgz")
}

#[allow(dead_code)]
fn parse_win_pre_built_index_html(
    index_html: &str,
) -> Result<Vec<AvailableToolchainWindowsPreBuilt>> {
    parse_index_html::<AvailableToolchainWindowsPreBuilt>(index_html, "-embed-amd64.zip")
}
