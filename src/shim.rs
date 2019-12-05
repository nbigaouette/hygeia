use std::{env, path::PathBuf};

use anyhow::Result;
use regex::Regex;
use semver::VersionReq;
use thiserror::Error;

use crate::{
    dir_monitor::DirectoryMonitor,
    os,
    toolchain::{installed::InstalledToolchain, CompatibleToolchainBuilder},
    utils, EXECUTABLE_NAME,
};

#[derive(Debug, Error)]
pub enum ShimError {
    #[error("No interpreter found to run command: {0:?}")]
    MissingInterpreter(String),
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
    let command_string_with_major_version = os::command_with_major_version(command, toolchain)?;

    let bin_dir = toolchain.location.clone();

    let current_paths: Vec<PathBuf> = match env::var("PATH") {
        Ok(path) => env::split_paths(&path).collect(),
        Err(err) => {
            log::error!("Failed to get environment variable PATH: {:?}", err);
            vec![PathBuf::new()]
        }
    };
    let new_paths: Vec<PathBuf> = {
        let mut tmp = os::paths_to_prepends(&toolchain.version)?;
        tmp.extend_from_slice(&current_paths);
        // Delete the shims path from the list
        // This should prevent calling our shims by accident.
        let shims_dir = utils::directory::shims()?;
        tmp.retain(|x| *x != shims_dir);
        tmp
    };
    let new_path = env::join_paths(new_paths.iter())?;

    log::debug!("Toolchain: {}", toolchain);
    log::debug!("Command:   {:?}", command_string_with_major_version);
    log::debug!("Arguments: {:?}", arguments);
    log::debug!("Path:      {}", new_path.to_string_lossy());

    let mut bin_dir_monitor = DirectoryMonitor::new(&bin_dir)?;

    let status = std::process::Command::new(&command_string_with_major_version)
        .args(arguments)
        // Replace it with our update
        .env("PATH", &new_path)
        .status()?;

    let new_bin_files: Vec<_> = bin_dir_monitor.check()?.collect();

    // Create a hard-link for the new bins
    let shim_dir = utils::directory::shims()?;
    let executable_path = shim_dir.join(EXECUTABLE_NAME);
    for new_bin_file_path in new_bin_files {
        match new_bin_file_path.file_name() {
            Some(new_bin_filename) => {
                log::debug!("Creating a hardlink for {:?}", new_bin_file_path);
                let new_bin_path = shim_dir.join(new_bin_filename);
                utils::create_hard_link(&executable_path, new_bin_path)?;
            }
            None => {
                log::error!("Cannot get path's filename part: {:?}", new_bin_file_path);
            }
        }
    }

    if status.success() {
        log::debug!("Success!");
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Failed to execute command (exit code: {:?}): {} {}\nPATH: \"{}\"",
            status.code(),
            command_string_with_major_version,
            arguments
                .iter()
                .map(|s| s.as_ref())
                .collect::<Vec<&str>>()
                .join(" "),
            new_path.to_string_lossy()
        ))
    }
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
