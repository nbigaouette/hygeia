use std::{env, fs, io::Write};

use failure::format_err;
use log::debug;
#[cfg(target_os = "windows")]
use log::error;
use shlex;
use structopt::{clap::Shell, StructOpt};
use subprocess::{Exec, Redirection};

use crate::config::Cfg;
use crate::pycors::active_version;
use crate::settings::Settings;
use crate::utils;
use crate::Result;

pub fn python_shim(cfg: &Option<Cfg>, settings: &Settings, arguments: &[String]) -> Result<()> {
    run(cfg, settings, "python", arguments)
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
    let cfg = cfg
        .as_ref()
        .ok_or_else(|| format_err!("No Python runtime configured. Use `pycors use <version>`."))?;

    let active_python = active_version(&cfg.version, settings)
        .ok_or_else(|| format_err!("No active Python runtime found."))?;

    debug!("active_python: {:?}", active_python);

    let bin_path = active_python.location.join("bin");

    let path_env = env::var("PATH")?;
    if path_env.is_empty() {
        env::set_var("PATH", &bin_path);
    } else {
        env::set_var("PATH", format!("{}:{}", bin_path.display(), path_env));
    }

    debug!("Command: {:?}   Arguments: {:?}", command, arguments);

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

    // Add ~/.pycors/bin to $PATH in ~/.bash_profile

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
                debug!("Adding {:?} to $PATH in {:?}...", bin_dir, bash_profile);
                let mut file = fs::OpenOptions::new().append(true).open(&bash_profile)?;
                let lines = &[
                    String::from(""),
                    "#################################################".to_string(),
                    "# These lines were added by pycors.".to_string(),
                    "# See https://github.com/nbigaouette/pycors".to_string(),
                    format!(r#"export PATH="{}:$PATH""#, bin_dir.display()),
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
