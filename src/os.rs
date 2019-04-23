use failure::format_err;
use semver::Version;

use crate::{settings::PythonVersion, Result};

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
    interpreter_to_use: &PythonVersion,
) -> Result<String> {
    // NOTE: Make sure the command given by the user contains the major Python version
    //       appended. This should prevent having a Python 3 interpreter in `.python-version`
    //       but being called `python` by the user, ending up executing, say, /usr/local/bin/python`
    //       which is itself a Python 2 interpreter.
    #[allow(unused_variables)]
    let last_command_char = format!(
        "{}",
        command
            .chars()
            .last()
            .ok_or_else(|| format_err!("Cannot get last character from command {:?}", command))?
    );

    let command_string_with_major_version = {
        #[cfg(target_os = "windows")]
        {
            // Not implemented yet due to Windows using a `.exe` extension
            log::error!("Adding the major Python version to binary not implemented on Windows");
            command.to_string()
        }
        #[cfg(not(target_os = "windows"))]
        {
            if last_command_char == "2" || last_command_char == "3" {
                command.to_string()
            } else {
                log::debug!(
                    "Appending Python interpreter major version {} to command.",
                    interpreter_to_use.version.major
                );
                format!("{}{}", command, interpreter_to_use.version.major)
            }
        }
    };

    Ok(command_string_with_major_version)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
