use std::{env, path::PathBuf};

use failure::format_err;
use log::{debug, error, warn};
use structopt::StructOpt;

mod config;
mod download;
mod pycors;
mod settings;
mod shim;
mod utils;

use crate::config::{load_config_file, Cfg};
use crate::pycors::pycors;
use crate::settings::{load_settings_file, PythonVersion, Settings};
use crate::shim::python_shim;

pub type Result<T> = std::result::Result<T, failure::Error>;

/// Control which Python toolchain to use on a directory basis.
#[derive(StructOpt, Debug)]
struct Opt {
    #[structopt(subcommand)]
    subcommand: Option<Command>,
}

#[derive(StructOpt, Debug)]
enum Command {
    /// Print to stdout an autocomplete script for the specified shell.
    ///
    /// For example:
    ///     pycors autocomplete bash > /etc/bash_completion.d/pycors.bash-completion
    #[structopt(name = "autocomplete")]
    Autocomplete { shell: String },

    /// List installed Python versions.
    #[structopt(name = "list")]
    List,

    /// Use specified Python versions.
    ///
    /// The specified Python version will be installed if not already installed.
    ///
    /// For example:
    ///     pycors use 3.6
    /// will install ~3.6
    #[structopt(name = "use")]
    Use { version: String },
}

fn main() -> Result<()> {
    env_logger::init();

    let (mut settings, settings_file) = load_settings_file()?;
    // Invert the Option<Result> to Result<Option> and use ? to unwrap the Result.
    let cfg_opt = load_config_file().map_or(Ok(None), |v| v.map(Some))?;

    match env::args().next() {
        None => {
            error!("Cannot get first argument.");
            Err(format_err!("Cannot get first argument"))?
        }
        Some(arg) => {
            if arg.ends_with("pycors") {
                debug!("Running pycors");
                pycors(&cfg_opt, &mut settings, settings_file)?;
            } else {
                debug!("Running a Python shim");
                python_shim()?;
            }
        }
    }

    Ok(())
}
