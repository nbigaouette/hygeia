use std::path::PathBuf;

use structopt::{self, StructOpt};

pub mod autocomplete;
pub mod install;
pub mod list;
pub mod path;
pub mod run;
pub mod select;
pub mod setup;
pub mod version;

#[derive(StructOpt, Debug)]
pub enum Command {
    /// Print to stdout an autocomplete script for the specified shell
    ///
    /// For example:
    ///     pycors autocomplete bash > /etc/bash_completion.d/pycors.bash-completion
    #[structopt(name = "autocomplete")]
    Autocomplete { shell: structopt::clap::Shell },

    /// List installed Python versions
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

    /// Select specified Python versions to use
    ///
    /// The specified Python version will _not_ be installed if not already installed.
    /// Use `pycors install` for this.
    ///
    /// For example:
    ///     pycors select 3.6
    /// will select ~3.6 (the most up to date version of the 3.6 series).
    ///
    ///     pycors select =3.7.2
    /// will select an exact version.
    #[structopt(name = "select")]
    Select {
        version: String,

        /// Install extra Python packages from file at default location
        ///
        /// The default location is ${PYCORS_HOME}/config.toml
        ///
        /// Installation will be performed using pip.
        #[structopt(long = "extra", short = "e")]
        install_extra_packages: bool,

        /// Install extra Python packages from specific file
        ///
        /// Installation will be performed using pip.
        #[structopt(long = "extra-from", short = "f")]
        install_extra_packages_from: Option<PathBuf>,
    },

    /// Install version, either from the provided version or from `.python-version`
    #[structopt(name = "install")]
    Install {
        /// Specified version to install
        from_version: Option<String>,

        /// Install extra Python packages from file at default location
        ///
        /// The default location is ${PYCORS_HOME}/config.toml
        ///
        /// Installation will be performed using pip.
        #[structopt(long = "extra", short = "e")]
        install_extra_packages: bool,

        /// Install extra Python packages from specific file
        ///
        /// Installation will be performed using pip.
        #[structopt(long = "extra-from", short = "f")]
        install_extra_packages_from: Option<PathBuf>,
    },

    /// Run a binary from the installed `.python-version`
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
