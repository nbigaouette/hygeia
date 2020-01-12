use crate::{toolchain::installed::InstalledToolchain, Result};

#[cfg_attr(not(windows), allow(dead_code))]
pub fn command_with_major_version(
    command: &str,
    interpreter_to_use: &InstalledToolchain,
) -> Result<String> {
    let (command, extension) = if command.ends_with(".exe") {
        (command.trim_end_matches(".exe"), ".exe")
    } else {
        (command, "")
    };

    let mut command_string_with_major_version =
        super::unix::command_with_major_version(command, interpreter_to_use)?;
    command_string_with_major_version.push_str(extension);

    Ok(command_string_with_major_version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    use semver::Version;

    #[test]
    fn append_version_to_command_no_extension_success() {
        let interpreter = InstalledToolchain {
            location: Path::new("/usr/bin").into(),
            version: Version::parse("3.7.3").unwrap(),
        };
        let cmd = command_with_major_version("python", &interpreter).unwrap();
        assert_eq!(cmd, "python3");
    }

    #[test]
    fn append_version_to_command_exe_success() {
        let interpreter = InstalledToolchain {
            location: Path::new("/usr/bin").into(),
            version: Version::parse("3.7.3").unwrap(),
        };
        let cmd = command_with_major_version("python.exe", &interpreter).unwrap();
        assert_eq!(cmd, "python3.exe");
    }
}
