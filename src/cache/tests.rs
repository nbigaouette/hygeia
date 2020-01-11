use std::fs;

use chrono::Duration;

use super::*;
use crate::utils::directory::MockPycorsHomeProviderTrait;
use pycors_test_helpers::create_test_temp_dir;

use mockall::predicate::*;

const SOURCE_INDEX_HTML: &str = include_str!("../../tests/fixtures/html/source/index.html");
const WIN_PREBUILT_INDEX_HTML: &str = include_str!("../../tests/fixtures/html/windows/index.html");

macro_rules! atwfs {
    ($version:expr, $version_url:expr, $version_archive:expr) => {{
        AvailableToolchainFromSource {
            version: $version.parse().unwrap(),
            base_url: concat!("https://www.python.org/ftp/python/", $version_url)
                .parse()
                .unwrap(),
            source_tar_gz: concat!("Python-", $version_archive, ".tgz").into(),
        }
    }};
}

macro_rules! atwpb {
    ($version:expr, $version_url:expr, $version_archive:expr) => {{
        AvailableToolchainWindowsPreBuilt {
            version: $version.parse().unwrap(),
            base_url: concat!("https://www.python.org/ftp/python/", $version_url)
                .parse()
                .unwrap(),
            win_pre_built: concat!("python-", $version_archive, "-embed-amd64.zip").into(),
        }
    }};
}

#[test]
fn cache_new_empty() {
    let home = create_test_temp_dir!();
    let project_home = home.join(".pycors");

    let mocked_home = Some(home);
    let mocked_project_home = Some(project_home.clone());

    // The test expects an empty directory
    if project_home.exists() {
        fs::remove_dir_all(&project_home).unwrap();
    }

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_project_home()
        .times(3)
        .return_const(mocked_project_home);
    mock.expect_home().times(0).return_const(mocked_home);

    let paths_provider = PycorsPathsProvider::from(mock);

    let mut mock = MockToolchainsCacheFetch::new();
    mock.expect_get()
        .times(1)
        .returning(|| Ok(SOURCE_INDEX_HTML.to_string()));
    let _cache = AvailableToolchainsCache::new(&paths_provider, &mock).unwrap();
}

#[test]
fn cache_up_to_date() {
    let home = create_test_temp_dir!();
    let project_home = home.join(".pycors");

    let mocked_home1 = Some(home.clone());
    let mocked_home2 = Some(home);

    let mocked_project_home1 = Some(project_home.clone());
    let mocked_project_home2 = Some(project_home.clone());

    // The test expects an empty directory
    if project_home.exists() {
        fs::remove_dir_all(&project_home).unwrap();
    }

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_project_home()
        .times(1)
        .return_const(mocked_project_home1);
    mock.expect_home().times(0).return_const(mocked_home1);

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

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_project_home()
        .times(2)
        .return_const(mocked_project_home2);
    mock.expect_home().times(0).return_const(mocked_home2);
    let paths_provider = PycorsPathsProvider::from(mock);

    // Let's create the cache for real
    let mut mock = MockToolchainsCacheFetch::new();
    mock.expect_get()
        .times(0) // Cache file is up to date, no download required.
        .returning(|| Ok(SOURCE_INDEX_HTML.to_string()));
    let _cache = AvailableToolchainsCache::new(&paths_provider, &mock).unwrap();
}

