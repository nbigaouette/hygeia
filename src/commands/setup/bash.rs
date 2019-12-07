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

const BASH_TEMPLATE: &str = r#"# Add the shims directory to path, removing all other
# occurrences of it from current $PATH.
if [ -z ${PYCORS_INITIALIZED+x} ]; then
    # Setup pycors: prepends the shims directory to PATH
    export PATH="${PYCORS_HOME}/shims:${PATH//${PYCORS_HOME}/}"
    export PYCORS_INITIALIZED=1
else
    # Shell already setup for pycors.
    # Disable in case we enter a 'poetry shell'
    if [ -z ${POETRY_ACTIVE+x} ]; then
        # Not in a 'poetry shell', activating.
        export PATH="${PYCORS_HOME}/shims:${PATH//${PYCORS_HOME}/}"
    else
        # Poetry is active; disable the shim
        echo "Pycors detected an active poetry shell, disabling the shim."
        export PATH="${PATH//${PYCORS_HOME}/}"
    fi
fi"#;

pub fn setup_bash(home: &Path, config_home_dir: &Path, shims_dir: &Path) -> Result<()> {
    let bash_config_files = &[home.join(".bashrc"), home.join(".bash_profile")];

    // Add the autocomplete too
    let autocomplete_file = config_home_dir.join(&format!("{}.bash-completion", EXECUTABLE_NAME));
    let mut f = fs::File::create(&autocomplete_file)?;
    commands::autocomplete::run(Shell::Bash, &mut f)?;

    let export_home_line = format!(
        r#"export PYCORS_HOME="{}""#,
        utils::directory::config_home()?.display()
    );

    let lines_to_append: Vec<&str> = vec![&export_home_line, BASH_TEMPLATE];

    // FIXME: Don't append the same content in two files; save the content to a file and
    //        add a 'source ...' to the two files.
    for bash_config_file in bash_config_files {
        log::info!(
            "Adding {:?} to $PATH in {:?}...",
            shims_dir,
            bash_config_file
        );

        let do_edit_file = if !bash_config_file.exists() {
            true
        } else {
            // Verify that file does not contain a line `export PYCORS_HOME=...`
            // FIXME: Don't just skip; remove it and append *at the end*
            //        to make sure the shims path appear first in PATH.
            let f = BufReader::new(fs::File::open(&bash_config_file)?);
            !file_contains(f, &export_home_line)?
        };

        if do_edit_file {
            let file = BufWriter::new(
                fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&bash_config_file)?,
            );
            append_to(file, &lines_to_append, &autocomplete_file)?;
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
    Ok(f.lines()
        .find(|line| match line {
            Err(e) => {
                log::error!("Failed to read line: {:?}", e,);
                false
            }
            Ok(line) => {
                if line == line_to_check.as_ref() {
                    log::debug!("File already contains pycors setup. Skipping.",);
                    true
                } else {
                    false
                }
            }
        })
        .is_some())
}

fn append_to<W, S>(mut f: W, lines_to_append: &[S], autocomplete_file: &Path) -> Result<()>
where
    W: Write,
    S: AsRef<str>,
{
    let lines = &[
        String::from(""),
        String::from(
            "#############################################################################",
        ),
        format!("# {}", SHELL_CONFIG_IDENTIFYING_PATTERN_START),
        format!("# See {}", env!("CARGO_PKG_HOMEPAGE")),
        format!("# WARNING: Those lines _need_ to be at the end of"),
        format!(
            "#          the file: {} needs to appear as soon",
            EXECUTABLE_NAME
        ),
        format!("#          as possible in the $PATH environment"),
        format!("#          variable to function properly."),
        lines_to_append
            .iter()
            .map(|s| s.as_ref())
            .collect::<Vec<_>>()
            .join("\n"),
        format!(r#"source "{}""#, autocomplete_file.display()),
        format!("# {}", SHELL_CONFIG_IDENTIFYING_PATTERN_END),
        String::from(
            "#############################################################################",
        ),
    ];
    for line in lines {
        // debug!("    {}", line);
        writeln!(f, "{}", line)?;
    }

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
    }

    #[test]
    fn file_contains_failure() {
        let pattern_to_match = "Pattern to find";
        let file_content = format!("Line 1\nLine 2\nDoes not contain pattern\nLine 4");
        assert!(!file_contains(file_content.as_bytes(), &pattern_to_match).unwrap());
    }

    #[test]
    fn append_to_string() {
        let mut writer: Vec<u8> = Vec::new();
        let lines_to_append = vec![String::from("# Line to append")];

        let autocomplete_file = Path::new("foo.sh");

        append_to(&mut writer, &lines_to_append, &autocomplete_file).unwrap();

        let expected = format!(
            r#"
#############################################################################
# {}
# See {}
# WARNING: Those lines _need_ to be at the end of
#          the file: {} needs to appear as soon
#          as possible in the $PATH environment
#          variable to function properly.
# Line to append
source "foo.sh"
# {}
#############################################################################
"#,
            SHELL_CONFIG_IDENTIFYING_PATTERN_START,
            env!("CARGO_PKG_HOMEPAGE"),
            EXECUTABLE_NAME,
            SHELL_CONFIG_IDENTIFYING_PATTERN_END
        );
        let written = String::from_utf8(writer).unwrap();

        assert_eq!(written, expected);
    }
}
