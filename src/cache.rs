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
    constants::{PYTHON_SOURCE_INDEX_URL, PYTHON_WINDOWS_INDEX_URL},
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
    fn get_source(&self) -> Result<String>;
    fn get_win_prebuilt(&self) -> Result<String>;
}

pub struct ToolchainsCacheFetchOnline;

impl ToolchainsCacheFetch for ToolchainsCacheFetchOnline {
    fn get_source(&self) -> Result<String> {
        let mut downloader = HyperDownloader::new(PYTHON_SOURCE_INDEX_URL)?;
        // HTML file is too small to bother with a progress bar
        let with_progress_bar = false;
        let mut rt = tokio::runtime::Runtime::new()?;
        let index_html: String =
            rt.block_on(download_to_string(&mut downloader, with_progress_bar))?;

        Ok(index_html)
    }
    fn get_win_prebuilt(&self) -> Result<String> {
        let mut downloader = HyperDownloader::new(PYTHON_WINDOWS_INDEX_URL)?;
        // HTML file is too small to bother with a progress bar
        let with_progress_bar = false;
        let mut rt = tokio::runtime::Runtime::new()?;
        let index_html: String =
            rt.block_on(download_to_string(&mut downloader, with_progress_bar))?;

        Ok(index_html)
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct AvailableToolchain {
    pub version: Version,
    pub base_url: Url,
    pub source_tar_gz: String,
    pub win_pre_built: Option<String>,
}

trait AvailableToolchainTrait {
    fn new(version: Version, base_url: Url, filename: String) -> Self;
    fn version(&self) -> &Version;
}

#[derive(Debug, PartialEq)]
pub struct AvailableToolchainFromSource {
    pub version: Version,
    pub base_url: Url,
    pub source_tar_gz: String,
}

#[derive(Debug, PartialEq)]
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

impl AvailableToolchain {
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
    pub fn windows_pre_built_url(&self) -> Option<Url> {
        self.win_pre_built.as_ref().map(|win_pre_built| {
            let mut new_url = self.base_url.clone();
            new_url
                .path_segments_mut()
                .unwrap()
                .extend(&[win_pre_built]);
            new_url
        })
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
        let index_html_source: String = downloader.get_source()?;
        let index_html_win_prebuilt: String = downloader.get_win_prebuilt()?;

        let available_toolchains_source = parse_source_index_html(&index_html_source)?;
        let available_toolchains_win_prebuilt =
            parse_win_pre_built_index_html(&index_html_win_prebuilt)?;
        self.available = merge_available_toolchains(
            available_toolchains_source,
            available_toolchains_win_prebuilt,
        );

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

fn merge_available_toolchains(
    available_toolchains_source: Vec<AvailableToolchainFromSource>,
    available_toolchains_win_prebuilt: Vec<AvailableToolchainWindowsPreBuilt>,
) -> Vec<AvailableToolchain> {
    let mut result = Vec::with_capacity(
        available_toolchains_source.len() + available_toolchains_win_prebuilt.len(),
    );

    let mut source_iter = available_toolchains_source.into_iter();
    let mut pre_built_iter = available_toolchains_win_prebuilt.into_iter();

    let mut next_source: Option<AvailableToolchainFromSource> = source_iter.next();
    let mut next_pre_built: Option<AvailableToolchainWindowsPreBuilt> = pre_built_iter.next();
    loop {
        match (&mut next_source, &mut next_pre_built) {
            (None, None) => break,
            (Some(source), None) => {
                result.push(AvailableToolchain {
                    version: source.version.clone(),
                    base_url: source.base_url.clone(),
                    source_tar_gz: source.source_tar_gz.clone(),
                    win_pre_built: None,
                });
                next_source = source_iter.next();
            }
            (None, Some(_pre_built)) => {
                unreachable!(
                    "We should not find a pre-built package without corresponding source archive."
                );
            }
            (Some(source), Some(pre_built)) => match source.version.cmp(&pre_built.version) {
                std::cmp::Ordering::Greater => {
                    result.push(AvailableToolchain {
                        version: source.version.clone(),
                        base_url: source.base_url.clone(),
                        source_tar_gz: source.source_tar_gz.clone(),
                        win_pre_built: None,
                    });
                    next_source = source_iter.next();
                }
                std::cmp::Ordering::Less => {
                    unreachable!(
                        "We should not find a pre-built package without corresponding source archive."
                    );
                }
                std::cmp::Ordering::Equal => {
                    result.push(AvailableToolchain {
                        version: pre_built.version.clone(),
                        base_url: pre_built.base_url.clone(),
                        source_tar_gz: source.source_tar_gz.clone(),
                        win_pre_built: Some(pre_built.win_pre_built.clone()),
                    });
                    next_source = source_iter.next();
                    next_pre_built = pre_built_iter.next();
                }
            },
        }
    }

    result
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
