use semver::Version;

use crate::{utils, Result};

pub fn build_filename_tgz(version: &Version) -> Result<String> {
    Ok(format!("{}.tgz", utils::build_basename(version)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_filename_from_version_372() {
        let version = Version::parse("3.7.2").unwrap();

        let filename_tgz = build_filename_tgz(&version).unwrap();
        assert_eq!(&filename_tgz, "Python-3.7.2.tgz");
    }

    #[test]
    fn build_filename_from_version_372rc1() {
        let version = Version::parse("3.7.2-rc1").unwrap();

        let filename_tgz = build_filename_tgz(&version).unwrap();
        assert_eq!(&filename_tgz, "Python-3.7.2rc1.tgz");
    }
}
