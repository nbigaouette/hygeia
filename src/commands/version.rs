use crate::{config::Cfg, settings::Settings, utils, Result};

pub fn print_active_interpreter_version(cfg: &Option<Cfg>, settings: &Settings) -> Result<()> {
    let interpreter_to_use = utils::get_interpreter_to_use(cfg, settings)?;
    println!("{}", interpreter_to_use.version);
    Ok(())
}