use crate::{
    toolchain::CompatibleToolchainBuilder, utils::directory::PycorsPathsProviderFromEnv, Result,
};

pub fn run(version: Option<String>) -> Result<()> {
    let compatible_toolchain_builder = match version {
        Some(version) => CompatibleToolchainBuilder::new().load_from_string(&version),
        None => CompatibleToolchainBuilder::new().load_from_file(),
    };
    let compatible_toolchain = compatible_toolchain_builder
        .pick_latest_if_none_found()
        .compatible_version(PycorsPathsProviderFromEnv::new())?;

    match compatible_toolchain {
        Some(compatible_toolchain) => println!("{}", compatible_toolchain.location.display()),
        None => {
            log::error!("No Python interpreter found at all. Please install at least one!");
            println!()
        }
    }

    Ok(())
}
