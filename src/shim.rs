use std::env;

use failure::format_err;
use log::debug;
use shlex;
use subprocess::{Exec, Redirection};

use crate::config::Cfg;
use crate::pycors::active_version;
use crate::settings::Settings;
use crate::Result;

pub fn python_shim(cfg: &Option<Cfg>, settings: &Settings, arguments: &[String]) -> Result<()> {
    run(cfg, settings, "python", arguments)
}

pub fn run_command(cfg: &Option<Cfg>, settings: &Settings, command_and_args: &str) -> Result<()> {
    let s = shlex::split(&command_and_args)
        .ok_or_else(|| format_err!("Failed to split command from {:?}", command_and_args))?;
    let (cmd, arguments) = s.split_at(1);
    let cmd = cmd
        .iter()
        .nth(0)
        .ok_or_else(|| format_err!("Failed to extract command from {:?}", command_and_args))?;

    run(cfg, settings, cmd, arguments)
}

fn run<S>(cfg: &Option<Cfg>, settings: &Settings, command: &str, arguments: &[S]) -> Result<()>
where
    S: AsRef<str> + std::convert::AsRef<std::ffi::OsStr> + std::fmt::Debug,
{
    let arguments = arguments.as_ref();

    let cfg = cfg
        .as_ref()
        .ok_or_else(|| format_err!("No Python runtime configured. Use `pycors use <version>`."))?;

    let active_python = active_version(&cfg.version, settings)
        .ok_or_else(|| format_err!("No active Python runtime found."))?;

    debug!("active_python: {:?}", active_python);

    let bin_path = active_python.location.join("bin");

    let path_env = env::var("PATH")?;
    if path_env.is_empty() {
        env::set_var("PATH", &bin_path);
    } else {
        env::set_var("PATH", format!("{}:{}", bin_path.display(), path_env));
    }

    debug!("Command: {:?}   Arguments: {:?}", command, arguments);

    Exec::cmd(&command)
        .args(arguments)
        .stdout(Redirection::None)
        .stderr(Redirection::None)
        .join()?;

    Ok(())
}

pub fn setup_shim() -> Result<()> {
    debug!("Setting up the shim...");

    Ok(())
}
