use crate::{selected::SelectedVersion, settings::Settings, utils, Result};

pub fn run(selected_version: &Option<SelectedVersion>, settings: &Settings) -> Result<()> {
    let interpreter_to_use = utils::get_interpreter_to_use(selected_version, settings)?;
    println!("{}", interpreter_to_use.version);
    Ok(())
}
