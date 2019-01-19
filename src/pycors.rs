use structopt::StructOpt;

use crate::commands;
use crate::config::Cfg;
use crate::settings::Settings;
use crate::Result;
use crate::{commands::Command, Opt};

pub fn pycors(cfg: &Option<Cfg>, settings: &Settings) -> Result<()> {
    let opt = Opt::from_args();
    log::debug!("{:?}", opt);

    if let Some(subcommand) = opt.subcommand {
        match subcommand {
            Command::Autocomplete { shell } => {
                commands::autocomplete::print_autocomplete_to_stdout(shell)?;
            }
            Command::List => {
                commands::list::print_to_stdout_available_python_versions(cfg, settings)?
            }
            Command::Path => commands::path::print_active_interpreter_path(cfg, settings)?,
            Command::Version => commands::version::print_active_interpreter_version(cfg, settings)?,
            Command::Use { version } => {
                commands::use_command::use_given_version(&version, settings)?
            }
            Command::Install { from_version } => {
                commands::install::install_python(from_version, cfg, settings)?;
            }
            Command::Run { command } => commands::run::run_command(cfg, settings, &command)?,
            Command::Setup { shell } => commands::setup::setup_shim(shell)?,
        }
    } else {
    }

    Ok(())
}