#[test]
fn cache_corrupted() {
    let home = create_test_temp_dir!();
    let project_home = home.join(".pycors");

    let mocked_home1 = Some(home.clone());
    let mocked_home2 = Some(home);
    let mocked_project_home1 = Some(project_home.clone());
    let mocked_project_home2 = Some(project_home.clone());

    // The test expects an empty directory
    if project_home.exists() {
        fs::remove_dir_all(&project_home).unwrap();
    }

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_project_home()
        .times(1)
        .return_const(mocked_project_home1);
    mock.expect_home().times(0).return_const(mocked_home1);

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

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_project_home()
        .times(3)
        .return_const(mocked_project_home2);
    mock.expect_home().times(0).return_const(mocked_home2);
    let paths_provider = PycorsPathsProvider::from(mock);

    let mut mock = MockToolchainsCacheFetch::new();
    mock.expect_get()
        .times(1) // Cache file is corrupted, new download required.
        .returning(|| Ok(SOURCE_INDEX_HTML.to_string()));
    let _cache = AvailableToolchainsCache::new(&paths_provider, &mock).unwrap();
}

#[test]
fn cache_outdated() {
    let home = create_test_temp_dir!();
    let project_home = home.join(".pycors");

    let mocked_home1 = Some(home.clone());
    let mocked_home2 = Some(home);
    let mocked_project_home1 = Some(project_home.clone());
    let mocked_project_home2 = Some(project_home.clone());

    // The test expects an empty directory
    if project_home.exists() {
        fs::remove_dir_all(&project_home).unwrap();
    }

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_project_home()
        .times(1)
        .return_const(mocked_project_home1);
    mock.expect_home().times(0).return_const(mocked_home1);

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

    let mut mock = MockPycorsHomeProviderTrait::new();
    mock.expect_project_home()
        .times(3)
        .return_const(mocked_project_home2);
    mock.expect_home().times(0).return_const(mocked_home2);
    let paths_provider = PycorsPathsProvider::from(mock);

    let mut mock = MockToolchainsCacheFetch::new();
    mock.expect_get()
        .times(1) // Cache file is outdated, new download required.
        .returning(|| Ok(SOURCE_INDEX_HTML.to_string()));
    let _cache = AvailableToolchainsCache::new(&paths_provider, &mock).unwrap();
}

