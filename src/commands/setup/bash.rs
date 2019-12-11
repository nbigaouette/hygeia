use std::{
    fs,
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
};

use anyhow::Context;
use structopt::clap::Shell;

use crate::{
    commands,
    constants::{
        EXECUTABLE_NAME, SHELL_CONFIG_IDENTIFYING_PATTERN_END,
        SHELL_CONFIG_IDENTIFYING_PATTERN_START,
    },
    utils::{self, directory::PycorsPathsProviderFromEnv},
    Result,
};

pub fn setup_bash(home: &Path) -> Result<()> {
    let exec_name_capital = EXECUTABLE_NAME.to_uppercase();
    let paths_provider = PycorsPathsProviderFromEnv::new();

    // Add the autocomplete too
    let autocomplete_file = paths_provider
        .config_home()
        .join(utils::directory::shell::bash::config::autocomplete());
    let mut f = fs::File::create(&autocomplete_file)
        .with_context(|| format!("Failed creating file {:?}", autocomplete_file))?;
    commands::autocomplete::run(Shell::Bash, &mut f)?;

    let config_lines: Vec<String> = vec![
        String::from(r#"# Add the shims directory to path, removing all other"#),
        String::from(r#"# occurrences of it from current $PATH."#),
        format!(
            r#"if [ -z ${{{}_INITIALIZED+x}} ]; then"#,
            exec_name_capital
        ),
        format!(
            r#"    # Setup {}: prepends the shims directory to PATH"#,
            EXECUTABLE_NAME
        ),
        format!(
            r#"    export PATH="${{{}_HOME}}/shims:${{PATH//${{{}_HOME}}/}}""#,
            exec_name_capital, exec_name_capital
        ),
        format!(r#"    export {}_INITIALIZED=1"#, exec_name_capital),
        String::from(r#"else"#),
        String::from(r#"    # Shell already setup for pycors."#),
        String::from(r#"    # Disable in case we enter a 'poetry shell'"#),
        String::from(r#"    if [ -z ${POETRY_ACTIVE+x} ]; then"#),
        String::from(r#"        # Not in a 'poetry shell', activating."#),
        format!(
            r#"        export PATH="${{{}_HOME}}/shims:${{PATH//${{{}_HOME}}/}}""#,
            exec_name_capital, exec_name_capital
        ),
        String::from(r#"    else"#),
        String::from(r#"        # Poetry is active; disable the shim"#),
        String::from(
            r#"        echo "Pycors detected an active poetry shell, disabling the shim.""#,
        ),
        format!(
            r#"        export PATH="${{PATH//${{{}_HOME}}/}}""#,
            exec_name_capital
        ),
        String::from(r#"    fi"#),
        String::from(r#"fi"#),
    ];

    let config_file = paths_provider
        .config_home()
        .join(utils::directory::shell::bash::config::file_path());
    let f = BufWriter::new(fs::File::create(&config_file)?);
    write_config_to(f, &config_lines, &autocomplete_file)?;

    for bash_config_file in &[".bashrc", ".bash_profile"] {
        let tmp_file_path = PycorsPathsProviderFromEnv::new()
            .cache()
            .join(bash_config_file);
        let bash_config_file = home.join(bash_config_file);

        if !bash_config_file.exists() {
            log::debug!("File {:?} does not exists, skipping.", bash_config_file);
            continue;
        }

        log::info!("Adding configuration to {:?}...", bash_config_file);

        let mut tmp_file =
            BufWriter::new(fs::File::create(&tmp_file_path).with_context(|| {
                format!("Failed to create temporary fille {:?}", tmp_file_path)
            })?);
        let mut config_reader = BufReader::new(
            fs::File::open(&bash_config_file)
                .with_context(|| format!("Failed to open file {:?}", bash_config_file))?,
        );
        remove_block(&mut config_reader, &mut tmp_file).with_context(|| {
            format!(
                "Failed to remove custom config block from {:?}",
                bash_config_file
            )
        })?;
        // Make sure we close the file
        std::mem::drop(config_reader);

        write_header_to(&mut tmp_file)
            .with_context(|| format!("Failed to write block header to {:?}", tmp_file_path))?;
        writeln!(
            &mut tmp_file,
            "{}",
            format!(
                r#"export {}_HOME="{}""#,
                exec_name_capital,
                PycorsPathsProviderFromEnv::new().config_home().display()
            )
        )
        .with_context(|| format!("Failed to export line to {:?}", tmp_file_path))?;
        writeln!(
            &mut tmp_file,
            "{}",
            format!(
                r#"source ${{{}_HOME}}/{}"#,
                exec_name_capital,
                utils::directory::shell::bash::config::file_path().display()
            )
        )
        .with_context(|| format!("Failed to write source line to {:?}", tmp_file_path))?;
        write_footer_to(&mut tmp_file)
            .with_context(|| format!("Failed to write block footer to {:?}", tmp_file_path))?;
        std::mem::drop(tmp_file);

        // Move tmp file back atomically
        fs::rename(&tmp_file_path, &bash_config_file).with_context(|| {
            format!(
                "Failed to rename temporary file {:?} to {:?}",
                tmp_file_path, bash_config_file
            )
        })?;
    }

    Ok(())
}

fn write_header_to<W>(f: &mut W) -> Result<()>
where
    W: Write,
{
    let lines = &[
        String::from(""),
        String::from(""),
        String::from(
            "#############################################################################",
        ),
        format!("# {}", SHELL_CONFIG_IDENTIFYING_PATTERN_START),
        format!(
            "# These lines were added by {} and are required for it to function",
            EXECUTABLE_NAME
        ),
        String::from("# properly (including the comments!)"),
        format!("# See {}", env!("CARGO_PKG_HOMEPAGE")),
        format!(
            "# WARNING: Those lines _need_ to be at the end of the file: {} needs to",
            EXECUTABLE_NAME
        ),
        String::from("#          appear as soon as possible in the $PATH environment variable to"),
        String::from("#          to function properly."),
    ];
    for line in lines {
        writeln!(f, "{}", line)?;
    }

    Ok(())
}

fn write_footer_to<W>(f: &mut W) -> Result<()>
where
    W: Write,
{
    let lines = &[
        format!("# {}", SHELL_CONFIG_IDENTIFYING_PATTERN_END),
        String::from(
            "#############################################################################",
        ),
    ];
    for line in lines {
        writeln!(f, "{}", line)?;
    }

    Ok(())
}

fn write_config_to<W, S>(mut f: W, lines_to_append: &[S], autocomplete_file: &Path) -> Result<()>
where
    W: Write,
    S: AsRef<str>,
{
    for line in lines_to_append {
        writeln!(f, "{}", line.as_ref())?;
    }
    writeln!(f, r#"source "{}""#, autocomplete_file.display())?;

    Ok(())
}

fn remove_block<R, W>(f_in: &mut R, f_out: &mut W) -> Result<()>
where
    W: Write,
    R: BufRead,
{
    let original_content: Vec<String> = f_in
        .lines()
        .filter_map(|line| match line {
            Err(e) => {
                log::error!("Error reading line: {:?}", e);
                None
            }
            Ok(line) => Some(line),
        })
        .collect();

    let mut idx = 0;
    let mut outside_block = true;
    while idx < original_content.len() {
        let current_line = &original_content[idx];
        match original_content.get(idx + 1) {
            None => {
                // There is no next line; 'original_content[idx]' is the last line.
                if outside_block {
                    writeln!(f_out, "{}", current_line)?;
                }
            }
            Some(next_line) => {
                if next_line.contains(&SHELL_CONFIG_IDENTIFYING_PATTERN_START) {
                    // Next line is the start of our block pattern.
                    // This means the current line is the '############...' header.
                    outside_block = false;
                } else if current_line.contains(&SHELL_CONFIG_IDENTIFYING_PATTERN_END) {
                    // Current line is the end of our block pattern.
                    // This means the next line is the '############...' footer.
                    // We thus want to advance the line index two times
                    outside_block = true;
                    idx += 1;
                } else if outside_block {
                    writeln!(f_out, "{}", current_line)?;
                } else {
                    assert!(!outside_block);
                }
            }
        }
        idx += 1;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn write_config_to_string() {
        let mut writer: Vec<u8> = Vec::new();
        let lines_to_append = vec![String::from("# Line to append")];

        let autocomplete_file = Path::new("foo.sh");

        write_config_to(&mut writer, &lines_to_append, &autocomplete_file).unwrap();

        let expected = "# Line to append\nsource \"foo.sh\"\n";
        let written = String::from_utf8(writer).unwrap();

        assert_eq!(written, expected);
    }

    #[test]
    fn remove_block_from_string() {
        let input = format!(
            "line 1\n#### Header\n# {}\nbla bla bla\n# {}\n### Footer\nline 5",
            SHELL_CONFIG_IDENTIFYING_PATTERN_START, SHELL_CONFIG_IDENTIFYING_PATTERN_END
        );
        let expected = "line 1\nline 5\n";

        let mut writer: Vec<u8> = Vec::new();
        let mut reader = Cursor::new(input);

        remove_block(&mut reader, &mut writer).unwrap();

        let written = String::from_utf8(writer).unwrap();

        assert_eq!(written, expected);
    }
}
