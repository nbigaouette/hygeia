use std::{env, ffi::OsString};

use failure::format_err;
use subprocess::{Exec, Redirection};

use crate::{settings::PythonVersion, utils, Result};

pub fn run<S>(interpreter_to_use: &PythonVersion, command: &str, arguments: &[S]) -> Result<()>
where
    S: AsRef<str> + std::convert::AsRef<std::ffi::OsStr> + std::fmt::Debug,
{
    log::debug!("interpreter_to_use: {:?}", interpreter_to_use);

    let install_dir = utils::install_dir(&interpreter_to_use.version)?;
    let bin_dir = install_dir.join("bin");

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

    // Prepend `bin_dir` to `PATH`
    let new_path = match env::var("PATH") {
        Ok(path) => {
            let mut paths = env::split_paths(&path).collect::<Vec<_>>();
            paths.push(bin_dir);
            env::join_paths(paths)?
        }
        Err(err) => {
            log::error!("Failed to get environment variable PATH: {:?}", err);
            OsString::new()
        }
    };

    Exec::cmd(&command_full_path)
        .args(arguments)
        .env("PATH", new_path)
        .stdout(Redirection::None)
        .stderr(Redirection::None)
        .join()?;

    Ok(())
}
