use std::{env, ffi::OsString};

use anyhow::{Context, Result};
use regex::Regex;
use semver::VersionReq;
use subprocess::{self, Exec, Redirection};
use thiserror::Error;

use crate::{
    dir_monitor::DirectoryMonitor,
    os::command_with_major_version,
    toolchain::{installed::InstalledToolchain, CompatibleToolchainBuilder},
    utils, EXECUTABLE_NAME,
};

#[derive(Debug, Error)]
pub enum ShimError {
    #[error("No interpreter found to run command: {0:?}")]
    MissingInterpreter(String),
    #[error("subprocess::PopenError: {0:?}")]
    PopenError(subprocess::PopenError),
}

pub fn run<S>(command: &str, arguments: &[S]) -> Result<()>
where
    S: AsRef<str> + std::convert::AsRef<std::ffi::OsStr> + std::fmt::Debug,
{
    // Try to detect if a command with a version appended is run, for example 'python2.7'
    // or 'python3'.
    let command_version = extract_major_version_from_executable_name(command);

    let compatible_toolchain = CompatibleToolchainBuilder::new()
        .load_from_file()
        .overwrite(command_version)
        .pick_latest_if_none_found()
        .compatible_version()?;

    match compatible_toolchain {
        Some(compatible_toolchain) => run_with(&compatible_toolchain, command, arguments),
        None => {
            log::error!("No Python interpreter found at all. Please install at least one!");
            Err(ShimError::MissingInterpreter(command.to_string()).into())
        }
    }
}

pub fn run_with<S>(toolchain: &InstalledToolchain, command: &str, arguments: &[S]) -> Result<()>
where
    S: AsRef<str> + std::convert::AsRef<std::ffi::OsStr> + std::fmt::Debug,
{
    log::debug!("toolchain: {:?}", toolchain);

    let command_string_with_major_version = command_with_major_version(command, toolchain)?;

    let command_full_path = toolchain.location.join(command_string_with_major_version);
    let command_full_path = if command_full_path.exists() {
        command_full_path
    } else {
        toolchain.location.join(command)
    };

    log::debug!("Command:   {:?}", command_full_path);
    log::debug!("Arguments: {:?}", arguments);

    let bin_dir = toolchain.location.clone();

    // Prepend `bin_dir` to `PATH`
    let new_path = match env::var("PATH") {
        Ok(path) => {
            let mut paths = env::split_paths(&path).collect::<Vec<_>>();
            paths.push(bin_dir.clone());
            env::join_paths(paths)?
        }
        Err(err) => {
            log::error!("Failed to get environment variable PATH: {:?}", err);
            OsString::new()
        }
    };

    let mut bin_dir_monitor = DirectoryMonitor::new(&bin_dir)?;

    Exec::cmd(&command_full_path)
        .args(arguments)
        .env("PATH", new_path)
        .stdout(Redirection::None)
        .stderr(Redirection::None)
        .join()
        .map_err(ShimError::PopenError)
        .with_context(|| {
            format!(
                "Failed command: {} {}",
                command_full_path.display(),
                arguments
                    .iter()
                    .map(|s| s.as_ref())
                    .collect::<Vec<&str>>()
                    .join(" ")
            )
        })?;

    let new_bin_files: Vec<_> = bin_dir_monitor.check()?.collect();

    // Create a hard-link for the new bins
    let shim_dir = utils::directory::shims()?;
    let executable_path = shim_dir.join(EXECUTABLE_NAME);
    for new_bin_file_path in new_bin_files {
        match new_bin_file_path.file_name() {
            Some(new_bin_filename) => {
                let new_bin_path = shim_dir.join(new_bin_filename);
                utils::create_hard_link(&executable_path, new_bin_path)?;
            }
            None => {
                log::error!("Cannot get path's filename part: {:?}", new_bin_file_path);
            }
        }
    }

    Ok(())
}

