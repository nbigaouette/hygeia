use std::{env, fs, io::Write};

use failure::format_err;
use log::debug;
#[cfg(target_os = "windows")]
use log::error;
use semver::VersionReq;
use shlex;
use structopt::{clap::Shell, StructOpt};
use subprocess::{Exec, Redirection};

use crate::config::Cfg;
use crate::pycors::active_version;
use crate::settings::Settings;
use crate::utils;
use crate::{Opt, Result};

pub fn python_shim(
    command: &str,
    cfg: &Option<Cfg>,
    settings: &Settings,
    arguments: &[String],
) -> Result<()> {
    run(cfg, settings, command, arguments)
}

pub fn run_command(cfg: &Option<Cfg>, settings: &Settings, command_and_args: &str) -> Result<()> {
    let s = shlex::split(&command_and_args)
        .ok_or_else(|| format_err!("Failed to split command from {:?}", command_and_args))?;
    let (cmd, arguments) = s.split_at(1);
    let cmd = cmd
        .get(0)
        .ok_or_else(|| format_err!("Failed to extract command from {:?}", command_and_args))?;

    run(cfg, settings, cmd, arguments)
}

fn run<S>(cfg: &Option<Cfg>, settings: &Settings, command: &str, arguments: &[S]) -> Result<()>
where
    S: AsRef<str> + std::convert::AsRef<std::ffi::OsStr> + std::fmt::Debug,
{
    // If `cfg` is `None`, check if there is something in `Settings`; pick the first found
    // interpreter to construct a `cfg`.
    // Since the `cfg` used in the functions is expected to be a reference, we need to store
    // the setting's cfg in a variable to be able to refer to it.
    let latest_interpreter_in_settings = match settings.installed_python.iter().nth(0) {
        None => None,
        Some(latest_interpreter_found) => Some(Cfg {
            version: VersionReq::exact(&latest_interpreter_found.version),
        }),
    };

    let cfg: &Cfg = cfg
        .as_ref()
        .or_else(|| latest_interpreter_in_settings.as_ref())
        .ok_or_else(|| format_err!("No Python runtime configured. Use `pycors use <version>`."))?;

    let active_python = active_version(&cfg.version, settings).ok_or_else(|| {
        error!(
            "Could not find Python {} as requested from the file `.python-version`.",
            cfg.version
        );
        error!("Either:");
        error!("    1) Remove the file `.python-version` to use (one of) the interpreter(s) available in your $PATH.");
        error!("    2) Edit the file to use an installed interpreter.");
        error!("       For example, to list available interpreters:");
        error!("           pycors list");
        error!("       Then select a version to use:");
        error!("           pycors use ~3.7");
        format_err!("No active Python runtime found.")
    })?;

    debug!("active_python: {:?}", active_python);

    let bin_path = active_python.location.join("bin");

    let path_env = env::var("PATH")?;
    if path_env.is_empty() {
        env::set_var("PATH", &bin_path);
    } else {
        env::set_var("PATH", format!("{}:{}", bin_path.display(), path_env));
    }

    // debug!("Command:   {:?}", command_full_path);
    debug!("Arguments: {:?}", arguments);

    Exec::cmd(&command)
        .args(arguments)
        .stdout(Redirection::None)
        .stderr(Redirection::None)
        .join()?;

    Ok(())
}

pub fn setup_shim(shell: &Shell) -> Result<()> {
    debug!("Setting up the shim...");

    // Copy itself into ~/.pycors/bin
    let pycors_home_dir = utils::pycors_home()?;
    let bin_dir = pycors_home_dir.join("bin");
    if !utils::path_exists(&bin_dir) {
        debug!("Directory {:?} does not exists, creating.", bin_dir);
        fs::create_dir_all(&bin_dir)?;
    }
    let copy_from = env::current_exe()?;
    let copy_to = bin_dir.join("pycors");
    debug!("Copying {:?} into {:?}...", copy_from, copy_to);
    utils::copy_file(&copy_from, &copy_to)?;

    // Once the shim is in place, create hard links to it.
    let hardlinks_version_suffix = &[
        "python###",
        "idle###",
        "pip###",
        "pydoc###",
        // Internals
        "python###-config",
        "python###dm-config",
        // Extras
        "pipenv###",
        "poetry###",
    ];
    let hardlinks_dash_version_suffix = &["2to3###", "easy_install###", "pyvenv###"];

    // Create simple hardlinks: `pycors` --> `bin`
    utils::create_hard_links(&copy_from, hardlinks_version_suffix, &bin_dir, "")?;
    utils::create_hard_links(&copy_from, hardlinks_dash_version_suffix, &bin_dir, "")?;

    // Create major version hardlinks: `pycors` --> `bin3` and `pycors` --> `bin2`
    for major in &["2", "3"] {
        utils::create_hard_links(&copy_from, hardlinks_version_suffix, &bin_dir, major)?;
        utils::create_hard_links(
            &copy_from,
            hardlinks_dash_version_suffix,
            &bin_dir,
            &format!("-{}", major),
        )?;
    }

    // Create an dummy file that will be recognized when searching the PATH for
    // python interpreters. We don't want to "find" the shims we install here.
    let pycors_dummy_file = bin_dir.join("pycors_dummy_file");
    let mut file = fs::File::create(&pycors_dummy_file)?;
    writeln!(file, "This file's job is to tell pycors the directory contains shim, not real Python interpreters.")?;

    // Add ~/.pycors/bin to $PATH in ~/.bash_profile and install autocomplete
    match shell {
        structopt::clap::Shell::Bash => {
            #[cfg(target_os = "windows")]
            {
                let message = "Windows support not yet implemented.";
                error!("{}", message);
                Err(format_err!("{}", message))
            }
            #[cfg(not(target_os = "windows"))]
            {
                let home =
                    dirs::home_dir().ok_or_else(|| format_err!("Error getting home directory"))?;
                let bash_profile = home.join(".bash_profile");

                // Add the autocomplete too
                let autocomplete_file = pycors_home_dir.join("pycors.bash-completion");
                let mut f = fs::File::create(&autocomplete_file)?;
                Opt::clap().gen_completions_to("pycors", *shell, &mut f);

                debug!("Adding {:?} to $PATH in {:?}...", bin_dir, bash_profile);
                let mut file = fs::OpenOptions::new().append(true).open(&bash_profile)?;
                let lines = &[
                    String::from(""),
                    "#################################################".to_string(),
                    "# These lines were added by pycors.".to_string(),
                    "# See https://github.com/nbigaouette/pycors".to_string(),
                    format!(r#"export PATH="{}:$PATH""#, bin_dir.display()),
                    format!(r#"source "{}""#, autocomplete_file.display()),
                    "#################################################".to_string(),
                ];
                for line in lines {
                    // debug!("    {}", line);
                    writeln!(file, "{}", line)?;
                }

                Ok(())
            }
        }
        _ => Err(format_err!("Unsupported shell: {}", shell)),
    }
}