#[test]
fn parse_source_html() {
    let parsed: Vec<AvailableToolchainFromSource> =
        parse_source_index_html(SOURCE_INDEX_HTML).unwrap();
    assert_eq!(parsed.len(), 217);

    assert_eq!(parsed[0].version, Version::parse("3.9.0-a2").unwrap());
    assert_eq!(
        parsed[0].base_url,
        Url::parse("https://www.python.org/ftp/python/3.9.0").unwrap()
    );
    assert_eq!(parsed[0].source_tar_gz, "Python-3.9.0a2.tgz");

    #[rustfmt::skip]
    let expected: Vec<AvailableToolchainFromSource> = vec![
        atwfs!("3.9.0-a2", "3.9.0", "3.9.0a2"),
        atwfs!("3.9.0-a1", "3.9.0", "3.9.0a1"),
        atwfs!("3.8.1", "3.8.1", "3.8.1"),
        atwfs!("3.8.1-rc1", "3.8.1", "3.8.1rc1"),
        atwfs!("3.8.0", "3.8.0", "3.8.0"),
        atwfs!("3.8.0-rc1", "3.8.0", "3.8.0rc1"),
        atwfs!("3.8.0-b4", "3.8.0", "3.8.0b4"),
        atwfs!("3.8.0-b3", "3.8.0", "3.8.0b3"),
        atwfs!("3.8.0-b2", "3.8.0", "3.8.0b2"),
        atwfs!("3.8.0-b1", "3.8.0", "3.8.0b1"),
        atwfs!("3.8.0-a4", "3.8.0", "3.8.0a4"),
        atwfs!("3.8.0-a3", "3.8.0", "3.8.0a3"),
        atwfs!("3.8.0-a2", "3.8.0", "3.8.0a2"),
        atwfs!("3.8.0-a1", "3.8.0", "3.8.0a1"),
        atwfs!("3.7.6", "3.7.6", "3.7.6"),
        atwfs!("3.7.6-rc1", "3.7.6", "3.7.6rc1"),
        atwfs!("3.7.5", "3.7.5", "3.7.5"),
        atwfs!("3.7.5-rc1", "3.7.5", "3.7.5rc1"),
        atwfs!("3.7.4", "3.7.4", "3.7.4"),
        atwfs!("3.7.4-rc1", "3.7.4", "3.7.4rc1"),
        atwfs!("3.7.3", "3.7.3", "3.7.3"),
        atwfs!("3.7.3-rc1", "3.7.3", "3.7.3rc1"),
        atwfs!("3.7.2", "3.7.2", "3.7.2"),
        atwfs!("3.7.2-rc1", "3.7.2", "3.7.2rc1"),
        atwfs!("3.7.1", "3.7.1", "3.7.1"),
        atwfs!("3.7.1-rc2", "3.7.1", "3.7.1rc2"),
        atwfs!("3.7.1-rc1", "3.7.1", "3.7.1rc1"),
        atwfs!("3.7.0", "3.7.0", "3.7.0"),
        atwfs!("3.7.0-rc1", "3.7.0", "3.7.0rc1"),
        atwfs!("3.7.0-b5", "3.7.0", "3.7.0b5"),
        atwfs!("3.7.0-b2", "3.7.0", "3.7.0b2"),
        atwfs!("3.7.0-b1", "3.7.0", "3.7.0b1"),
        atwfs!("3.7.0-a4", "3.7.0", "3.7.0a4"),
        atwfs!("3.7.0-a3", "3.7.0", "3.7.0a3"),
        atwfs!("3.7.0-a2", "3.7.0", "3.7.0a2"),
        atwfs!("3.7.0-a1", "3.7.0", "3.7.0a1"),
        atwfs!("3.6.10", "3.6.10", "3.6.10"),
        atwfs!("3.6.10-rc1", "3.6.10", "3.6.10rc1"),
        atwfs!("3.6.9", "3.6.9", "3.6.9"),
        atwfs!("3.6.9-rc1", "3.6.9", "3.6.9rc1"),
        atwfs!("3.6.8", "3.6.8", "3.6.8"),
        atwfs!("3.6.8-rc1", "3.6.8", "3.6.8rc1"),
        atwfs!("3.6.7", "3.6.7", "3.6.7"),
        atwfs!("3.6.7-rc2", "3.6.7", "3.6.7rc2"),
        atwfs!("3.6.7-rc1", "3.6.7", "3.6.7rc1"),
        atwfs!("3.6.6", "3.6.6", "3.6.6"),
        atwfs!("3.6.6-rc1", "3.6.6", "3.6.6rc1"),
        atwfs!("3.6.5", "3.6.5", "3.6.5"),
        atwfs!("3.6.5-rc1", "3.6.5", "3.6.5rc1"),
        atwfs!("3.6.4", "3.6.4", "3.6.4"),
        atwfs!("3.6.4-rc1", "3.6.4", "3.6.4rc1"),
        atwfs!("3.6.3", "3.6.3", "3.6.3"),
        atwfs!("3.6.3-rc1", "3.6.3", "3.6.3rc1"),
        atwfs!("3.6.2", "3.6.2", "3.6.2"),
        atwfs!("3.6.2-rc2", "3.6.2", "3.6.2rc2"),
        atwfs!("3.6.2-rc1", "3.6.2", "3.6.2rc1"),
        atwfs!("3.6.1", "3.6.1", "3.6.1"),
        atwfs!("3.6.1-rc1", "3.6.1", "3.6.1rc1"),
        atwfs!("3.6.0", "3.6.0", "3.6.0"),
        atwfs!("3.6.0-rc2", "3.6.0", "3.6.0rc2"),
        atwfs!("3.6.0-rc1", "3.6.0", "3.6.0rc1"),
        atwfs!("3.6.0-b4", "3.6.0", "3.6.0b4"),
        atwfs!("3.6.0-b3", "3.6.0", "3.6.0b3"),
        atwfs!("3.6.0-b2", "3.6.0", "3.6.0b2"),
        atwfs!("3.6.0-b1", "3.6.0", "3.6.0b1"),
        atwfs!("3.6.0-a4", "3.6.0", "3.6.0a4"),
        atwfs!("3.6.0-a3", "3.6.0", "3.6.0a3"),
        atwfs!("3.6.0-a2", "3.6.0", "3.6.0a2"),
        atwfs!("3.6.0-a1", "3.6.0", "3.6.0a1"),
        atwfs!("3.5.9", "3.5.9", "3.5.9"),
        atwfs!("3.5.8", "3.5.8", "3.5.8"),
        atwfs!("3.5.8-rc2", "3.5.8", "3.5.8rc2"),
        atwfs!("3.5.8-rc1", "3.5.8", "3.5.8rc1"),
        atwfs!("3.5.7", "3.5.7", "3.5.7"),
        atwfs!("3.5.7-rc1", "3.5.7", "3.5.7rc1"),
        atwfs!("3.5.6", "3.5.6", "3.5.6"),
        atwfs!("3.5.6-rc1", "3.5.6", "3.5.6rc1"),
        atwfs!("3.5.5", "3.5.5", "3.5.5"),
        atwfs!("3.5.5-rc1", "3.5.5", "3.5.5rc1"),
        atwfs!("3.5.4", "3.5.4", "3.5.4"),
        atwfs!("3.5.4-rc1", "3.5.4", "3.5.4rc1"),
        atwfs!("3.5.3", "3.5.3", "3.5.3"),
        atwfs!("3.5.3-rc1", "3.5.3", "3.5.3rc1"),
        atwfs!("3.5.2", "3.5.2", "3.5.2"),
        atwfs!("3.5.2-rc1", "3.5.2", "3.5.2rc1"),
        atwfs!("3.5.1", "3.5.1", "3.5.1"),
        atwfs!("3.5.1-rc1", "3.5.1", "3.5.1rc1"),
        atwfs!("3.5.0", "3.5.0", "3.5.0"),
        atwfs!("3.5.0-rc4", "3.5.0", "3.5.0rc4"),
        atwfs!("3.5.0-rc3", "3.5.0", "3.5.0rc3"),
        atwfs!("3.5.0-rc2", "3.5.0", "3.5.0rc2"),
        atwfs!("3.5.0-rc1", "3.5.0", "3.5.0rc1"),
        atwfs!("3.5.0-b4", "3.5.0", "3.5.0b4"),
        atwfs!("3.5.0-b3", "3.5.0", "3.5.0b3"),
        atwfs!("3.5.0-b2", "3.5.0", "3.5.0b2"),
        atwfs!("3.5.0-b1", "3.5.0", "3.5.0b1"),
        atwfs!("3.5.0-a4", "3.5.0", "3.5.0a4"),
        atwfs!("3.5.0-a3", "3.5.0", "3.5.0a3"),
        atwfs!("3.5.0-a2", "3.5.0", "3.5.0a2"),
        atwfs!("3.5.0-a1", "3.5.0", "3.5.0a1"),
        atwfs!("3.4.10", "3.4.10", "3.4.10"),
        atwfs!("3.4.10-rc1", "3.4.10", "3.4.10rc1"),
        atwfs!("3.4.9", "3.4.9", "3.4.9"),
        atwfs!("3.4.9-rc1", "3.4.9", "3.4.9rc1"),
        atwfs!("3.4.8", "3.4.8", "3.4.8"),
        atwfs!("3.4.8-rc1", "3.4.8", "3.4.8rc1"),
        atwfs!("3.4.7", "3.4.7", "3.4.7"),
        atwfs!("3.4.7-rc1", "3.4.7", "3.4.7rc1"),
        atwfs!("3.4.6", "3.4.6", "3.4.6"),
        atwfs!("3.4.6-rc1", "3.4.6", "3.4.6rc1"),
        atwfs!("3.4.5", "3.4.5", "3.4.5"),
        atwfs!("3.4.5-rc1", "3.4.5", "3.4.5rc1"),
        atwfs!("3.4.4", "3.4.4", "3.4.4"),
        atwfs!("3.4.4-rc1", "3.4.4", "3.4.4rc1"),
        atwfs!("3.4.3", "3.4.3", "3.4.3"),
        atwfs!("3.4.3-rc1", "3.4.3", "3.4.3rc1"),
        atwfs!("3.4.2", "3.4.2", "3.4.2"),
        atwfs!("3.4.2-rc1", "3.4.2", "3.4.2rc1"),
        atwfs!("3.4.1", "3.4.1", "3.4.1"),
        atwfs!("3.4.1-rc1", "3.4.1", "3.4.1rc1"),
        atwfs!("3.4.0", "3.4.0", "3.4.0"),
        atwfs!("3.4.0-rc3", "3.4.0", "3.4.0rc3"),
        atwfs!("3.3.7", "3.3.7", "3.3.7"),
        atwfs!("3.3.7-rc1", "3.3.7", "3.3.7rc1"),
        atwfs!("3.3.6", "3.3.6", "3.3.6"),
        atwfs!("3.3.6-rc1", "3.3.6", "3.3.6rc1"),
        atwfs!("3.3.5", "3.3.5", "3.3.5"),
        atwfs!("3.3.5-rc2", "3.3.5", "3.3.5rc2"),
        atwfs!("3.3.5-rc1", "3.3.5", "3.3.5rc1"),
        atwfs!("3.3.5-rc1", "3.3.5", "3.3.5rc1"),
        atwfs!("3.3.4", "3.3.4", "3.3.4"),
        atwfs!("3.3.3", "3.3.3", "3.3.3"),
        atwfs!("3.3.2", "3.3.2", "3.3.2"),
        atwfs!("3.3.1", "3.3.1", "3.3.1"),
        atwfs!("3.3.0", "3.3.0", "3.3.0"),
        atwfs!("3.2.6", "3.2.6", "3.2.6"),
        atwfs!("3.2.6-rc1", "3.2.6", "3.2.6rc1"),
        atwfs!("3.2.5", "3.2.5", "3.2.5"),
        atwfs!("3.2.4", "3.2.4", "3.2.4"),
        atwfs!("3.2.3", "3.2.3", "3.2.3"),
        atwfs!("3.2.2", "3.2.2", "3.2.2"),
        atwfs!("3.2.1", "3.2.1", "3.2.1"),
        atwfs!("3.2.0", "3.2", "3.2"),
        atwfs!("3.1.5", "3.1.5", "3.1.5"),
        atwfs!("3.1.4", "3.1.4", "3.1.4"),
        atwfs!("3.1.3", "3.1.3", "3.1.3"),
        atwfs!("3.1.2", "3.1.2", "3.1.2"),
        atwfs!("3.1.1", "3.1.1", "3.1.1"),
        atwfs!("3.1.0", "3.1", "3.1"),
        atwfs!("3.0.1", "3.0.1", "3.0.1"),
        atwfs!("3.0.0", "3.0", "3.0"),
        atwfs!("2.7.17", "2.7.17", "2.7.17"),
        atwfs!("2.7.17-rc1", "2.7.17", "2.7.17rc1"),
        atwfs!("2.7.16", "2.7.16", "2.7.16"),
        atwfs!("2.7.16-rc1", "2.7.16", "2.7.16rc1"),
        atwfs!("2.7.15", "2.7.15", "2.7.15"),
        atwfs!("2.7.15-rc1", "2.7.15", "2.7.15rc1"),
        atwfs!("2.7.14", "2.7.14", "2.7.14"),
        atwfs!("2.7.14-rc1", "2.7.14", "2.7.14rc1"),
        atwfs!("2.7.13", "2.7.13", "2.7.13"),
        atwfs!("2.7.13-rc1", "2.7.13", "2.7.13rc1"),
        atwfs!("2.7.12", "2.7.12", "2.7.12"),
        atwfs!("2.7.12-rc1", "2.7.12", "2.7.12rc1"),
        atwfs!("2.7.11", "2.7.11", "2.7.11"),
        atwfs!("2.7.11-rc1", "2.7.11", "2.7.11rc1"),
        atwfs!("2.7.10", "2.7.10", "2.7.10"),
        atwfs!("2.7.10-rc1", "2.7.10", "2.7.10rc1"),
        atwfs!("2.7.9", "2.7.9", "2.7.9"),
        atwfs!("2.7.9-rc1", "2.7.9", "2.7.9rc1"),
        atwfs!("2.7.8", "2.7.8", "2.7.8"),
        atwfs!("2.7.7", "2.7.7", "2.7.7"),
        atwfs!("2.7.7-rc1", "2.7.7", "2.7.7rc1"),
        atwfs!("2.7.6", "2.7.6", "2.7.6"),
        atwfs!("2.7.5", "2.7.5", "2.7.5"),
        atwfs!("2.7.4", "2.7.4", "2.7.4"),
        atwfs!("2.7.3", "2.7.3", "2.7.3"),
        atwfs!("2.7.2", "2.7.2", "2.7.2"),
        atwfs!("2.7.1", "2.7.1", "2.7.1"),
        atwfs!("2.7.0", "2.7", "2.7"),
        atwfs!("2.6.9", "2.6.9", "2.6.9"),
        atwfs!("2.6.8", "2.6.8", "2.6.8"),
        atwfs!("2.6.7", "2.6.7", "2.6.7"),
        atwfs!("2.6.6", "2.6.6", "2.6.6"),
        atwfs!("2.6.5", "2.6.5", "2.6.5"),
        atwfs!("2.6.4", "2.6.4", "2.6.4"),
        atwfs!("2.6.3", "2.6.3", "2.6.3"),
        atwfs!("2.6.2", "2.6.2", "2.6.2"),
        atwfs!("2.6.1", "2.6.1", "2.6.1"),
        atwfs!("2.6.0", "2.6", "2.6"),
        atwfs!("2.5.6", "2.5.6", "2.5.6"),
        atwfs!("2.5.5", "2.5.5", "2.5.5"),
        atwfs!("2.5.4", "2.5.4", "2.5.4"),
        atwfs!("2.5.3", "2.5.3", "2.5.3"),
        atwfs!("2.5.2", "2.5.2", "2.5.2"),
        atwfs!("2.5.1", "2.5.1", "2.5.1"),
        atwfs!("2.5.0", "2.5", "2.5"),
        atwfs!("2.4.6", "2.4.6", "2.4.6"),
        atwfs!("2.4.5", "2.4.5", "2.4.5"),
        atwfs!("2.4.4", "2.4.4", "2.4.4"),
        atwfs!("2.4.3", "2.4.3", "2.4.3"),
        atwfs!("2.4.2", "2.4.2", "2.4.2"),
        atwfs!("2.4.1", "2.4.1", "2.4.1"),
        atwfs!("2.4.0", "2.4", "2.4"),
        atwfs!("2.3.7", "2.3.7", "2.3.7"),
        atwfs!("2.3.6", "2.3.6", "2.3.6"),
        atwfs!("2.3.5", "2.3.5", "2.3.5"),
        atwfs!("2.3.4", "2.3.4", "2.3.4"),
        atwfs!("2.3.3", "2.3.3", "2.3.3"),
        atwfs!("2.3.2", "2.3.2", "2.3.2"),
        atwfs!("2.3.1", "2.3.1", "2.3.1"),
        atwfs!("2.3.0", "2.3", "2.3"),
        atwfs!("2.2.3", "2.2.3", "2.2.3"),
        atwfs!("2.2.2", "2.2.2", "2.2.2"),
        atwfs!("2.2.1", "2.2.1", "2.2.1"),
        atwfs!("2.2.0", "2.2", "2.2"),
        atwfs!("2.1.3", "2.1.3", "2.1.3"),
        atwfs!("2.0.1", "2.0.1", "2.0.1"),
    ];
    assert_eq!(parsed, expected);
}

