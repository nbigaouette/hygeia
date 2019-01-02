use std::env;

use failure::format_err;
use log::debug;
use shlex;
use subprocess::{Exec, Redirection};

use crate::config::Cfg;
use crate::pycors::active_version;
use crate::settings::Settings;
use crate::Result;

pub fn python_shim() -> Result<()> {
    unimplemented!()
}

pub fn run(cfg: &Option<Cfg>, settings: &Settings, commands: &str) -> Result<()> {
    let cfg = cfg
        .as_ref()
        .ok_or_else(|| format_err!("No Python runtime configured. Use `pycors use <version>`."))?;

    let active_python = active_version(&cfg.version, settings)
        .ok_or_else(|| format_err!("No active Python runtime found."))?;

    debug!("active_python: {:?}", active_python);

    let path_env = env::var("PATH")?;
    if path_env.is_empty() {
        env::set_var("PATH", &active_python.location);
    } else {
        env::set_var(
            "PATH",
            format!("{}:{}", active_python.location.display(), path_env),
        );
    }

    let s = shlex::split(&commands)
        .ok_or_else(|| format_err!("Failed to split command from {:?}", commands))?;
    let (cmd, arguments) = s.split_at(1);
    let cmd = cmd
        .iter()
        .nth(0)
        .ok_or_else(|| format_err!("Failed to extract command from {:?}", commands))?;

    debug!("Command: {:?}   Arguments: {:?}", cmd, arguments);

    Exec::cmd(&cmd)
        .args(arguments)
        .stdout(Redirection::None)
        .stderr(Redirection::None)
        .join()?;

    Ok(())
}
