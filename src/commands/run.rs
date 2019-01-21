use failure::format_err;

use crate::{config::Cfg, settings::Settings, shim, utils, Result};

pub fn run(cfg: &Option<Cfg>, settings: &Settings, command_and_args: &str) -> Result<()> {
    let s = shlex::split(&command_and_args)
        .ok_or_else(|| format_err!("Failed to split command from {:?}", command_and_args))?;
    let (cmd, arguments) = s.split_at(1);
    let cmd = cmd
        .get(0)
        .ok_or_else(|| format_err!("Failed to extract command from {:?}", command_and_args))?;

    let interpreter_to_use = utils::get_interpreter_to_use(cfg, settings)?;

    shim::run(&interpreter_to_use, cmd, arguments)
}
