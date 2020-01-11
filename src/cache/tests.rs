use std::fs;

use chrono::Duration;

use super::*;
use crate::utils::directory::MockPycorsHomeProviderTrait;
use pycors_test_helpers::create_test_temp_dir;

use mockall::predicate::*;

const INDEX_HTML: &str = include_str!("../../tests/fixtures/index.html");

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
        .returning(|| Ok(INDEX_HTML.to_string()));
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
        .returning(|| Ok(INDEX_HTML.to_string()));
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
        .returning(|| Ok(INDEX_HTML.to_string()));
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
        .returning(|| Ok(INDEX_HTML.to_string()));
    let _cache = AvailableToolchainsCache::new(&paths_provider, &mock).unwrap();
}

#[test]
fn parse_html() {
    let parsed: Vec<AvailableToolchain> = parse_index_html(INDEX_HTML).unwrap();
    assert_eq!(parsed.len(), 213);

    assert_eq!(
        parsed[0].source_url(),
        Url::parse("https://www.python.org/ftp/python/3.9.0/Python-3.9.0a1.tgz").unwrap()
    );

    #[rustfmt::skip]
    let expected: Vec<AvailableToolchain> = vec![
        AvailableToolchain { version: "3.9.0-a1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.9.0".parse().unwrap(), source_tar_gz: "Python-3.9.0a1.tgz".into() },
        AvailableToolchain { version: "3.8.1-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.8.1".parse().unwrap(), source_tar_gz: "Python-3.8.1rc1.tgz".into() },
        AvailableToolchain { version: "3.8.0".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.8.0".parse().unwrap(), source_tar_gz: "Python-3.8.0.tgz".into() },
        AvailableToolchain { version: "3.8.0-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.8.0".parse().unwrap(), source_tar_gz: "Python-3.8.0rc1.tgz".into() },
        AvailableToolchain { version: "3.8.0-b4".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.8.0".parse().unwrap(), source_tar_gz: "Python-3.8.0b4.tgz".into() },
        AvailableToolchain { version: "3.8.0-b3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.8.0".parse().unwrap(), source_tar_gz: "Python-3.8.0b3.tgz".into() },
        AvailableToolchain { version: "3.8.0-b2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.8.0".parse().unwrap(), source_tar_gz: "Python-3.8.0b2.tgz".into() },
        AvailableToolchain { version: "3.8.0-b1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.8.0".parse().unwrap(), source_tar_gz: "Python-3.8.0b1.tgz".into() },
        AvailableToolchain { version: "3.8.0-a4".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.8.0".parse().unwrap(), source_tar_gz: "Python-3.8.0a4.tgz".into() },
        AvailableToolchain { version: "3.8.0-a3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.8.0".parse().unwrap(), source_tar_gz: "Python-3.8.0a3.tgz".into() },
        AvailableToolchain { version: "3.8.0-a2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.8.0".parse().unwrap(), source_tar_gz: "Python-3.8.0a2.tgz".into() },
        AvailableToolchain { version: "3.8.0-a1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.8.0".parse().unwrap(), source_tar_gz: "Python-3.8.0a1.tgz".into() },
        AvailableToolchain { version: "3.7.6-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.7.6".parse().unwrap(), source_tar_gz: "Python-3.7.6rc1.tgz".into() },
        AvailableToolchain { version: "3.7.5".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.7.5".parse().unwrap(), source_tar_gz: "Python-3.7.5.tgz".into() },
        AvailableToolchain { version: "3.7.5-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.7.5".parse().unwrap(), source_tar_gz: "Python-3.7.5rc1.tgz".into() },
        AvailableToolchain { version: "3.7.4".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.7.4".parse().unwrap(), source_tar_gz: "Python-3.7.4.tgz".into() },
        AvailableToolchain { version: "3.7.4-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.7.4".parse().unwrap(), source_tar_gz: "Python-3.7.4rc1.tgz".into() },
        AvailableToolchain { version: "3.7.3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.7.3".parse().unwrap(), source_tar_gz: "Python-3.7.3.tgz".into() },
        AvailableToolchain { version: "3.7.3-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.7.3".parse().unwrap(), source_tar_gz: "Python-3.7.3rc1.tgz".into() },
        AvailableToolchain { version: "3.7.2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.7.2".parse().unwrap(), source_tar_gz: "Python-3.7.2.tgz".into() },
        AvailableToolchain { version: "3.7.2-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.7.2".parse().unwrap(), source_tar_gz: "Python-3.7.2rc1.tgz".into() },
        AvailableToolchain { version: "3.7.1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.7.1".parse().unwrap(), source_tar_gz: "Python-3.7.1.tgz".into() },
        AvailableToolchain { version: "3.7.1-rc2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.7.1".parse().unwrap(), source_tar_gz: "Python-3.7.1rc2.tgz".into() },
        AvailableToolchain { version: "3.7.1-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.7.1".parse().unwrap(), source_tar_gz: "Python-3.7.1rc1.tgz".into() },
        AvailableToolchain { version: "3.7.0".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.7.0".parse().unwrap(), source_tar_gz: "Python-3.7.0.tgz".into() },
        AvailableToolchain { version: "3.7.0-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.7.0".parse().unwrap(), source_tar_gz: "Python-3.7.0rc1.tgz".into() },
        AvailableToolchain { version: "3.7.0-b5".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.7.0".parse().unwrap(), source_tar_gz: "Python-3.7.0b5.tgz".into() },
        AvailableToolchain { version: "3.7.0-b2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.7.0".parse().unwrap(), source_tar_gz: "Python-3.7.0b2.tgz".into() },
        AvailableToolchain { version: "3.7.0-b1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.7.0".parse().unwrap(), source_tar_gz: "Python-3.7.0b1.tgz".into() },
        AvailableToolchain { version: "3.7.0-a4".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.7.0".parse().unwrap(), source_tar_gz: "Python-3.7.0a4.tgz".into() },
        AvailableToolchain { version: "3.7.0-a3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.7.0".parse().unwrap(), source_tar_gz: "Python-3.7.0a3.tgz".into() },
        AvailableToolchain { version: "3.7.0-a2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.7.0".parse().unwrap(), source_tar_gz: "Python-3.7.0a2.tgz".into() },
        AvailableToolchain { version: "3.7.0-a1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.7.0".parse().unwrap(), source_tar_gz: "Python-3.7.0a1.tgz".into() },
        AvailableToolchain { version: "3.6.10-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.10".parse().unwrap(), source_tar_gz: "Python-3.6.10rc1.tgz".into() },
        AvailableToolchain { version: "3.6.9".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.9".parse().unwrap(), source_tar_gz: "Python-3.6.9.tgz".into() },
        AvailableToolchain { version: "3.6.9-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.9".parse().unwrap(), source_tar_gz: "Python-3.6.9rc1.tgz".into() },
        AvailableToolchain { version: "3.6.8".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.8".parse().unwrap(), source_tar_gz: "Python-3.6.8.tgz".into() },
        AvailableToolchain { version: "3.6.8-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.8".parse().unwrap(), source_tar_gz: "Python-3.6.8rc1.tgz".into() },
        AvailableToolchain { version: "3.6.7".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.7".parse().unwrap(), source_tar_gz: "Python-3.6.7.tgz".into() },
        AvailableToolchain { version: "3.6.7-rc2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.7".parse().unwrap(), source_tar_gz: "Python-3.6.7rc2.tgz".into() },
        AvailableToolchain { version: "3.6.7-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.7".parse().unwrap(), source_tar_gz: "Python-3.6.7rc1.tgz".into() },
        AvailableToolchain { version: "3.6.6".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.6".parse().unwrap(), source_tar_gz: "Python-3.6.6.tgz".into() },
        AvailableToolchain { version: "3.6.6-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.6".parse().unwrap(), source_tar_gz: "Python-3.6.6rc1.tgz".into() },
        AvailableToolchain { version: "3.6.5".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.5".parse().unwrap(), source_tar_gz: "Python-3.6.5.tgz".into() },
        AvailableToolchain { version: "3.6.5-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.5".parse().unwrap(), source_tar_gz: "Python-3.6.5rc1.tgz".into() },
        AvailableToolchain { version: "3.6.4".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.4".parse().unwrap(), source_tar_gz: "Python-3.6.4.tgz".into() },
        AvailableToolchain { version: "3.6.4-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.4".parse().unwrap(), source_tar_gz: "Python-3.6.4rc1.tgz".into() },
        AvailableToolchain { version: "3.6.3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.3".parse().unwrap(), source_tar_gz: "Python-3.6.3.tgz".into() },
        AvailableToolchain { version: "3.6.3-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.3".parse().unwrap(), source_tar_gz: "Python-3.6.3rc1.tgz".into() },
        AvailableToolchain { version: "3.6.2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.2".parse().unwrap(), source_tar_gz: "Python-3.6.2.tgz".into() },
        AvailableToolchain { version: "3.6.2-rc2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.2".parse().unwrap(), source_tar_gz: "Python-3.6.2rc2.tgz".into() },
        AvailableToolchain { version: "3.6.2-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.2".parse().unwrap(), source_tar_gz: "Python-3.6.2rc1.tgz".into() },
        AvailableToolchain { version: "3.6.1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.1".parse().unwrap(), source_tar_gz: "Python-3.6.1.tgz".into() },
        AvailableToolchain { version: "3.6.1-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.1".parse().unwrap(), source_tar_gz: "Python-3.6.1rc1.tgz".into() },
        AvailableToolchain { version: "3.6.0".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.0".parse().unwrap(), source_tar_gz: "Python-3.6.0.tgz".into() },
        AvailableToolchain { version: "3.6.0-rc2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.0".parse().unwrap(), source_tar_gz: "Python-3.6.0rc2.tgz".into() },
        AvailableToolchain { version: "3.6.0-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.0".parse().unwrap(), source_tar_gz: "Python-3.6.0rc1.tgz".into() },
        AvailableToolchain { version: "3.6.0-b4".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.0".parse().unwrap(), source_tar_gz: "Python-3.6.0b4.tgz".into() },
        AvailableToolchain { version: "3.6.0-b3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.0".parse().unwrap(), source_tar_gz: "Python-3.6.0b3.tgz".into() },
        AvailableToolchain { version: "3.6.0-b2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.0".parse().unwrap(), source_tar_gz: "Python-3.6.0b2.tgz".into() },
        AvailableToolchain { version: "3.6.0-b1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.0".parse().unwrap(), source_tar_gz: "Python-3.6.0b1.tgz".into() },
        AvailableToolchain { version: "3.6.0-a4".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.0".parse().unwrap(), source_tar_gz: "Python-3.6.0a4.tgz".into() },
        AvailableToolchain { version: "3.6.0-a3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.0".parse().unwrap(), source_tar_gz: "Python-3.6.0a3.tgz".into() },
        AvailableToolchain { version: "3.6.0-a2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.0".parse().unwrap(), source_tar_gz: "Python-3.6.0a2.tgz".into() },
        AvailableToolchain { version: "3.6.0-a1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.6.0".parse().unwrap(), source_tar_gz: "Python-3.6.0a1.tgz".into() },
        AvailableToolchain { version: "3.5.9".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.9".parse().unwrap(), source_tar_gz: "Python-3.5.9.tgz".into() },
        AvailableToolchain { version: "3.5.8".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.8".parse().unwrap(), source_tar_gz: "Python-3.5.8.tgz".into() },
        AvailableToolchain { version: "3.5.8-rc2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.8".parse().unwrap(), source_tar_gz: "Python-3.5.8rc2.tgz".into() },
        AvailableToolchain { version: "3.5.8-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.8".parse().unwrap(), source_tar_gz: "Python-3.5.8rc1.tgz".into() },
        AvailableToolchain { version: "3.5.7".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.7".parse().unwrap(), source_tar_gz: "Python-3.5.7.tgz".into() },
        AvailableToolchain { version: "3.5.7-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.7".parse().unwrap(), source_tar_gz: "Python-3.5.7rc1.tgz".into() },
        AvailableToolchain { version: "3.5.6".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.6".parse().unwrap(), source_tar_gz: "Python-3.5.6.tgz".into() },
        AvailableToolchain { version: "3.5.6-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.6".parse().unwrap(), source_tar_gz: "Python-3.5.6rc1.tgz".into() },
        AvailableToolchain { version: "3.5.5".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.5".parse().unwrap(), source_tar_gz: "Python-3.5.5.tgz".into() },
        AvailableToolchain { version: "3.5.5-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.5".parse().unwrap(), source_tar_gz: "Python-3.5.5rc1.tgz".into() },
        AvailableToolchain { version: "3.5.4".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.4".parse().unwrap(), source_tar_gz: "Python-3.5.4.tgz".into() },
        AvailableToolchain { version: "3.5.4-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.4".parse().unwrap(), source_tar_gz: "Python-3.5.4rc1.tgz".into() },
        AvailableToolchain { version: "3.5.3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.3".parse().unwrap(), source_tar_gz: "Python-3.5.3.tgz".into() },
        AvailableToolchain { version: "3.5.3-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.3".parse().unwrap(), source_tar_gz: "Python-3.5.3rc1.tgz".into() },
        AvailableToolchain { version: "3.5.2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.2".parse().unwrap(), source_tar_gz: "Python-3.5.2.tgz".into() },
        AvailableToolchain { version: "3.5.2-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.2".parse().unwrap(), source_tar_gz: "Python-3.5.2rc1.tgz".into() },
        AvailableToolchain { version: "3.5.1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.1".parse().unwrap(), source_tar_gz: "Python-3.5.1.tgz".into() },
        AvailableToolchain { version: "3.5.1-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.1".parse().unwrap(), source_tar_gz: "Python-3.5.1rc1.tgz".into() },
        AvailableToolchain { version: "3.5.0".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.0".parse().unwrap(), source_tar_gz: "Python-3.5.0.tgz".into() },
        AvailableToolchain { version: "3.5.0-rc4".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.0".parse().unwrap(), source_tar_gz: "Python-3.5.0rc4.tgz".into() },
        AvailableToolchain { version: "3.5.0-rc3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.0".parse().unwrap(), source_tar_gz: "Python-3.5.0rc3.tgz".into() },
        AvailableToolchain { version: "3.5.0-rc2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.0".parse().unwrap(), source_tar_gz: "Python-3.5.0rc2.tgz".into() },
        AvailableToolchain { version: "3.5.0-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.0".parse().unwrap(), source_tar_gz: "Python-3.5.0rc1.tgz".into() },
        AvailableToolchain { version: "3.5.0-b4".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.0".parse().unwrap(), source_tar_gz: "Python-3.5.0b4.tgz".into() },
        AvailableToolchain { version: "3.5.0-b3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.0".parse().unwrap(), source_tar_gz: "Python-3.5.0b3.tgz".into() },
        AvailableToolchain { version: "3.5.0-b2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.0".parse().unwrap(), source_tar_gz: "Python-3.5.0b2.tgz".into() },
        AvailableToolchain { version: "3.5.0-b1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.0".parse().unwrap(), source_tar_gz: "Python-3.5.0b1.tgz".into() },
        AvailableToolchain { version: "3.5.0-a4".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.0".parse().unwrap(), source_tar_gz: "Python-3.5.0a4.tgz".into() },
        AvailableToolchain { version: "3.5.0-a3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.0".parse().unwrap(), source_tar_gz: "Python-3.5.0a3.tgz".into() },
        AvailableToolchain { version: "3.5.0-a2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.0".parse().unwrap(), source_tar_gz: "Python-3.5.0a2.tgz".into() },
        AvailableToolchain { version: "3.5.0-a1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.5.0".parse().unwrap(), source_tar_gz: "Python-3.5.0a1.tgz".into() },
        AvailableToolchain { version: "3.4.10".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.4.10".parse().unwrap(), source_tar_gz: "Python-3.4.10.tgz".into() },
        AvailableToolchain { version: "3.4.10-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.4.10".parse().unwrap(), source_tar_gz: "Python-3.4.10rc1.tgz".into() },
        AvailableToolchain { version: "3.4.9".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.4.9".parse().unwrap(), source_tar_gz: "Python-3.4.9.tgz".into() },
        AvailableToolchain { version: "3.4.9-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.4.9".parse().unwrap(), source_tar_gz: "Python-3.4.9rc1.tgz".into() },
        AvailableToolchain { version: "3.4.8".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.4.8".parse().unwrap(), source_tar_gz: "Python-3.4.8.tgz".into() },
        AvailableToolchain { version: "3.4.8-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.4.8".parse().unwrap(), source_tar_gz: "Python-3.4.8rc1.tgz".into() },
        AvailableToolchain { version: "3.4.7".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.4.7".parse().unwrap(), source_tar_gz: "Python-3.4.7.tgz".into() },
        AvailableToolchain { version: "3.4.7-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.4.7".parse().unwrap(), source_tar_gz: "Python-3.4.7rc1.tgz".into() },
        AvailableToolchain { version: "3.4.6".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.4.6".parse().unwrap(), source_tar_gz: "Python-3.4.6.tgz".into() },
        AvailableToolchain { version: "3.4.6-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.4.6".parse().unwrap(), source_tar_gz: "Python-3.4.6rc1.tgz".into() },
        AvailableToolchain { version: "3.4.5".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.4.5".parse().unwrap(), source_tar_gz: "Python-3.4.5.tgz".into() },
        AvailableToolchain { version: "3.4.5-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.4.5".parse().unwrap(), source_tar_gz: "Python-3.4.5rc1.tgz".into() },
        AvailableToolchain { version: "3.4.4".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.4.4".parse().unwrap(), source_tar_gz: "Python-3.4.4.tgz".into() },
        AvailableToolchain { version: "3.4.4-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.4.4".parse().unwrap(), source_tar_gz: "Python-3.4.4rc1.tgz".into() },
        AvailableToolchain { version: "3.4.3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.4.3".parse().unwrap(), source_tar_gz: "Python-3.4.3.tgz".into() },
        AvailableToolchain { version: "3.4.3-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.4.3".parse().unwrap(), source_tar_gz: "Python-3.4.3rc1.tgz".into() },
        AvailableToolchain { version: "3.4.2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.4.2".parse().unwrap(), source_tar_gz: "Python-3.4.2.tgz".into() },
        AvailableToolchain { version: "3.4.2-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.4.2".parse().unwrap(), source_tar_gz: "Python-3.4.2rc1.tgz".into() },
        AvailableToolchain { version: "3.4.1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.4.1".parse().unwrap(), source_tar_gz: "Python-3.4.1.tgz".into() },
        AvailableToolchain { version: "3.4.1-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.4.1".parse().unwrap(), source_tar_gz: "Python-3.4.1rc1.tgz".into() },
        AvailableToolchain { version: "3.4.0".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.4.0".parse().unwrap(), source_tar_gz: "Python-3.4.0.tgz".into() },
        AvailableToolchain { version: "3.4.0-rc3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.4.0".parse().unwrap(), source_tar_gz: "Python-3.4.0rc3.tgz".into() },
        AvailableToolchain { version: "3.3.7".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.3.7".parse().unwrap(), source_tar_gz: "Python-3.3.7.tgz".into() },
        AvailableToolchain { version: "3.3.7-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.3.7".parse().unwrap(), source_tar_gz: "Python-3.3.7rc1.tgz".into() },
        AvailableToolchain { version: "3.3.6".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.3.6".parse().unwrap(), source_tar_gz: "Python-3.3.6.tgz".into() },
        AvailableToolchain { version: "3.3.6-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.3.6".parse().unwrap(), source_tar_gz: "Python-3.3.6rc1.tgz".into() },
        AvailableToolchain { version: "3.3.5".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.3.5".parse().unwrap(), source_tar_gz: "Python-3.3.5.tgz".into() },
        AvailableToolchain { version: "3.3.5-rc2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.3.5".parse().unwrap(), source_tar_gz: "Python-3.3.5rc2.tgz".into() },
        AvailableToolchain { version: "3.3.5-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.3.5".parse().unwrap(), source_tar_gz: "Python-3.3.5rc1.tgz".into() },
        AvailableToolchain { version: "3.3.5-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.3.5".parse().unwrap(), source_tar_gz: "Python-3.3.5rc1.tgz".into() },
        AvailableToolchain { version: "3.3.4".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.3.4".parse().unwrap(), source_tar_gz: "Python-3.3.4.tgz".into() },
        AvailableToolchain { version: "3.3.3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.3.3".parse().unwrap(), source_tar_gz: "Python-3.3.3.tgz".into() },
        AvailableToolchain { version: "3.3.2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.3.2".parse().unwrap(), source_tar_gz: "Python-3.3.2.tgz".into() },
        AvailableToolchain { version: "3.3.1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.3.1".parse().unwrap(), source_tar_gz: "Python-3.3.1.tgz".into() },
        AvailableToolchain { version: "3.3.0".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.3.0".parse().unwrap(), source_tar_gz: "Python-3.3.0.tgz".into() },
        AvailableToolchain { version: "3.2.6".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.2.6".parse().unwrap(), source_tar_gz: "Python-3.2.6.tgz".into() },
        AvailableToolchain { version: "3.2.6-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.2.6".parse().unwrap(), source_tar_gz: "Python-3.2.6rc1.tgz".into() },
        AvailableToolchain { version: "3.2.5".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.2.5".parse().unwrap(), source_tar_gz: "Python-3.2.5.tgz".into() },
        AvailableToolchain { version: "3.2.4".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.2.4".parse().unwrap(), source_tar_gz: "Python-3.2.4.tgz".into() },
        AvailableToolchain { version: "3.2.3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.2.3".parse().unwrap(), source_tar_gz: "Python-3.2.3.tgz".into() },
        AvailableToolchain { version: "3.2.2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.2.2".parse().unwrap(), source_tar_gz: "Python-3.2.2.tgz".into() },
        AvailableToolchain { version: "3.2.1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.2.1".parse().unwrap(), source_tar_gz: "Python-3.2.1.tgz".into() },
        AvailableToolchain { version: "3.2.0".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.2".parse().unwrap(), source_tar_gz: "Python-3.2.tgz".into() },
        AvailableToolchain { version: "3.1.5".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.1.5".parse().unwrap(), source_tar_gz: "Python-3.1.5.tgz".into() },
        AvailableToolchain { version: "3.1.4".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.1.4".parse().unwrap(), source_tar_gz: "Python-3.1.4.tgz".into() },
        AvailableToolchain { version: "3.1.3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.1.3".parse().unwrap(), source_tar_gz: "Python-3.1.3.tgz".into() },
        AvailableToolchain { version: "3.1.2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.1.2".parse().unwrap(), source_tar_gz: "Python-3.1.2.tgz".into() },
        AvailableToolchain { version: "3.1.1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.1.1".parse().unwrap(), source_tar_gz: "Python-3.1.1.tgz".into() },
        AvailableToolchain { version: "3.1.0".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.1".parse().unwrap(), source_tar_gz: "Python-3.1.tgz".into() },
        AvailableToolchain { version: "3.0.1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.0.1".parse().unwrap(), source_tar_gz: "Python-3.0.1.tgz".into() },
        AvailableToolchain { version: "3.0.0".parse().unwrap(), base_url: "https://www.python.org/ftp/python/3.0".parse().unwrap(), source_tar_gz: "Python-3.0.tgz".into() },
        AvailableToolchain { version: "2.7.17".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.17".parse().unwrap(), source_tar_gz: "Python-2.7.17.tgz".into() },
        AvailableToolchain { version: "2.7.17-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.17".parse().unwrap(), source_tar_gz: "Python-2.7.17rc1.tgz".into() },
        AvailableToolchain { version: "2.7.16".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.16".parse().unwrap(), source_tar_gz: "Python-2.7.16.tgz".into() },
        AvailableToolchain { version: "2.7.16-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.16".parse().unwrap(), source_tar_gz: "Python-2.7.16rc1.tgz".into() },
        AvailableToolchain { version: "2.7.15".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.15".parse().unwrap(), source_tar_gz: "Python-2.7.15.tgz".into() },
        AvailableToolchain { version: "2.7.15-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.15".parse().unwrap(), source_tar_gz: "Python-2.7.15rc1.tgz".into() },
        AvailableToolchain { version: "2.7.14".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.14".parse().unwrap(), source_tar_gz: "Python-2.7.14.tgz".into() },
        AvailableToolchain { version: "2.7.14-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.14".parse().unwrap(), source_tar_gz: "Python-2.7.14rc1.tgz".into() },
        AvailableToolchain { version: "2.7.13".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.13".parse().unwrap(), source_tar_gz: "Python-2.7.13.tgz".into() },
        AvailableToolchain { version: "2.7.13-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.13".parse().unwrap(), source_tar_gz: "Python-2.7.13rc1.tgz".into() },
        AvailableToolchain { version: "2.7.12".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.12".parse().unwrap(), source_tar_gz: "Python-2.7.12.tgz".into() },
        AvailableToolchain { version: "2.7.12-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.12".parse().unwrap(), source_tar_gz: "Python-2.7.12rc1.tgz".into() },
        AvailableToolchain { version: "2.7.11".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.11".parse().unwrap(), source_tar_gz: "Python-2.7.11.tgz".into() },
        AvailableToolchain { version: "2.7.11-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.11".parse().unwrap(), source_tar_gz: "Python-2.7.11rc1.tgz".into() },
        AvailableToolchain { version: "2.7.10".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.10".parse().unwrap(), source_tar_gz: "Python-2.7.10.tgz".into() },
        AvailableToolchain { version: "2.7.10-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.10".parse().unwrap(), source_tar_gz: "Python-2.7.10rc1.tgz".into() },
        AvailableToolchain { version: "2.7.9".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.9".parse().unwrap(), source_tar_gz: "Python-2.7.9.tgz".into() },
        AvailableToolchain { version: "2.7.9-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.9".parse().unwrap(), source_tar_gz: "Python-2.7.9rc1.tgz".into() },
        AvailableToolchain { version: "2.7.8".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.8".parse().unwrap(), source_tar_gz: "Python-2.7.8.tgz".into() },
        AvailableToolchain { version: "2.7.7".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.7".parse().unwrap(), source_tar_gz: "Python-2.7.7.tgz".into() },
        AvailableToolchain { version: "2.7.7-rc1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.7".parse().unwrap(), source_tar_gz: "Python-2.7.7rc1.tgz".into() },
        AvailableToolchain { version: "2.7.6".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.6".parse().unwrap(), source_tar_gz: "Python-2.7.6.tgz".into() },
        AvailableToolchain { version: "2.7.5".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.5".parse().unwrap(), source_tar_gz: "Python-2.7.5.tgz".into() },
        AvailableToolchain { version: "2.7.4".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.4".parse().unwrap(), source_tar_gz: "Python-2.7.4.tgz".into() },
        AvailableToolchain { version: "2.7.3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.3".parse().unwrap(), source_tar_gz: "Python-2.7.3.tgz".into() },
        AvailableToolchain { version: "2.7.2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.2".parse().unwrap(), source_tar_gz: "Python-2.7.2.tgz".into() },
        AvailableToolchain { version: "2.7.1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7.1".parse().unwrap(), source_tar_gz: "Python-2.7.1.tgz".into() },
        AvailableToolchain { version: "2.7.0".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.7".parse().unwrap(), source_tar_gz: "Python-2.7.tgz".into() },
        AvailableToolchain { version: "2.6.9".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.6.9".parse().unwrap(), source_tar_gz: "Python-2.6.9.tgz".into() },
        AvailableToolchain { version: "2.6.8".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.6.8".parse().unwrap(), source_tar_gz: "Python-2.6.8.tgz".into() },
        AvailableToolchain { version: "2.6.7".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.6.7".parse().unwrap(), source_tar_gz: "Python-2.6.7.tgz".into() },
        AvailableToolchain { version: "2.6.6".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.6.6".parse().unwrap(), source_tar_gz: "Python-2.6.6.tgz".into() },
        AvailableToolchain { version: "2.6.5".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.6.5".parse().unwrap(), source_tar_gz: "Python-2.6.5.tgz".into() },
        AvailableToolchain { version: "2.6.4".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.6.4".parse().unwrap(), source_tar_gz: "Python-2.6.4.tgz".into() },
        AvailableToolchain { version: "2.6.3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.6.3".parse().unwrap(), source_tar_gz: "Python-2.6.3.tgz".into() },
        AvailableToolchain { version: "2.6.2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.6.2".parse().unwrap(), source_tar_gz: "Python-2.6.2.tgz".into() },
        AvailableToolchain { version: "2.6.1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.6.1".parse().unwrap(), source_tar_gz: "Python-2.6.1.tgz".into() },
        AvailableToolchain { version: "2.6.0".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.6".parse().unwrap(), source_tar_gz: "Python-2.6.tgz".into() },
        AvailableToolchain { version: "2.5.6".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.5.6".parse().unwrap(), source_tar_gz: "Python-2.5.6.tgz".into() },
        AvailableToolchain { version: "2.5.5".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.5.5".parse().unwrap(), source_tar_gz: "Python-2.5.5.tgz".into() },
        AvailableToolchain { version: "2.5.4".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.5.4".parse().unwrap(), source_tar_gz: "Python-2.5.4.tgz".into() },
        AvailableToolchain { version: "2.5.3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.5.3".parse().unwrap(), source_tar_gz: "Python-2.5.3.tgz".into() },
        AvailableToolchain { version: "2.5.2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.5.2".parse().unwrap(), source_tar_gz: "Python-2.5.2.tgz".into() },
        AvailableToolchain { version: "2.5.1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.5.1".parse().unwrap(), source_tar_gz: "Python-2.5.1.tgz".into() },
        AvailableToolchain { version: "2.5.0".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.5".parse().unwrap(), source_tar_gz: "Python-2.5.tgz".into() },
        AvailableToolchain { version: "2.4.6".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.4.6".parse().unwrap(), source_tar_gz: "Python-2.4.6.tgz".into() },
        AvailableToolchain { version: "2.4.5".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.4.5".parse().unwrap(), source_tar_gz: "Python-2.4.5.tgz".into() },
        AvailableToolchain { version: "2.4.4".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.4.4".parse().unwrap(), source_tar_gz: "Python-2.4.4.tgz".into() },
        AvailableToolchain { version: "2.4.3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.4.3".parse().unwrap(), source_tar_gz: "Python-2.4.3.tgz".into() },
        AvailableToolchain { version: "2.4.2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.4.2".parse().unwrap(), source_tar_gz: "Python-2.4.2.tgz".into() },
        AvailableToolchain { version: "2.4.1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.4.1".parse().unwrap(), source_tar_gz: "Python-2.4.1.tgz".into() },
        AvailableToolchain { version: "2.4.0".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.4".parse().unwrap(), source_tar_gz: "Python-2.4.tgz".into() },
        AvailableToolchain { version: "2.3.7".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.3.7".parse().unwrap(), source_tar_gz: "Python-2.3.7.tgz".into() },
        AvailableToolchain { version: "2.3.6".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.3.6".parse().unwrap(), source_tar_gz: "Python-2.3.6.tgz".into() },
        AvailableToolchain { version: "2.3.5".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.3.5".parse().unwrap(), source_tar_gz: "Python-2.3.5.tgz".into() },
        AvailableToolchain { version: "2.3.4".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.3.4".parse().unwrap(), source_tar_gz: "Python-2.3.4.tgz".into() },
        AvailableToolchain { version: "2.3.3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.3.3".parse().unwrap(), source_tar_gz: "Python-2.3.3.tgz".into() },
        AvailableToolchain { version: "2.3.2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.3.2".parse().unwrap(), source_tar_gz: "Python-2.3.2.tgz".into() },
        AvailableToolchain { version: "2.3.1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.3.1".parse().unwrap(), source_tar_gz: "Python-2.3.1.tgz".into() },
        AvailableToolchain { version: "2.3.0".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.3".parse().unwrap(), source_tar_gz: "Python-2.3.tgz".into() },
        AvailableToolchain { version: "2.2.3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.2.3".parse().unwrap(), source_tar_gz: "Python-2.2.3.tgz".into() },
        AvailableToolchain { version: "2.2.2".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.2.2".parse().unwrap(), source_tar_gz: "Python-2.2.2.tgz".into() },
        AvailableToolchain { version: "2.2.1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.2.1".parse().unwrap(), source_tar_gz: "Python-2.2.1.tgz".into() },
        AvailableToolchain { version: "2.2.0".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.2".parse().unwrap(), source_tar_gz: "Python-2.2.tgz".into() },
        AvailableToolchain { version: "2.1.3".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.1.3".parse().unwrap(), source_tar_gz: "Python-2.1.3.tgz".into() },
        AvailableToolchain { version: "2.0.1".parse().unwrap(), base_url: "https://www.python.org/ftp/python/2.0.1".parse().unwrap(), source_tar_gz: "Python-2.0.1.tgz".into() },
    ];
    assert_eq!(parsed, expected);
}