#[test]
fn parse_win_prebuilt_html() {
    let parsed: Vec<AvailableToolchainWindowsPreBuilt> =
        parse_win_pre_built_index_html(WIN_PREBUILT_INDEX_HTML).unwrap();
    assert_eq!(parsed.len(), 82);

    // #[rustfmt::skip]
    let expected: Vec<AvailableToolchainWindowsPreBuilt> = vec![
        atwpb!("3.9.0-a2", "3.9.0", "3.9.0a2"),
        atwpb!("3.9.0-a1", "3.9.0", "3.9.0a1"),
        atwpb!("3.8.1", "3.8.1", "3.8.1"),
        atwpb!("3.8.1-rc1", "3.8.1", "3.8.1rc1"),
        atwpb!("3.8.0", "3.8.0", "3.8.0"),
        atwpb!("3.8.0-rc1", "3.8.0", "3.8.0rc1"),
        atwpb!("3.8.0-b4", "3.8.0", "3.8.0b4"),
        atwpb!("3.8.0-b3", "3.8.0", "3.8.0b3"),
        atwpb!("3.8.0-b2", "3.8.0", "3.8.0b2"),
        atwpb!("3.8.0-b1", "3.8.0", "3.8.0b1"),
        atwpb!("3.8.0-a4", "3.8.0", "3.8.0a4"),
        atwpb!("3.8.0-a3", "3.8.0", "3.8.0a3"),
        atwpb!("3.8.0-a2", "3.8.0", "3.8.0a2"),
        atwpb!("3.8.0-a1", "3.8.0", "3.8.0a1"),
        atwpb!("3.7.6", "3.7.6", "3.7.6"),
        atwpb!("3.7.6-rc1", "3.7.6", "3.7.6rc1"),
        atwpb!("3.7.5", "3.7.5", "3.7.5"),
        atwpb!("3.7.5-rc1", "3.7.5", "3.7.5rc1"),
        atwpb!("3.7.4", "3.7.4", "3.7.4"),
        atwpb!("3.7.4-rc1", "3.7.4", "3.7.4rc1"),
        atwpb!("3.7.3", "3.7.3", "3.7.3"),
        atwpb!("3.7.3-rc1", "3.7.3", "3.7.3rc1"),
        atwpb!("3.7.2", "3.7.2", "3.7.2.post1"),
        atwpb!("3.7.2-rc1", "3.7.2", "3.7.2rc1"),
        atwpb!("3.7.1", "3.7.1", "3.7.1"),
        atwpb!("3.7.1-rc2", "3.7.1", "3.7.1rc2"),
        atwpb!("3.7.1-rc1", "3.7.1", "3.7.1rc1"),
        atwpb!("3.7.0", "3.7.0", "3.7.0"),
        atwpb!("3.7.0-rc1", "3.7.0", "3.7.0rc1"),
        atwpb!("3.7.0-b5", "3.7.0", "3.7.0b5"),
        atwpb!("3.7.0-b2", "3.7.0", "3.7.0b2"),
        atwpb!("3.7.0-b1", "3.7.0", "3.7.0b1"),
        atwpb!("3.7.0-a4", "3.7.0", "3.7.0a4"),
        atwpb!("3.7.0-a3", "3.7.0", "3.7.0a3"),
        atwpb!("3.7.0-a2", "3.7.0", "3.7.0a2"),
        atwpb!("3.7.0-a1", "3.7.0", "3.7.0a1"),
        atwpb!("3.6.8", "3.6.8", "3.6.8"),
        atwpb!("3.6.8-rc1", "3.6.8", "3.6.8rc1"),
        atwpb!("3.6.7", "3.6.7", "3.6.7"),
        atwpb!("3.6.7-rc2", "3.6.7", "3.6.7rc2"),
        atwpb!("3.6.7-rc1", "3.6.7", "3.6.7rc1"),
        atwpb!("3.6.6", "3.6.6", "3.6.6"),
        atwpb!("3.6.6-rc1", "3.6.6", "3.6.6rc1"),
        atwpb!("3.6.5", "3.6.5", "3.6.5"),
        atwpb!("3.6.5-rc1", "3.6.5", "3.6.5rc1"),
        atwpb!("3.6.4", "3.6.4", "3.6.4"),
        atwpb!("3.6.4-rc1", "3.6.4", "3.6.4rc1"),
        atwpb!("3.6.3", "3.6.3", "3.6.3"),
        atwpb!("3.6.3-rc1", "3.6.3", "3.6.3rc1"),
        atwpb!("3.6.2", "3.6.2", "3.6.2"),
        atwpb!("3.6.2-rc2", "3.6.2", "3.6.2rc2"),
        atwpb!("3.6.2-rc1", "3.6.2", "3.6.2rc1"),
        atwpb!("3.6.1", "3.6.1", "3.6.1"),
        atwpb!("3.6.1-rc1", "3.6.1", "3.6.1rc1"),
        atwpb!("3.6.0", "3.6.0", "3.6.0"),
        atwpb!("3.6.0-rc2", "3.6.0", "3.6.0rc2"),
        atwpb!("3.6.0-rc1", "3.6.0", "3.6.0rc1"),
        atwpb!("3.6.0-b4", "3.6.0", "3.6.0b4"),
        atwpb!("3.6.0-b3", "3.6.0", "3.6.0b3"),
        atwpb!("3.6.0-b2", "3.6.0", "3.6.0b2"),
        atwpb!("3.6.0-b1", "3.6.0", "3.6.0b1"),
        atwpb!("3.6.0-a4", "3.6.0", "3.6.0a4"),
        atwpb!("3.6.0-a3", "3.6.0", "3.6.0a3"),
        atwpb!("3.6.0-a2", "3.6.0", "3.6.0a2"),
        atwpb!("3.6.0-a1", "3.6.0", "3.6.0a1"),
        atwpb!("3.5.4", "3.5.4", "3.5.4"),
        atwpb!("3.5.4-rc1", "3.5.4", "3.5.4rc1"),
        atwpb!("3.5.3", "3.5.3", "3.5.3"),
        atwpb!("3.5.3-rc1", "3.5.3", "3.5.3rc1"),
        atwpb!("3.5.2", "3.5.2", "3.5.2"),
        atwpb!("3.5.2-rc1", "3.5.2", "3.5.2rc1"),
        atwpb!("3.5.1", "3.5.1", "3.5.1"),
        atwpb!("3.5.1-rc1", "3.5.1", "3.5.1rc1"),
        atwpb!("3.5.0", "3.5.0", "3.5.0"),
        atwpb!("3.5.0-rc4", "3.5.0", "3.5.0rc4"),
        atwpb!("3.5.0-rc3", "3.5.0", "3.5.0rc3"),
        atwpb!("3.5.0-rc2", "3.5.0", "3.5.0rc2"),
        atwpb!("3.5.0-rc1", "3.5.0", "3.5.0rc1"),
        atwpb!("3.5.0-b4", "3.5.0", "3.5.0b4"),
        atwpb!("3.5.0-b3", "3.5.0", "3.5.0b3"),
        atwpb!("3.5.0-b2", "3.5.0", "3.5.0b2"),
        atwpb!("3.5.0-b1", "3.5.0", "3.5.0b1"),
    ];
    assert_eq!(parsed, expected);
}