fn extract_major_version_from_executable_name(exe: &str) -> Option<VersionReq> {
    // See https://regex101.com/r/gn7Goy/1
    let re = Regex::new(r#"(?x)[^0-9](?P<major>2|3)\b(?P<minor>\.\d+)?"#)
        .expect("Regex is expected to be valid");
    match re.captures(exe) {
        None => None,
        Some(caps) => {
            // Allocate just enough for the longest expected Python version: '3.99'
            let mut v = String::with_capacity(4);
            // If there's a match, the major version should at least be present, and thus
            // safe to unwrap.
            v.push_str(caps.get(1).unwrap().as_str());
            // Did we found a minor version?
            if let Some(minor) = caps.get(2) {
                // NOTE: The match contains the dot
                v.push_str(minor.as_str());
            }

            VersionReq::parse(&v).ok()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_major_version_from_executable_name_python() {
        assert_eq!(extract_major_version_from_executable_name("python"), None);
    }

    #[test]
    fn extract_major_version_from_executable_name_python2() {
        assert_eq!(
            extract_major_version_from_executable_name("python2"),
            Some(VersionReq::parse("2").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python3() {
        assert_eq!(
            extract_major_version_from_executable_name("python3"),
            Some(VersionReq::parse("3").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python22() {
        assert_eq!(
            extract_major_version_from_executable_name("python2.2"),
            Some(VersionReq::parse("2.2").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python23() {
        assert_eq!(
            extract_major_version_from_executable_name("python2.3"),
            Some(VersionReq::parse("2.3").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python27() {
        assert_eq!(
            extract_major_version_from_executable_name("python2.7"),
            Some(VersionReq::parse("2.7").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python32() {
        assert_eq!(
            extract_major_version_from_executable_name("python3.2"),
            Some(VersionReq::parse("3.2").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python33() {
        assert_eq!(
            extract_major_version_from_executable_name("python3.3"),
            Some(VersionReq::parse("3.3").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python37() {
        assert_eq!(
            extract_major_version_from_executable_name("python3.7"),
            Some(VersionReq::parse("3.7").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python2m() {
        assert_eq!(
            extract_major_version_from_executable_name("python2.2m"),
            Some(VersionReq::parse("2.2").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python23m() {
        assert_eq!(
            extract_major_version_from_executable_name("python2.3m"),
            Some(VersionReq::parse("2.3").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python27m() {
        assert_eq!(
            extract_major_version_from_executable_name("python2.7m"),
            Some(VersionReq::parse("2.7").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python32m() {
        assert_eq!(
            extract_major_version_from_executable_name("python3.2m"),
            Some(VersionReq::parse("3.2").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python33m() {
        assert_eq!(
            extract_major_version_from_executable_name("python3.3m"),
            Some(VersionReq::parse("3.3").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python37m() {
        assert_eq!(
            extract_major_version_from_executable_name("python3.7m"),
            Some(VersionReq::parse("3.7").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_pythonw() {
        assert_eq!(extract_major_version_from_executable_name("pythonw"), None,);
    }

    #[test]
    fn extract_major_version_from_executable_name_pythonw22() {
        assert_eq!(
            extract_major_version_from_executable_name("pythonw2.2"),
            Some(VersionReq::parse("2.2").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_pythonw23() {
        assert_eq!(
            extract_major_version_from_executable_name("pythonw2.3"),
            Some(VersionReq::parse("2.3").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_pythonw27() {
        assert_eq!(
            extract_major_version_from_executable_name("pythonw2.7"),
            Some(VersionReq::parse("2.7").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_pythonw32() {
        assert_eq!(
            extract_major_version_from_executable_name("pythonw3.2"),
            Some(VersionReq::parse("3.2").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_pythonw33() {
        assert_eq!(
            extract_major_version_from_executable_name("pythonw3.3"),
            Some(VersionReq::parse("3.3").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_pythonw37() {
        assert_eq!(
            extract_major_version_from_executable_name("pythonw3.7"),
            Some(VersionReq::parse("3.7").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python_build() {
        assert_eq!(
            extract_major_version_from_executable_name("python-build"),
            None
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python_config() {
        assert_eq!(
            extract_major_version_from_executable_name("python-config"),
            None
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python2_config() {
        assert_eq!(
            extract_major_version_from_executable_name("python2-config"),
            Some(VersionReq::parse("2").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python3_config() {
        assert_eq!(
            extract_major_version_from_executable_name("python3-config"),
            Some(VersionReq::parse("3").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python22_config() {
        assert_eq!(
            extract_major_version_from_executable_name("python2.2-config"),
            Some(VersionReq::parse("2.2").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python3_config2() {
        assert_eq!(
            extract_major_version_from_executable_name("python2.3-config"),
            Some(VersionReq::parse("2.3").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python27_config() {
        assert_eq!(
            extract_major_version_from_executable_name("python2.7-config"),
            Some(VersionReq::parse("2.7").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python22m_config() {
        assert_eq!(
            extract_major_version_from_executable_name("python2.2m-config"),
            Some(VersionReq::parse("2.2").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python23m_config() {
        assert_eq!(
            extract_major_version_from_executable_name("python2.3m-config"),
            Some(VersionReq::parse("2.3").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python27m_config() {
        assert_eq!(
            extract_major_version_from_executable_name("python2.7m-config"),
            Some(VersionReq::parse("2.7").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python32mm_config() {
        assert_eq!(
            extract_major_version_from_executable_name("python3.2m-config"),
            Some(VersionReq::parse("3.2").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python33m_config() {
        assert_eq!(
            extract_major_version_from_executable_name("python3.3m-config"),
            Some(VersionReq::parse("3.3").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_python37m_config() {
        assert_eq!(
            extract_major_version_from_executable_name("python3.7m-config"),
            Some(VersionReq::parse("3.7").unwrap())
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_random_middle_23232() {
        assert_eq!(
            extract_major_version_from_executable_name("prandom23232odfsif"),
            None
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_random_end2() {
        assert_eq!(
            extract_major_version_from_executable_name("prandom23232"),
            None
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_random_end3() {
        assert_eq!(
            extract_major_version_from_executable_name("prandom232323"),
            None
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_random_middle_256757() {
        assert_eq!(
            extract_major_version_from_executable_name("prandom256757odfsif"),
            None
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_random_middle_672575() {
        assert_eq!(
            extract_major_version_from_executable_name("prandom672575odfsif"),
            None
        );
    }

    #[test]
    fn extract_major_version_from_executable_name_random_2() {
        assert_eq!(
            extract_major_version_from_executable_name("prandom2odfsif"),
            None
        );
    }
}
