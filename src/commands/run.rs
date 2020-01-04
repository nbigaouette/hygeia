use anyhow::{anyhow, Result};
use thiserror::Error;

use crate::{
    shim, toolchain::CompatibleToolchainBuilder, utils::directory::PycorsPathsProviderFromEnv,
};

#[derive(Debug, Error)]
pub enum RunError {
    #[error("No interpreter found to run command: {0}")]
    MissingInterpreter(String),
}

pub fn run(version: Option<String>, command_and_args: &str) -> Result<()> {
    let s = shlex::split(&command_and_args)
        .ok_or_else(|| anyhow!("Failed to split command from {:?}", command_and_args))?;
    let (cmd, arguments) = s.split_at(1);
    let cmd = cmd
        .get(0)
        .ok_or_else(|| anyhow!("Failed to extract command from {:?}", command_and_args))?;

    let compatible_toolchain_builder = match version {
        Some(version) => CompatibleToolchainBuilder::new().load_from_string(&version),
        None => CompatibleToolchainBuilder::new().load_from_file(),
    };
    let compatible_toolchain = compatible_toolchain_builder
        .pick_latest_if_none_found()
        .compatible_version(PycorsPathsProviderFromEnv::new())?;

    match compatible_toolchain {
        Some(compatible_toolchain) => shim::run_with(&compatible_toolchain, cmd, arguments),
        None => {
            log::error!("No Python interpreter found at all. Please install at least one!");
            Err(RunError::MissingInterpreter(command_and_args.to_string()).into())
        }
    }
}
