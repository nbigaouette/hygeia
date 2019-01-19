use crate::{config::Cfg, settings::Settings, utils, Result};

pub fn run(cfg: &Option<Cfg>, settings: &Settings) -> Result<()> {
    let interpreter_to_use = utils::get_interpreter_to_use(cfg, settings)?;
    println!("{}", interpreter_to_use.location.display());
    Ok(())
}
