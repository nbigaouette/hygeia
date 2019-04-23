use semver::Version;

use crate::{settings::PythonVersion, utils, Result};

pub fn build_filename_exe(version: &Version) -> Result<String> {
    Ok(format!(
        "{}-amd64.exe",
        utils::build_basename(version)?.replace("Python", "python")
    ))
}

pub fn command_with_major_version(
    command: &str,
    _interpreter_to_use: &PythonVersion,
) -> Result<String> {
    let command_string_with_major_version = {
        // Not implemented yet due to Windows using a `.exe` extension
        log::error!("Adding the major Python version to binary not implemented on Windows");
        command.to_string()
    };

    Ok(command_string_with_major_version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

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

    #[test]
    fn append_version_to_command_success() {
        let interpreter = PythonVersion {
            location: Path::new("/usr/bin").into(),
            version: Version::parse("3.7.3").unwrap(),
        };
        let cmd = command_with_major_version("python", &interpreter).unwrap();
        assert_eq!(cmd, "python3");
    }
}
