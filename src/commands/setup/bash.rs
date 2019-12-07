use std::{
    fs,
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
};

use structopt::clap::Shell;

use crate::{
    commands,
    constants::{
        EXECUTABLE_NAME, SHELL_CONFIG_IDENTIFYING_PATTERN_END,
        SHELL_CONFIG_IDENTIFYING_PATTERN_START,
    },
    utils, Result,
};

pub fn setup_bash(home: &Path, config_home_dir: &Path) -> Result<()> {
    let exec_name_capital = EXECUTABLE_NAME.to_uppercase();

    // Add the autocomplete too
    let autocomplete_file = config_home_dir.join(&format!("{}.bash-completion", EXECUTABLE_NAME));
    let mut f = fs::File::create(&autocomplete_file)?;
    commands::autocomplete::run(Shell::Bash, &mut f)?;

    let config_lines: Vec<String> = vec![
        format!(r#"# Add the shims directory to path, removing all other"#,),
        format!(r#"# occurrences of it from current $PATH."#,),
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
        format!(r#"else"#,),
        format!(r#"    # Shell already setup for pycors."#,),
        format!(r#"    # Disable in case we enter a 'poetry shell'"#,),
        format!(r#"    if [ -z ${{POETRY_ACTIVE+x}} ]; then"#,),
        format!(r#"        # Not in a 'poetry shell', activating."#,),
        format!(
            r#"        export PATH="${{{}_HOME}}/shims:${{PATH//${{{}_HOME}}/}}""#,
            exec_name_capital, exec_name_capital
        ),
        format!(r#"    else"#,),
        format!(r#"        # Poetry is active; disable the shim"#,),
        format!(r#"        echo "Pycors detected an active poetry shell, disabling the shim.""#,),
        format!(
            r#"        export PATH="${{PATH//${{{}_HOME}}/}}""#,
            exec_name_capital
        ),
        format!(r#"    fi"#,),
        format!(r#"fi"#,),
    ];

    let config_file = utils::directory::shell::bash::config::file_absolute()?;
    fs::create_dir_all(utils::directory::shell::bash::config::dir_absolute()?)?;
    let f = BufWriter::new(fs::File::create(&config_file)?);
    write_config_to(f, &config_lines, &autocomplete_file)?;

    for bash_config_file in &[".bashrc", ".bash_profile"] {
        let bash_config_file = home.join(bash_config_file);
        log::info!("Adding configuration to {:?}...", bash_config_file);

        let do_edit_file = if !bash_config_file.exists() {
            true
        } else {
            // Verify that file does not contain 'SHELL_CONFIG_IDENTIFYING_PATTERN_START'
            // FIXME: Don't just skip; remove it and append *at the end*
            //        to make sure the shims path appear first in PATH.
            let f = BufReader::new(fs::File::open(&bash_config_file)?);
            !file_contains(f, SHELL_CONFIG_IDENTIFYING_PATTERN_START)?
        };
        if do_edit_file {
            let mut file = BufWriter::new(
                fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&bash_config_file)?,
            );
            write_header_to(&mut file)?;
            writeln!(
                &mut file,
                "{}",
                format!(
                    r#"export {}_HOME="{}""#,
                    exec_name_capital,
                    utils::directory::config_home()?.display()
                )
            )?;
            writeln!(
                &mut file,
                "{}",
                format!(
                    r#"source ${{{}_HOME}}/{}/{}"#,
                    exec_name_capital,
                    utils::directory::shell::bash::config::dir_relative().display(),
                    utils::directory::shell::bash::config::file_name(),
                )
            )?;
            write_footer_to(&mut file)?;
        } else {
            log::warn!("Skipping since file already modified.");
        }
    }

    Ok(())
}

fn file_contains<R, S>(f: R, line_to_check: S) -> Result<bool>
where
    R: BufRead,
    S: AsRef<str>,
{
    let line_to_check = line_to_check.as_ref().trim_start_matches("#").trim();
    Ok(f.lines()
        .find(|line| match line {
            Err(e) => {
                log::error!("Failed to read line: {:?}", e,);
                false
            }
            Ok(line) => {
                if line.trim_start_matches("# ") == line_to_check {
                    log::debug!("File already contains pycors setup. Skipping.",);
                    true
                } else {
                    false
                }
            }
        })
        .is_some())
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
        format!("# properly (including the comments!)"),
        format!("# See {}", env!("CARGO_PKG_HOMEPAGE")),
        format!(
            "# WARNING: Those lines _need_ to be at the end of the file: {} needs to",
            EXECUTABLE_NAME
        ),
        format!("#          appear as soon as possible in the $PATH environment variable to",),
        format!("#          to function properly."),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_contains_success() {
        let pattern_to_match = "Pattern to find";
        let file_content = format!("Line 1\nLine 2\n{}\nLine 4", pattern_to_match);
        assert!(file_contains(file_content.as_bytes(), &pattern_to_match).unwrap());
        let file_content = format!("Line 1\nLine 2\n# {}\nLine 4", pattern_to_match);
        assert!(file_contains(file_content.as_bytes(), &pattern_to_match).unwrap());
    }

    #[test]
    fn file_contains_failure() {
        let pattern_to_match = "Pattern to find";
        let file_content = format!("Line 1\nLine 2\nDoes not contain pattern\nLine 4");
        assert!(!file_contains(file_content.as_bytes(), &pattern_to_match).unwrap());
    }

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
}
