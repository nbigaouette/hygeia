use std::env;

use failure::format_err;
use log::{debug, error};
use structopt::StructOpt;

mod compile;
mod config;
mod download;
mod pycors;
mod settings;
mod shim;
mod utils;

use crate::config::load_config_file;
use crate::pycors::pycors;
use crate::settings::Settings;
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

    /// Install version from `.python-version`.
    #[structopt(name = "install")]
    Install,

    /// Uninstall the given installed version
    #[structopt(name = "uninstall")]
    Uninstall { version: String },

    /// Run a binary from the installed `.python-version`.
    ///
    /// For example:
    ///     pycors run "python -v"
    ///     pycors run "python -c \"print('string with spaces')\""
    #[structopt(name = "run")]
    Run { command: String },
}

fn main() -> Result<()> {
    env_logger::init();

    let settings = Settings::new()?;
    // Invert the Option<Result> to Result<Option> and use ? to unwrap the Result.
    let cfg_opt = load_config_file().map_or(Ok(None), |v| v.map(Some))?;

    let arguments: Vec<_> = env::args().collect();
    let (first_arg, remaining_args) = arguments.split_at(1);

    if first_arg.is_empty() {
        error!("Cannot get first argument.");
        Err(format_err!("Cannot get first argument"))?
    } else {
        let first_arg = &first_arg[0];
        if first_arg.ends_with("pycors") {
            debug!("Running pycors");
            pycors(&cfg_opt, &settings)?;
        } else {
            debug!("Running a Python shim");
            python_shim(&cfg_opt, &settings, remaining_args)?;
        }
    }

    Ok(())
}
