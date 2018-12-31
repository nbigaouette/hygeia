use std::path::PathBuf;

use failure::format_err;
use log::{debug, info, warn};
use prettytable::{cell, row, Cell, Row, Table};
use semver::{Version, VersionReq};
use structopt::StructOpt;

use crate::config::Cfg;
use crate::download::download_source;
use crate::settings::{PythonVersion, Settings};
use crate::Result;
use crate::{Command, Opt};

pub fn pycors(cfg: &Option<Cfg>, settings: &mut Settings, settings_file: PathBuf) -> Result<()> {
    let opt = Opt::from_args();
    debug!("{:?}", opt);

    if let Some(subcommand) = opt.subcommand {
        match subcommand {
            Command::Autocomplete { shell } => {
                print_autocomplete_to_stdout(&shell)?;
            }
            Command::List => print_to_stdout_available_python_versions(cfg, settings)?,
            Command::Use { version } => use_given_version(&version, settings)?,
            Command::Install => install_python(cfg, settings, settings_file)?,
        }
    } else {
    }

    Ok(())
}

fn print_autocomplete_to_stdout(shell: &str) -> Result<()> {
    let shell = shell
        .parse::<structopt::clap::Shell>()
        .map_err(|string| format_err!("{}", string))?;
    Opt::clap().gen_completions_to("pycors", shell, &mut std::io::stdout());
    Ok(())
}

fn use_given_version(requested_version: &str, settings: &Settings) -> Result<()> {
    // Convert the requested version string to proper VersionReq
    // FIXME: Should a `~` be explicitly added here if user does not provide it?
    debug!("Requesting version: {}", requested_version);
    let version: VersionReq = requested_version.parse()?;
    debug!("Semantic version requirement: {}", version);

    let python_to_use = active_version(&version, settings)
        .ok_or_else(|| format_err!("No compatible version found"))?;

    debug!(
        "Using {} from {}",
        python_to_use.version,
        python_to_use.location.display()
    );

    // Write to `.python-version`
    Cfg {
        version: VersionReq::exact(&python_to_use.version),
    }
    .save()?;

    Ok(())
}

fn active_version<'a>(version: &VersionReq, settings: &'a Settings) -> Option<&'a PythonVersion> {
    // Find the compatible versions from the installed list
    let mut compatible_versions: Vec<&'a PythonVersion> = settings
        .installed_python
        .iter()
        .filter(|installed_python| version.matches(&installed_python.version))
        .collect();
    // Sort to get latest version
    compatible_versions.sort_by_key(|compatible_version| &compatible_version.version);
    debug!("Compatible versions found: {:?}", compatible_versions);

    compatible_versions.last().map(|v| *v)
}

fn print_to_stdout_available_python_versions(cfg: &Option<Cfg>, settings: &Settings) -> Result<()> {
    let mut table = Table::new();
    // Header
    table.add_row(row!["Active", "Version", "Location"]);

    let active_python = match cfg {
        None => None,
        Some(cfg) => active_version(&cfg.version, settings),
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

fn install_python(
    cfg: &Option<Cfg>,
    settings: &mut Settings,
    settings_file: PathBuf,
) -> Result<()> {
    let version: VersionReq = match cfg {
        None => Cfg::from_user_input()?.version,
        Some(cfg) => cfg.version.clone(),
    };
    debug!("Installing Python {}", version);

    if settings
        .installed_python
        .iter()
        .find(|installed_python| version.matches(&installed_python.version))
        .is_some()
    {
        info!("Python version {} already installed!", version);
    } else {
        // Get the last version compatible with the given version
        // download_source(&version)?;
        download_source(&Version::parse("3.7.2")?)?;
    }

    Ok(())
}
