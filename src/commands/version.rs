use crate::{selected::SelectedVersion, settings::InstalledToolchain, utils, Result};

pub fn run(
    selected_version: &Option<SelectedVersion>,
    installed_toolchains: &[InstalledToolchain],
) -> Result<()> {
    let interpreter_to_use = utils::get_interpreter_to_use(selected_version, installed_toolchains)?;
    println!("{}", interpreter_to_use.version);
    Ok(())
}
