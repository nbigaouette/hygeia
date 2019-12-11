use std::path::PathBuf;

use semver::Version;

use crate::{utils::directory::PycorsPathsProviderFromEnv, Result};

pub mod unix;
pub mod windows;

pub fn build_filename(version: &Version) -> Result<String> {
    #[cfg(not(target_os = "windows"))]
    {
        unix::build_filename_tgz(version)
    }
    #[cfg(target_os = "windows")]
    {
        windows::build_filename_zip(version)
    }
}

pub fn paths_to_prepends(version: &Version) -> Result<Vec<PathBuf>> {
    let bin_dir = PycorsPathsProviderFromEnv::new().bin_dir(version);

    let mut paths = Vec::new();

    paths.push(bin_dir.clone());

    #[cfg(windows)]
    {
        paths.push(bin_dir.join("Scripts"));
    }

    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_filename_from_version_372() {
        let version = Version::parse("3.7.2").unwrap();
        let filename = build_filename(&version).unwrap();
        assert!(filename == "Python-3.7.2.tgz" || filename == "python-3.7.2-embed-amd64.zip");
    }

    #[test]
    fn build_filename_from_version_372rc1() {
        let version = Version::parse("3.7.2-rc1").unwrap();
        let filename = build_filename(&version).unwrap();
        assert!(filename == "Python-3.7.2rc1.tgz" || filename == "python-3.7.2rc1-embed-amd64.zip");
    }
}
