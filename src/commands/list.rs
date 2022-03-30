use std::path::PathBuf;

use prettytable::{cell, row, Cell, Row, Table};
use semver::VersionReq;

use crate::{
    constants::EXECUTABLE_NAME,
    toolchain::{
        find_installed_toolchains, installed::InstalledToolchain, is_a_custom_install,
        SelectedToolchain, ToolchainFile,
    },
    utils::directory::PycorsPathsProviderFromEnv,
    Result,
};

pub fn run() -> Result<()> {
    let paths_provider = PycorsPathsProviderFromEnv::new();
    let installed_toolchains: Vec<InstalledToolchain> = find_installed_toolchains(&paths_provider)?;

    let mut toolchains_table = ToolChainTable::new(&installed_toolchains);

    if let Some(toolchain_file) = ToolchainFile::load()? {
        let selected_toolchain =
            SelectedToolchain::from_toolchain_file(&toolchain_file, &installed_toolchains);

        // Information was loaded from .python-version. Mark the relevant installed toolchain
        // as being active. If not found, add it to the list as not-installed.
        toolchains_table.append(&selected_toolchain, true);
    }

    toolchains_table.printstd();

    Ok(())
}

struct ToolChainTableLine {
    active: bool,
    version: Option<VersionReq>,
    custom_install: bool,
    location: Option<PathBuf>,
    installed: bool,
}

struct ToolChainTable(Vec<ToolChainTableLine>);

impl ToolChainTable {
    fn new(installed_toolchains: &[InstalledToolchain]) -> ToolChainTable {
        let list: Vec<ToolChainTableLine> = installed_toolchains
            .iter()
            .map(|t| ToolChainTableLine {
                active: false,
                version: Some(format!("={}", t.version).parse().unwrap()),
                custom_install: t.is_custom_install(),
                location: Some(t.location.clone()),
                installed: true,
            })
            .collect();
        ToolChainTable(list)
    }

    fn append(&mut self, toolchain: &SelectedToolchain, active: bool) {
        match self.0.iter_mut().find(|t| match (&t.version, &t.location) {
            (None, _) => false,
            (_, None) => false,
            (Some(version), Some(location)) => {
                toolchain.same_location(location) && toolchain.same_version(version)
            }
        }) {
            Some(installed_toolchain_line) => {
                // We found the toolchain in the list; change its properties
                installed_toolchain_line.active = active;
            }
            None => {
                // The passed toolchain was not found in the list. Append it.
                let line: ToolChainTableLine = match toolchain {
                    SelectedToolchain::InstalledToolchain(t) => ToolChainTableLine {
                        active,
                        version: Some(format!("={}", t.version).parse().unwrap()),
                        custom_install: is_a_custom_install(&t.location),
                        location: Some(t.location.clone()),
                        installed: true,
                    },
                    SelectedToolchain::NotInstalledToolchain(t) => ToolChainTableLine {
                        active,
                        version: t.version.clone(),
                        custom_install: t
                            .location
                            .as_ref()
                            .map(|p| is_a_custom_install(&p))
                            .unwrap_or(false),
                        location: t.location.clone(),
                        installed: false,
                    },
                };
                // Insert at the top of the list
                self.0.insert(0, line);
            }
        }
    }
}

impl ToolChainTable {
    fn printstd(&self) {
        let mut table = Table::new();
        // ╭──────────┬───────────┬───────────────────────┬────────────╮
        // │ Active   │ Version   │ Installed by hygeia   │ Location   │
        // ╰──────────┴───────────┴───────────────────────┴────────────╯
        // Header
        table.add_row(row![
            "Active",
            "Version",
            &format!("Installed by {}", EXECUTABLE_NAME),
            "Location"
        ]);

        let green = prettytable::Attr::ForegroundColor(prettytable::color::GREEN);
        let red = prettytable::Attr::ForegroundColor(prettytable::color::RED);
        let bold = prettytable::Attr::Bold;

        self.0.iter().for_each(|t: &ToolChainTableLine| {
            let (active_char, line_color, line_style) = match (t.active, t.installed) {
                (true, true) => ("✓", Some(green), Some(bold)),
                (true, false) => ("✗", Some(red), Some(bold)),
                (false, _) => ("", None, None),
            };
            let custom_char = if t.custom_install { "✓" } else { "" };

            let mut col_1 = Cell::new_align(active_char, prettytable::format::Alignment::CENTER);

            let mut col_2 = Cell::new_align(
                &t.version
                    .as_ref()
                    .map(|t| format!("{}", t).replace('=', ""))
                    .unwrap_or_default(),
                prettytable::format::Alignment::CENTER,
            );

            let mut col_3 = Cell::new_align(custom_char, prettytable::format::Alignment::CENTER);

            let mut col_4 = Cell::new_align(
                &t.location
                    .as_ref()
                    .map(|t| format!("{}", t.display()))
                    .unwrap_or_default(),
                prettytable::format::Alignment::LEFT,
            );

            if let Some(c) = line_color {
                col_1.style(c);
                col_2.style(c);
                col_3.style(c);
                col_4.style(c);
            }

            if let Some(c) = line_style {
                col_1.style(c);
                col_2.style(c);
                col_3.style(c);
                col_4.style(c);
            }

            table.add_row(Row::new(vec![col_1, col_2, col_3, col_4]));
        });

        table.printstd();
    }
}
