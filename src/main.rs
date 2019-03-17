use std::env;

use failure::format_err;
use human_panic::setup_panic;
use log::{debug, error};
use structopt::StructOpt;

mod commands;
mod config;
mod settings;
mod shim;
mod utils;

use crate::{
    commands::Command,
    config::{load_config_file, Cfg},
    settings::Settings,
};

pub type Result<T> = std::result::Result<T, failure::Error>;

/// Control which Python toolchain to use on a directory basis.
#[derive(StructOpt, Debug)]
struct Opt {
    #[structopt(subcommand)]
    subcommand: Option<commands::Command>,
}

fn main() -> Result<()> {
    setup_panic!();

    std::env::var("RUST_LOG").or_else(|_| -> Result<String> {
        let rust_log = "pycors=warn".to_string();
        std::env::set_var("RUST_LOG", &rust_log);
        Ok(rust_log)
    })?;

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

pub fn pycors(cfg: &Option<Cfg>, settings: &Settings) -> Result<()> {
    let opt = Opt::from_args();
    log::debug!("{:?}", opt);

    if let Some(subcommand) = opt.subcommand {
        match subcommand {
            Command::Autocomplete { shell } => {
                commands::autocomplete::run(shell)?;
            }
            Command::List => commands::list::run(cfg, settings)?,
            Command::Path => commands::path::run(cfg, settings)?,
            Command::Version => commands::version::run(cfg, settings)?,
            Command::Select { version } => commands::select::run(&version, settings)?,
            Command::Install {
                from_version,
                install_extra_packages,
                install_extra_packages_from,
            } => {
                commands::install::run(from_version, cfg, settings)?;
            }
            Command::Run { command } => commands::run::run(cfg, settings, &command)?,
            Command::Setup { shell } => commands::setup::run(shell)?,
        }
    } else {
    }

    Ok(())
}

pub fn python_shim(
    command: &str,
    cfg: &Option<Cfg>,
    settings: &Settings,
    arguments: &[String],
) -> Result<()> {
    let interpreter_to_use = utils::get_interpreter_to_use(cfg, settings)?;

    shim::run(&interpreter_to_use, command, arguments)
}
