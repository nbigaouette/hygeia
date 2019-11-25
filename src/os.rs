use semver::Version;

use crate::{toolchain::installed::InstalledToolchain, Result};

pub mod unix;
pub mod windows;

pub fn build_filename(version: &Version) -> Result<String> {
    #[cfg(not(target_os = "windows"))]
    {
        unix::build_filename_tgz(version)
    }
    #[cfg(target_os = "windows")]
    {
        windows::build_filename_exe(version)
    }
}

pub fn command_with_major_version(
    command: &str,
    interpreter_to_use: &InstalledToolchain,
) -> Result<String> {
    #[cfg(not(target_os = "windows"))]
    {
        unix::command_with_major_version(command, interpreter_to_use)
    }
    #[cfg(target_os = "windows")]
    {
        windows::command_with_major_version(command, interpreter_to_use)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn build_filename_from_version_372() {
        let version = Version::parse("3.7.2").unwrap();
        let filename = build_filename(&version).unwrap();
        assert!(filename == "Python-3.7.2.tgz" || filename == "python-3.7.2-amd64.exe");
    }

    #[test]
    fn build_filename_from_version_372rc1() {
        let version = Version::parse("3.7.2-rc1").unwrap();
        let filename = build_filename(&version).unwrap();
        assert!(filename == "Python-3.7.2rc1.tgz" || filename == "python-3.7.2rc1-amd64.exe");
    }

    #[test]
    fn append_version_to_command_success() {
        let interpreter = InstalledToolchain {
            location: Path::new("/usr/bin").into(),
            version: Version::parse("3.7.3").unwrap(),
        };
        let cmd = command_with_major_version("python", &interpreter).unwrap();
        assert_eq!(cmd, "python3");
    }
}
