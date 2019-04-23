use semver::Version;

use crate::{utils, Result};

pub fn build_filename_exe(version: &Version) -> Result<String> {
    Ok(format!(
        "{}-amd64.exe",
        utils::build_basename(version)?.replace("Python", "python")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_filename_from_version_372() {
        let version = Version::parse("3.7.2").unwrap();

        let filename_exe = build_filename_exe(&version).unwrap();
        assert_eq!(&filename_exe, "python-3.7.2-amd64.exe");
    }

    #[test]
    fn build_filename_from_version_372rc1() {
        let version = Version::parse("3.7.2-rc1").unwrap();

        let filename_exe = build_filename_exe(&version).unwrap();
        assert_eq!(&filename_exe, "python-3.7.2rc1-amd64.exe");
    }
}
