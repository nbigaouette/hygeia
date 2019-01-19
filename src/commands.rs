use structopt::{self, StructOpt};

pub mod autocomplete;
pub mod install;
pub mod list;
pub mod path;
pub mod run;
pub mod setup;
pub mod use_command;
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

    /// Use specified Python versions
    ///
    /// The specified Python version will be installed if not already installed.
    ///
    /// For example:
    ///     pycors use 3.6
    /// will install ~3.6
    #[structopt(name = "use")]
    Use { version: String },

    /// Install version, either from the provided version or from `.python-version`
    #[structopt(name = "install")]
    Install { from_version: Option<String> },

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
