use failure::format_err;
use semver::Version;

use crate::{toolchain::installed::InstalledToolchain, utils, Result};

#[cfg_attr(windows, allow(dead_code))]
pub fn build_filename_tgz(version: &Version) -> Result<String> {
    Ok(format!("{}.tgz", utils::build_basename(version)?))
}

#[cfg_attr(windows, allow(dead_code))]
pub fn command_with_major_version(
    command: &str,
    interpreter_to_use: &InstalledToolchain,
) -> Result<String> {
    // NOTE: Make sure the command given by the user contains the major Python version
    //       appended. This should prevent having a Python 3 interpreter in `.python-version`
    //       but being called `python` by the user, ending up executing, say, /usr/local/bin/python`
    //       which is itself a Python 2 interpreter.
    let last_command_char = format!(
        "{}",
        command
            .chars()
            .last()
            .ok_or_else(|| format_err!("Cannot get last character from command {:?}", command))?
    );

    let command_string_with_major_version = {
        if last_command_char == "2" || last_command_char == "3" {
            command.to_string()
        } else {
            log::debug!(
                "Appending Python interpreter major version {} to command.",
                interpreter_to_use.version.major
            );
            format!("{}{}", command, interpreter_to_use.version.major)
        }
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

        let filename_tgz = build_filename_tgz(&version).unwrap();
        assert_eq!(&filename_tgz, "Python-3.7.2.tgz");
    }

    #[test]
    fn build_filename_from_version_372rc1() {
        let version = Version::parse("3.7.2-rc1").unwrap();

        let filename_tgz = build_filename_tgz(&version).unwrap();
        assert_eq!(&filename_tgz, "Python-3.7.2rc1.tgz");
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
