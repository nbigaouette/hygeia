use failure::format_err;
use shlex;
use subprocess::{Exec, Redirection};

use crate::config::Cfg;
use crate::settings::Settings;
use crate::utils;
use crate::Result;

pub fn python_shim(
    command: &str,
    cfg: &Option<Cfg>,
    settings: &Settings,
    arguments: &[String],
) -> Result<()> {
    run(cfg, settings, command, arguments)
}

pub fn run_command(cfg: &Option<Cfg>, settings: &Settings, command_and_args: &str) -> Result<()> {
    let s = shlex::split(&command_and_args)
        .ok_or_else(|| format_err!("Failed to split command from {:?}", command_and_args))?;
    let (cmd, arguments) = s.split_at(1);
    let cmd = cmd
        .get(0)
        .ok_or_else(|| format_err!("Failed to extract command from {:?}", command_and_args))?;

    run(cfg, settings, cmd, arguments)
}

fn run<S>(cfg: &Option<Cfg>, settings: &Settings, command: &str, arguments: &[S]) -> Result<()>
where
    S: AsRef<str> + std::convert::AsRef<std::ffi::OsStr> + std::fmt::Debug,
{
    let interpreter_to_use = utils::get_interpreter_to_use(cfg, settings)?;

    log::debug!("interpreter_to_use: {:?}", interpreter_to_use);

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

    Exec::cmd(&command_full_path)
        .args(arguments)
        .stdout(Redirection::None)
        .stderr(Redirection::None)
        .join()?;

    Ok(())
}
