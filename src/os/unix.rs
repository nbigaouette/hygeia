use anyhow::{anyhow, Result};

use crate::toolchain::installed::InstalledToolchain;

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
            .ok_or_else(|| anyhow!("Cannot get last character from command {:?}", command))?
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

    use semver::Version;

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
