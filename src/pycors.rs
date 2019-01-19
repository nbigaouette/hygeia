use prettytable::{cell, row, Cell, Row, Table};
use structopt::StructOpt;

use crate::commands;
use crate::config::Cfg;
use crate::settings::Settings;
use crate::utils;
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
            Command::List => print_to_stdout_available_python_versions(cfg, settings)?,
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

fn print_to_stdout_available_python_versions(cfg: &Option<Cfg>, settings: &Settings) -> Result<()> {
    let mut table = Table::new();
    // Header
    table.add_row(row!["Active", "Version", "Location"]);

    let active_python = match cfg {
        None => None,
        Some(cfg) => utils::active_version(&cfg.version, settings),
    };

    if active_python.is_none() {
        if let Some(cfg) = cfg {
            table.add_row(Row::new(vec![
                Cell::new_align("✗", prettytable::format::Alignment::CENTER)
                    .with_style(prettytable::Attr::Bold)
                    .with_style(prettytable::Attr::ForegroundColor(prettytable::color::RED)),
                Cell::new_align(
                    &format!("{}", cfg.version),
                    prettytable::format::Alignment::CENTER,
                )
                .with_style(prettytable::Attr::Bold)
                .with_style(prettytable::Attr::ForegroundColor(prettytable::color::RED)),
                Cell::new_align("Not installed", prettytable::format::Alignment::CENTER)
                    .with_style(prettytable::Attr::Bold)
                    .with_style(prettytable::Attr::ForegroundColor(prettytable::color::RED)),
            ]));
        }
    }

    for installed_python in &settings.installed_python {
        let alignment = prettytable::format::Alignment::CENTER;

        let green = prettytable::Attr::ForegroundColor(prettytable::color::GREEN);

        let mut cell_active = Cell::new_align("", alignment);
        let mut cell_version = Cell::new_align(&format!("{}", installed_python.version), alignment);
        let mut cell_path = Cell::new_align(
            &format!("{}", installed_python.location.display()),
            prettytable::format::Alignment::LEFT,
        );

        if let Some(active_python) = active_python {
            if active_python == installed_python {
                cell_active = Cell::new_align("✓", alignment);
                cell_active = cell_active
                    .with_style(prettytable::Attr::Bold)
                    .with_style(green);
                cell_version = cell_version
                    .with_style(prettytable::Attr::Bold)
                    .with_style(green);
                cell_path = cell_path
                    .with_style(prettytable::Attr::Bold)
                    .with_style(green);
            }
        }

        table.add_row(Row::new(vec![cell_active, cell_version, cell_path]));
    }

    table.printstd();

    Ok(())
}
