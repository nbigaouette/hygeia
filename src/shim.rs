use std::{env, ffi::OsString};

use subprocess::{Exec, Redirection};

use crate::{
    dir_monitor::DirectoryMonitor, installed::InstalledToolchain, os::command_with_major_version,
    utils, Result, EXECUTABLE_NAME,
};

pub fn run<S>(interpreter_to_use: &InstalledToolchain, command: &str, arguments: &[S]) -> Result<()>
where
    S: AsRef<str> + std::convert::AsRef<std::ffi::OsStr> + std::fmt::Debug,
{
    log::debug!("interpreter_to_use: {:?}", interpreter_to_use);

    let command_string_with_major_version =
        command_with_major_version(command, interpreter_to_use)?;

    let command_full_path = interpreter_to_use
        .location
        .join(command_string_with_major_version);
    let command_full_path = if command_full_path.exists() {
        command_full_path
    } else {
        interpreter_to_use.location.join(command)
    };

    log::debug!("Command:   {:?}", command_full_path);
    log::debug!("Arguments: {:?}", arguments);

    let bin_dir = interpreter_to_use.location.clone();

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
        .join()?;

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
