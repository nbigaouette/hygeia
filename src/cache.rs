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
    download::download_to_string,
    utils::directory::{PycorsHomeProviderTrait, PycorsPathsProvider},
};

// FIXME: Pre-releases are available inside 'https://www.python.org/ftp/python/MAJOR.MINOR.PATCH'
//          This means that seeing 'MAJOR.MINOR.PATCH' in the index.html does not mean a
//          release is available; a pre-release might have created the directory.

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
        let mut rt = tokio::runtime::Runtime::new()?;
        let index_html: String = rt.block_on(download_to_string(PYTHON_SOURCE_INDEX_URL))?;
        Ok(index_html)
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct AvailableToolchain {
    pub version: Version,
    pub url: Url,
    pub source_tar_gz: String,
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
                    AvailableToolchainsCache::create(paths_provider, downloader)?
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

        self.available = parse_index_html(&index_html)?;

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

fn parse_index_html(index_html: &str) -> Result<Vec<AvailableToolchain>> {
    let re: Regex = RegexBuilder::new(
        r#"<a href="/downloads/release/python.*">Python (?P<version>[2-3].[0-9]+.[0-9]+)? .*Download <a href="(?P<url>[^<]*)?">Gzipped source tar"#
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
            let version = Version::parse(version).unwrap();
            let mut url = Url::parse(url).unwrap();
            let source_tar_gz = url.path_segments().unwrap().last().unwrap().to_string();
            url.path_segments_mut().unwrap().pop();
            url.set_scheme("https").unwrap(); // 3.3.4, 3.3.5 has "http" instead of "https"
            AvailableToolchain {
                version,
                url,
                source_tar_gz,
            }
        })
        .collect();

    // Sort the versions vector (in reverse order)
    toolchains.sort_unstable_by(|a, b| b.version.cmp(&a.version));
    Ok(toolchains)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use std::{env, fs, path::PathBuf};

    use chrono::Duration;

    use super::*;
    use crate::utils::directory::MockPycorsHomeProviderTrait;

    use mockall::predicate::*;

    const INDEX_HTML: &str = include_str!("../tests/fixtures/index.html");

    fn temp_dir() -> PathBuf {
        env::temp_dir()
            .join(crate::constants::EXECUTABLE_NAME)
            .join("cache")
            .join("tests")
    }

    #[test]
    fn cache_new_empty() {
        let pycors_home = temp_dir().join("cache_from_env");
        let mocked_pycors_home = Some(pycors_home.as_os_str().to_os_string());

        // The test expects an empty directory
        if pycors_home.exists() {
            fs::remove_dir_all(&pycors_home).unwrap();
        }

        let mut mock = MockPycorsHomeProviderTrait::new();
        mock.expect_home_env_variable()
            .times(3)
            .return_const(mocked_pycors_home.clone());

        let paths_provider = PycorsPathsProvider::from(mock);

        let mut mock = MockToolchainsCacheFetch::new();
        mock.expect_get()
            .times(1)
            .returning(|| Ok(INDEX_HTML.to_string()));
        let _cache = AvailableToolchainsCache::new(&paths_provider, &mock).unwrap();
    }

    #[test]
    fn cache_up_to_date() {
        let pycors_home = temp_dir().join("cache_up_to_date");
        let mocked_pycors_home = Some(pycors_home.as_os_str().to_os_string());

        // The test expects an empty directory
        if pycors_home.exists() {
            fs::remove_dir_all(&pycors_home).unwrap();
        }

        let mut mock = MockPycorsHomeProviderTrait::new();
        mock.expect_home_env_variable()
            .times(2 + 1) // +1 since we later get the cache file
            .return_const(mocked_pycors_home.clone());
        let paths_provider = PycorsPathsProvider::from(mock);
        let cache_file = paths_provider.available_toolchains_cache_file();

        // Save a dummy cache
        // NOTE: Since we call a method on 'paths_provider', this will increment the mock count
        let dummy_cache = AvailableToolchainsCache {
            last_updated: Utc::now() - Duration::days(1),
            available: Vec::new(),
        };
        let cache_json = serde_json::to_string(&dummy_cache).unwrap();
        fs::create_dir_all(cache_file.parent().unwrap()).unwrap();
        let mut f = File::create(cache_file).unwrap();
        f.write_all(cache_json.as_bytes()).unwrap();

        // Let's create the cache for real
        let mut mock = MockToolchainsCacheFetch::new();
        mock.expect_get()
            .times(0) // Cache file is up to date, no download required.
            .returning(|| Ok(INDEX_HTML.to_string()));
        let _cache = AvailableToolchainsCache::new(&paths_provider, &mock).unwrap();
    }

    #[test]
    fn cache_corrupted() {
        crate::tests::init_logger();

        let pycors_home = temp_dir().join("cache_corrupted");
        let mocked_pycors_home = Some(pycors_home.as_os_str().to_os_string());

        // The test expects an empty directory
        if pycors_home.exists() {
            fs::remove_dir_all(&pycors_home).unwrap();
        }

        let mut mock = MockPycorsHomeProviderTrait::new();
        mock.expect_home_env_variable()
            .times(3 + 1) // +1 since we later get the cache file
            .return_const(mocked_pycors_home.clone());
        let paths_provider = PycorsPathsProvider::from(mock);
        let cache_file = paths_provider.available_toolchains_cache_file();

        // Save a dummy cache
        // NOTE: Since we call a method on 'paths_provider', this will increment the mock count
        let dummy_cache = AvailableToolchainsCache {
            last_updated: Utc::now() - Duration::days(1),
            available: Vec::new(),
        };
        let cache_json = serde_json::to_string(&dummy_cache).unwrap();
        fs::create_dir_all(cache_file.parent().unwrap()).unwrap();
        let mut f = File::create(cache_file).unwrap();
        let cache_bytes = cache_json.as_bytes();
        // Save a corrupted version of the cache
        f.write_all(&cache_bytes[0..(cache_bytes.len() / 2)])
            .unwrap();

        // Let's create the cache for real
        let mut mock = MockToolchainsCacheFetch::new();
        mock.expect_get()
            .times(1) // Cache file is corrupted, new download required.
            .returning(|| Ok(INDEX_HTML.to_string()));
        let _cache = AvailableToolchainsCache::new(&paths_provider, &mock).unwrap();
    }

    #[test]
    fn cache_outdated() {
        crate::tests::init_logger();

        let pycors_home = temp_dir().join("cache_outdated");
        let mocked_pycors_home = Some(pycors_home.as_os_str().to_os_string());

        // The test expects an empty directory
        if pycors_home.exists() {
            fs::remove_dir_all(&pycors_home).unwrap();
        }

        let mut mock = MockPycorsHomeProviderTrait::new();
        mock.expect_home_env_variable()
            .times(3 + 1) // +1 since we later get the cache file
            .return_const(mocked_pycors_home.clone());
        let paths_provider = PycorsPathsProvider::from(mock);
        let cache_file = paths_provider.available_toolchains_cache_file();

        // Save a dummy cache
        // NOTE: Since we call a method on 'paths_provider', this will increment the mock count
        let dummy_cache = AvailableToolchainsCache {
            last_updated: Utc::now() - Duration::days(11),
            available: Vec::new(),
        };
        let cache_json = serde_json::to_string(&dummy_cache).unwrap();
        fs::create_dir_all(cache_file.parent().unwrap()).unwrap();
        let mut f = File::create(cache_file).unwrap();
        let cache_bytes = cache_json.as_bytes();
        // Save the cache
        f.write_all(&cache_bytes).unwrap();

        let mut mock = MockToolchainsCacheFetch::new();
        mock.expect_get()
            .times(1) // Cache file is outdated, new download required.
            .returning(|| Ok(INDEX_HTML.to_string()));
        let _cache = AvailableToolchainsCache::new(&paths_provider, &mock).unwrap();
    }

    #[test]
    fn parse_html() {
        let parsed: Vec<AvailableToolchain> = parse_index_html(INDEX_HTML).unwrap();
        assert_eq!(parsed.len(), 117);

        // Note: Trailing '/' is required for proper parsing
        pub const PYTHON_BASE_URL: &str = "https://www.python.org/ftp/python/";

        let url =
            Url::parse(PYTHON_BASE_URL).expect("Constant 'PYTHON_BASE_URL' should be parsable");

        #[rustfmt::skip]
        let expected: Vec<AvailableToolchain> = vec![
            AvailableToolchain{version: "3.8.0".parse().unwrap(), url: url.join("3.8.0").unwrap(), source_tar_gz: String::from("Python-3.8.0.tgz")},
            AvailableToolchain{version: "3.7.5".parse().unwrap(), url: url.join("3.7.5").unwrap(), source_tar_gz: String::from("Python-3.7.5.tgz")},
            AvailableToolchain{version: "3.7.4".parse().unwrap(), url: url.join("3.7.4").unwrap(), source_tar_gz: String::from("Python-3.7.4.tgz")},
            AvailableToolchain{version: "3.7.3".parse().unwrap(), url: url.join("3.7.3").unwrap(), source_tar_gz: String::from("Python-3.7.3.tgz")},
            AvailableToolchain{version: "3.7.2".parse().unwrap(), url: url.join("3.7.2").unwrap(), source_tar_gz: String::from("Python-3.7.2.tgz")},
            AvailableToolchain{version: "3.7.1".parse().unwrap(), url: url.join("3.7.1").unwrap(), source_tar_gz: String::from("Python-3.7.1.tgz")},
            AvailableToolchain{version: "3.7.0".parse().unwrap(), url: url.join("3.7.0").unwrap(), source_tar_gz: String::from("Python-3.7.0.tgz")},
            AvailableToolchain{version: "3.6.9".parse().unwrap(), url: url.join("3.6.9").unwrap(), source_tar_gz: String::from("Python-3.6.9.tgz")},
            AvailableToolchain{version: "3.6.8".parse().unwrap(), url: url.join("3.6.8").unwrap(), source_tar_gz: String::from("Python-3.6.8.tgz")},
            AvailableToolchain{version: "3.6.7".parse().unwrap(), url: url.join("3.6.7").unwrap(), source_tar_gz: String::from("Python-3.6.7.tgz")},
            AvailableToolchain{version: "3.6.6".parse().unwrap(), url: url.join("3.6.6").unwrap(), source_tar_gz: String::from("Python-3.6.6.tgz")},
            AvailableToolchain{version: "3.6.5".parse().unwrap(), url: url.join("3.6.5").unwrap(), source_tar_gz: String::from("Python-3.6.5.tgz")},
            AvailableToolchain{version: "3.6.4".parse().unwrap(), url: url.join("3.6.4").unwrap(), source_tar_gz: String::from("Python-3.6.4.tgz")},
            AvailableToolchain{version: "3.6.3".parse().unwrap(), url: url.join("3.6.3").unwrap(), source_tar_gz: String::from("Python-3.6.3.tgz")},
            AvailableToolchain{version: "3.6.2".parse().unwrap(), url: url.join("3.6.2").unwrap(), source_tar_gz: String::from("Python-3.6.2.tgz")},
            AvailableToolchain{version: "3.6.1".parse().unwrap(), url: url.join("3.6.1").unwrap(), source_tar_gz: String::from("Python-3.6.1.tgz")},
            AvailableToolchain{version: "3.6.0".parse().unwrap(), url: url.join("3.6.0").unwrap(), source_tar_gz: String::from("Python-3.6.0.tgz")},
            AvailableToolchain{version: "3.5.9".parse().unwrap(), url: url.join("3.5.9").unwrap(), source_tar_gz: String::from("Python-3.5.9.tgz")},
            AvailableToolchain{version: "3.5.8".parse().unwrap(), url: url.join("3.5.8").unwrap(), source_tar_gz: String::from("Python-3.5.8.tgz")},
            AvailableToolchain{version: "3.5.7".parse().unwrap(), url: url.join("3.5.7").unwrap(), source_tar_gz: String::from("Python-3.5.7.tgz")},
            AvailableToolchain{version: "3.5.6".parse().unwrap(), url: url.join("3.5.6").unwrap(), source_tar_gz: String::from("Python-3.5.6.tgz")},
            AvailableToolchain{version: "3.5.5".parse().unwrap(), url: url.join("3.5.5").unwrap(), source_tar_gz: String::from("Python-3.5.5.tgz")},
            AvailableToolchain{version: "3.5.4".parse().unwrap(), url: url.join("3.5.4").unwrap(), source_tar_gz: String::from("Python-3.5.4.tgz")},
            AvailableToolchain{version: "3.5.3".parse().unwrap(), url: url.join("3.5.3").unwrap(), source_tar_gz: String::from("Python-3.5.3.tgz")},
            AvailableToolchain{version: "3.5.2".parse().unwrap(), url: url.join("3.5.2").unwrap(), source_tar_gz: String::from("Python-3.5.2.tgz")},
            AvailableToolchain{version: "3.5.1".parse().unwrap(), url: url.join("3.5.1").unwrap(), source_tar_gz: String::from("Python-3.5.1.tgz")},
            AvailableToolchain{version: "3.5.0".parse().unwrap(), url: url.join("3.5.0").unwrap(), source_tar_gz: String::from("Python-3.5.0.tgz")},
            AvailableToolchain{version: "3.4.10".parse().unwrap(), url: url.join("3.4.10").unwrap(), source_tar_gz: String::from("Python-3.4.10.tgz")},
            AvailableToolchain{version: "3.4.9".parse().unwrap(), url: url.join("3.4.9").unwrap(), source_tar_gz: String::from("Python-3.4.9.tgz")},
            AvailableToolchain{version: "3.4.8".parse().unwrap(), url: url.join("3.4.8").unwrap(), source_tar_gz: String::from("Python-3.4.8.tgz")},
            AvailableToolchain{version: "3.4.7".parse().unwrap(), url: url.join("3.4.7").unwrap(), source_tar_gz: String::from("Python-3.4.7.tgz")},
            AvailableToolchain{version: "3.4.6".parse().unwrap(), url: url.join("3.4.6").unwrap(), source_tar_gz: String::from("Python-3.4.6.tgz")},
            AvailableToolchain{version: "3.4.5".parse().unwrap(), url: url.join("3.4.5").unwrap(), source_tar_gz: String::from("Python-3.4.5.tgz")},
            AvailableToolchain{version: "3.4.4".parse().unwrap(), url: url.join("3.4.4").unwrap(), source_tar_gz: String::from("Python-3.4.4.tgz")},
            AvailableToolchain{version: "3.4.3".parse().unwrap(), url: url.join("3.4.3").unwrap(), source_tar_gz: String::from("Python-3.4.3.tgz")},
            AvailableToolchain{version: "3.4.2".parse().unwrap(), url: url.join("3.4.2").unwrap(), source_tar_gz: String::from("Python-3.4.2.tgz")},
            AvailableToolchain{version: "3.4.1".parse().unwrap(), url: url.join("3.4.1").unwrap(), source_tar_gz: String::from("Python-3.4.1.tgz")},
            AvailableToolchain{version: "3.4.0".parse().unwrap(), url: url.join("3.4.0").unwrap(), source_tar_gz: String::from("Python-3.4.0.tgz")},
            AvailableToolchain{version: "3.3.7".parse().unwrap(), url: url.join("3.3.7").unwrap(), source_tar_gz: String::from("Python-3.3.7.tgz")},
            AvailableToolchain{version: "3.3.6".parse().unwrap(), url: url.join("3.3.6").unwrap(), source_tar_gz: String::from("Python-3.3.6.tgz")},
            AvailableToolchain{version: "3.3.5".parse().unwrap(), url: url.join("3.3.5").unwrap(), source_tar_gz: String::from("Python-3.3.5.tgz")},
            AvailableToolchain{version: "3.3.4".parse().unwrap(), url: url.join("3.3.4").unwrap(), source_tar_gz: String::from("Python-3.3.4.tgz")},
            AvailableToolchain{version: "3.3.3".parse().unwrap(), url: url.join("3.3.3").unwrap(), source_tar_gz: String::from("Python-3.3.3.tgz")},
            AvailableToolchain{version: "3.3.2".parse().unwrap(), url: url.join("3.3.2").unwrap(), source_tar_gz: String::from("Python-3.3.2.tgz")},
            AvailableToolchain{version: "3.3.1".parse().unwrap(), url: url.join("3.3.1").unwrap(), source_tar_gz: String::from("Python-3.3.1.tgz")},
            AvailableToolchain{version: "3.3.0".parse().unwrap(), url: url.join("3.3.0").unwrap(), source_tar_gz: String::from("Python-3.3.0.tgz")},
            AvailableToolchain{version: "3.2.6".parse().unwrap(), url: url.join("3.2.6").unwrap(), source_tar_gz: String::from("Python-3.2.6.tgz")},
            AvailableToolchain{version: "3.2.5".parse().unwrap(), url: url.join("3.2.5").unwrap(), source_tar_gz: String::from("Python-3.2.5.tgz")},
            AvailableToolchain{version: "3.2.4".parse().unwrap(), url: url.join("3.2.4").unwrap(), source_tar_gz: String::from("Python-3.2.4.tgz")},
            AvailableToolchain{version: "3.2.3".parse().unwrap(), url: url.join("3.2.3").unwrap(), source_tar_gz: String::from("Python-3.2.3.tgz")},
            AvailableToolchain{version: "3.2.2".parse().unwrap(), url: url.join("3.2.2").unwrap(), source_tar_gz: String::from("Python-3.2.2.tgz")},
            AvailableToolchain{version: "3.2.1".parse().unwrap(), url: url.join("3.2.1").unwrap(), source_tar_gz: String::from("Python-3.2.1.tgz")},
            AvailableToolchain{version: "3.2.0".parse().unwrap(), url: url.join("3.2").unwrap(), source_tar_gz: String::from("Python-3.2.tgz")},
            AvailableToolchain{version: "3.1.5".parse().unwrap(), url: url.join("3.1.5").unwrap(), source_tar_gz: String::from("Python-3.1.5.tgz")},
            AvailableToolchain{version: "3.1.4".parse().unwrap(), url: url.join("3.1.4").unwrap(), source_tar_gz: String::from("Python-3.1.4.tgz")},
            AvailableToolchain{version: "3.1.3".parse().unwrap(), url: url.join("3.1.3").unwrap(), source_tar_gz: String::from("Python-3.1.3.tgz")},
            AvailableToolchain{version: "3.1.2".parse().unwrap(), url: url.join("3.1.2").unwrap(), source_tar_gz: String::from("Python-3.1.2.tgz")},
            AvailableToolchain{version: "3.1.1".parse().unwrap(), url: url.join("3.1.1").unwrap(), source_tar_gz: String::from("Python-3.1.1.tgz")},
            AvailableToolchain{version: "3.1.0".parse().unwrap(), url: url.join("3.1").unwrap(), source_tar_gz: String::from("Python-3.1.tgz")},
            AvailableToolchain{version: "3.0.1".parse().unwrap(), url: url.join("3.0.1").unwrap(), source_tar_gz: String::from("Python-3.0.1.tgz")},
            AvailableToolchain{version: "3.0.0".parse().unwrap(), url: url.join("3.0").unwrap(), source_tar_gz: String::from("Python-3.0.tgz")},
            AvailableToolchain{version: "2.7.17".parse().unwrap(), url: url.join("2.7.17").unwrap(), source_tar_gz: String::from("Python-2.7.17.tgz")},
            AvailableToolchain{version: "2.7.16".parse().unwrap(), url: url.join("2.7.16").unwrap(), source_tar_gz: String::from("Python-2.7.16.tgz")},
            AvailableToolchain{version: "2.7.15".parse().unwrap(), url: url.join("2.7.15").unwrap(), source_tar_gz: String::from("Python-2.7.15.tgz")},
            AvailableToolchain{version: "2.7.14".parse().unwrap(), url: url.join("2.7.14").unwrap(), source_tar_gz: String::from("Python-2.7.14.tgz")},
            AvailableToolchain{version: "2.7.13".parse().unwrap(), url: url.join("2.7.13").unwrap(), source_tar_gz: String::from("Python-2.7.13.tgz")},
            AvailableToolchain{version: "2.7.12".parse().unwrap(), url: url.join("2.7.12").unwrap(), source_tar_gz: String::from("Python-2.7.12.tgz")},
            AvailableToolchain{version: "2.7.11".parse().unwrap(), url: url.join("2.7.11").unwrap(), source_tar_gz: String::from("Python-2.7.11.tgz")},
            AvailableToolchain{version: "2.7.10".parse().unwrap(), url: url.join("2.7.10").unwrap(), source_tar_gz: String::from("Python-2.7.10.tgz")},
            AvailableToolchain{version: "2.7.9".parse().unwrap(), url: url.join("2.7.9").unwrap(), source_tar_gz: String::from("Python-2.7.9.tgz")},
            AvailableToolchain{version: "2.7.8".parse().unwrap(), url: url.join("2.7.8").unwrap(), source_tar_gz: String::from("Python-2.7.8.tgz")},
            AvailableToolchain{version: "2.7.7".parse().unwrap(), url: url.join("2.7.7").unwrap(), source_tar_gz: String::from("Python-2.7.7.tgz")},
            AvailableToolchain{version: "2.7.6".parse().unwrap(), url: url.join("2.7.6").unwrap(), source_tar_gz: String::from("Python-2.7.6.tgz")},
            AvailableToolchain{version: "2.7.5".parse().unwrap(), url: url.join("2.7.5").unwrap(), source_tar_gz: String::from("Python-2.7.5.tgz")},
            AvailableToolchain{version: "2.7.4".parse().unwrap(), url: url.join("2.7.4").unwrap(), source_tar_gz: String::from("Python-2.7.4.tgz")},
            AvailableToolchain{version: "2.7.3".parse().unwrap(), url: url.join("2.7.3").unwrap(), source_tar_gz: String::from("Python-2.7.3.tgz")},
            AvailableToolchain{version: "2.7.2".parse().unwrap(), url: url.join("2.7.2").unwrap(), source_tar_gz: String::from("Python-2.7.2.tgz")},
            AvailableToolchain{version: "2.7.1".parse().unwrap(), url: url.join("2.7.1").unwrap(), source_tar_gz: String::from("Python-2.7.1.tgz")},
            AvailableToolchain{version: "2.7.0".parse().unwrap(), url: url.join("2.7").unwrap(), source_tar_gz: String::from("Python-2.7.tgz")},
            AvailableToolchain{version: "2.6.9".parse().unwrap(), url: url.join("2.6.9").unwrap(), source_tar_gz: String::from("Python-2.6.9.tgz")},
            AvailableToolchain{version: "2.6.8".parse().unwrap(), url: url.join("2.6.8").unwrap(), source_tar_gz: String::from("Python-2.6.8.tgz")},
            AvailableToolchain{version: "2.6.7".parse().unwrap(), url: url.join("2.6.7").unwrap(), source_tar_gz: String::from("Python-2.6.7.tgz")},
            AvailableToolchain{version: "2.6.6".parse().unwrap(), url: url.join("2.6.6").unwrap(), source_tar_gz: String::from("Python-2.6.6.tgz")},
            AvailableToolchain{version: "2.6.5".parse().unwrap(), url: url.join("2.6.5").unwrap(), source_tar_gz: String::from("Python-2.6.5.tgz")},
            AvailableToolchain{version: "2.6.4".parse().unwrap(), url: url.join("2.6.4").unwrap(), source_tar_gz: String::from("Python-2.6.4.tgz")},
            AvailableToolchain{version: "2.6.3".parse().unwrap(), url: url.join("2.6.3").unwrap(), source_tar_gz: String::from("Python-2.6.3.tgz")},
            AvailableToolchain{version: "2.6.2".parse().unwrap(), url: url.join("2.6.2").unwrap(), source_tar_gz: String::from("Python-2.6.2.tgz")},
            AvailableToolchain{version: "2.6.1".parse().unwrap(), url: url.join("2.6.1").unwrap(), source_tar_gz: String::from("Python-2.6.1.tgz")},
            AvailableToolchain{version: "2.6.0".parse().unwrap(), url: url.join("2.6").unwrap(), source_tar_gz: String::from("Python-2.6.tgz")},
            AvailableToolchain{version: "2.5.6".parse().unwrap(), url: url.join("2.5.6").unwrap(), source_tar_gz: String::from("Python-2.5.6.tgz")},
            AvailableToolchain{version: "2.5.5".parse().unwrap(), url: url.join("2.5.5").unwrap(), source_tar_gz: String::from("Python-2.5.5.tgz")},
            AvailableToolchain{version: "2.5.4".parse().unwrap(), url: url.join("2.5.4").unwrap(), source_tar_gz: String::from("Python-2.5.4.tgz")},
            AvailableToolchain{version: "2.5.3".parse().unwrap(), url: url.join("2.5.3").unwrap(), source_tar_gz: String::from("Python-2.5.3.tgz")},
            AvailableToolchain{version: "2.5.2".parse().unwrap(), url: url.join("2.5.2").unwrap(), source_tar_gz: String::from("Python-2.5.2.tgz")},
            AvailableToolchain{version: "2.5.1".parse().unwrap(), url: url.join("2.5.1").unwrap(), source_tar_gz: String::from("Python-2.5.1.tgz")},
            AvailableToolchain{version: "2.5.0".parse().unwrap(), url: url.join("2.5").unwrap(), source_tar_gz: String::from("Python-2.5.tgz")},
            AvailableToolchain{version: "2.4.6".parse().unwrap(), url: url.join("2.4.6").unwrap(), source_tar_gz: String::from("Python-2.4.6.tgz")},
            AvailableToolchain{version: "2.4.5".parse().unwrap(), url: url.join("2.4.5").unwrap(), source_tar_gz: String::from("Python-2.4.5.tgz")},
            AvailableToolchain{version: "2.4.4".parse().unwrap(), url: url.join("2.4.4").unwrap(), source_tar_gz: String::from("Python-2.4.4.tgz")},
            AvailableToolchain{version: "2.4.3".parse().unwrap(), url: url.join("2.4.3").unwrap(), source_tar_gz: String::from("Python-2.4.3.tgz")},
            AvailableToolchain{version: "2.4.2".parse().unwrap(), url: url.join("2.4.2").unwrap(), source_tar_gz: String::from("Python-2.4.2.tgz")},
            AvailableToolchain{version: "2.4.1".parse().unwrap(), url: url.join("2.4.1").unwrap(), source_tar_gz: String::from("Python-2.4.1.tgz")},
            AvailableToolchain{version: "2.4.0".parse().unwrap(), url: url.join("2.4").unwrap(), source_tar_gz: String::from("Python-2.4.tgz")},
            AvailableToolchain{version: "2.3.7".parse().unwrap(), url: url.join("2.3.7").unwrap(), source_tar_gz: String::from("Python-2.3.7.tgz")},
            AvailableToolchain{version: "2.3.6".parse().unwrap(), url: url.join("2.3.6").unwrap(), source_tar_gz: String::from("Python-2.3.6.tgz")},
            AvailableToolchain{version: "2.3.5".parse().unwrap(), url: url.join("2.3.5").unwrap(), source_tar_gz: String::from("Python-2.3.5.tgz")},
            AvailableToolchain{version: "2.3.4".parse().unwrap(), url: url.join("2.3.4").unwrap(), source_tar_gz: String::from("Python-2.3.4.tgz")},
            AvailableToolchain{version: "2.3.3".parse().unwrap(), url: url.join("2.3.3").unwrap(), source_tar_gz: String::from("Python-2.3.3.tgz")},
            AvailableToolchain{version: "2.3.2".parse().unwrap(), url: url.join("2.3.2").unwrap(), source_tar_gz: String::from("Python-2.3.2.tgz")},
            AvailableToolchain{version: "2.3.1".parse().unwrap(), url: url.join("2.3.1").unwrap(), source_tar_gz: String::from("Python-2.3.1.tgz")},
            AvailableToolchain{version: "2.3.0".parse().unwrap(), url: url.join("2.3").unwrap(), source_tar_gz: String::from("Python-2.3.tgz")},
            AvailableToolchain{version: "2.2.3".parse().unwrap(), url: url.join("2.2.3").unwrap(), source_tar_gz: String::from("Python-2.2.3.tgz")},
            AvailableToolchain{version: "2.2.2".parse().unwrap(), url: url.join("2.2.2").unwrap(), source_tar_gz: String::from("Python-2.2.2.tgz")},
            AvailableToolchain{version: "2.2.1".parse().unwrap(), url: url.join("2.2.1").unwrap(), source_tar_gz: String::from("Python-2.2.1.tgz")},
            AvailableToolchain{version: "2.2.0".parse().unwrap(), url: url.join("2.2").unwrap(), source_tar_gz: String::from("Python-2.2.tgz")},
            AvailableToolchain{version: "2.1.3".parse().unwrap(), url: url.join("2.1.3").unwrap(), source_tar_gz: String::from("Python-2.1.3.tgz")},
            AvailableToolchain{version: "2.0.1".parse().unwrap(), url: url.join("2.0.1").unwrap(), source_tar_gz: String::from("Python-2.0.1.tgz")},
        ];
        assert_eq!(parsed, expected);
    }
}
