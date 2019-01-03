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
    Autocomplete { shell: structopt::clap::Shell },

    /// List installed Python versions.
    #[structopt(name = "list")]
    List,

    /// Get path to active interpreter
    ///
    /// For example:
    ///     pycors path
    ///     /usr/local/bin
    #[structopt(name = "path")]
    Path,

    /// Get version of active interpreter
    ///
    /// For example:
    ///     pycors version
    ///     3.7.2
    #[structopt(name = "version")]
    Version,

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

    /// Setup the shim
    ///
    /// This will install pycor's binary to `~/.pycors/bin` and add the
    /// directory to the `$PATH` environment variable (through `~/.bash_profile`).
    ///
    /// Supported shells: Bash, Fish, Zsh, PowerShell and Elvish.
    #[structopt(name = "setup")]
    Setup { shell: structopt::clap::Shell },
}

fn main() -> Result<()> {
    env_logger::init();

    let settings = Settings::from_pycors_home()?;
    // Invert the Option<Result> to Result<Option> and use ? to unwrap the Result.
    let cfg_opt = load_config_file().map_or(Ok(None), |v| v.map(Some))?;

    let arguments: Vec<_> = env::args().collect();
    let (_, remaining_args) = arguments.split_at(1);

    match env::current_exe() {
        Err(e) => {
            let err_message = format!("Cannot get executable's path: {:?}", e);
            error!("{}", err_message);
            Err(format_err!("{}", err_message))?
        }
        Ok(current_exe) => {
            let exe = match current_exe.file_name() {
                Some(file_name) => file_name.to_str().ok_or_else(|| {
                    format_err!("Could not get str representation of {:?}", file_name)
                })?,
                None => {
                    let err_message = format!("Cannot get executable's path: {:?}", current_exe);
                    error!("{}", err_message);
                    Err(format_err!("{}", err_message))?
                }
            };

            if exe.starts_with("pycors") {
                debug!("Running pycors");
                pycors(&cfg_opt, &settings)?;
            } else {
                debug!("Running a Python shim");
                python_shim(exe, &cfg_opt, &settings, remaining_args)?;
            }
        }
    }

    Ok(())
}
