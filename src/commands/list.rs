use std::{fmt, path::PathBuf};

use prettytable::{cell, row, Cell, Row, Table};
use semver::Version;

use crate::{
    installed::{find_installed_toolchains, InstalledToolchain},
    utils, Result,
};

struct ToolChainTableLine {
    active: bool,
    version: Version,
    custom_install: bool,
    location: PathBuf,
    installed: bool,
}

struct ToolChainTable(Vec<ToolChainTableLine>);

impl ToolChainTable {
    fn new(installed_toolchains: &[InstalledToolchain]) -> ToolChainTable {
        let list: Vec<ToolChainTableLine> = installed_toolchains
            .iter()
            .map(|t| ToolChainTableLine {
                active: false,
                version: t.version.clone(),
                custom_install: t.is_custom_install(),
                location: t.location.clone(),
                installed: true,
            })
            .collect();
        ToolChainTable(list)
    }
}

impl ToolChainTable {
    fn printstd(&self) {
        let mut table = Table::new();
        // ╭──────────┬───────────┬───────────────────────┬────────────╮
        // │ Active   │ Version   │ Installed by pycors   │ Location   │
        // ╰──────────┴───────────┴───────────────────────┴────────────╯
        // Header
        table.add_row(row![
            "Active",
            "Version",
            &format!("Installed by {}", crate::EXECUTABLE_NAME),
            "Location"
        ]);

        let green = prettytable::Attr::ForegroundColor(prettytable::color::GREEN);
        let red = prettytable::Attr::ForegroundColor(prettytable::color::RED);
        let bold = prettytable::Attr::Bold;

        self.0.iter().for_each(|t| {
            let (active_char, line_color, line_style) = match (t.active, t.installed) {
                (true, true) => ("✓", Some(green), Some(bold)),
                (true, false) => ("✗", Some(red), None),
                (false, _) => ("", None, None),
            };
            let custom_char = if t.custom_install { "✓" } else { "" };

            let mut col_1 = Cell::new_align(active_char, prettytable::format::Alignment::CENTER);

            let mut col_2 = Cell::new_align(
                &format!("{}", t.version),
                prettytable::format::Alignment::CENTER,
            );

            let mut col_3 = Cell::new_align(&custom_char, prettytable::format::Alignment::CENTER);

            let mut col_4 = Cell::new_align(
                &format!("{}", t.location.display()),
                prettytable::format::Alignment::LEFT,
            );

            line_color.map(|c| {
                col_1.style(c);
                col_2.style(c);
                col_3.style(c);
                col_4.style(c);
            });
            line_style.map(|c| {
                col_1.style(c);
                col_2.style(c);
                col_3.style(c);
                col_4.style(c);
            });

            table.add_row(Row::new(vec![col_1, col_2, col_3, col_4]));
        });

        table.printstd();
    }
}

pub fn run() -> Result<()> {
    let installed_toolchains: Vec<InstalledToolchain> = find_installed_toolchains()?;

    let toolchains_table = ToolChainTable::new(&installed_toolchains);

    toolchains_table.printstd();

    Ok(())
}
