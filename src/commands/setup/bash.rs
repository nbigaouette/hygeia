use std::{
    fs,
    io::{BufRead, BufReader, Write},
    path::Path,
};

use structopt::clap::Shell;

use crate::{commands, constants::EXECUTABLE_NAME, utils, Result};

pub fn setup_bash(home: &Path, config_home_dir: &Path, shims_dir: &Path) -> Result<()> {
    let bash_config_files = &[home.join(".bashrc"), home.join(".bash_profile")];

    // Add the autocomplete too
    let autocomplete_file = config_home_dir.join(&format!("{}.bash-completion", EXECUTABLE_NAME));
    let mut f = fs::File::create(&autocomplete_file)?;
    commands::autocomplete::run(Shell::Bash, &mut f)?;

    let lines_to_append: Vec<String> = vec![
        format!(
            r#"export PYCORS_HOME="{}""#,
            utils::directory::config_home()?.display()
        ),
        String::from(r#"# Add the shims directory to path, removing all other"#),
        String::from(r#"# occurrences of it from current $PATH."#),
        String::from(r#"if [ -z ${PYCORS_INITIALIZED+x} ]; then"#),
        String::from(r#"    # Setup pycors: prepends the shims directory to PATH"#),
        String::from(r#"    export PATH="${PYCORS_HOME}/shims:${PATH//${PYCORS_HOME}/}""#),
        String::from(r#"    export PYCORS_INITIALIZED=1"#),
        String::from(r#"else"#),
        String::from(r#"    # Shell already setup for pycors."#),
        String::from(r#"    # Disable in case we enter a 'poetry shell'"#),
        String::from(r#"    if [ -z ${POETRY_ACTIVE+x} ]; then"#),
        String::from(r#"        # Not in a 'poetry shell', activating."#),
        String::from(r#"        export PATH="${PYCORS_HOME}/shims:${PATH//${PYCORS_HOME}/}""#),
        String::from(r#"    else"#),
        String::from(r#"        # Poetry is active; disable the shim"#),
        String::from(
            r#"        echo "Pycors detected an active poetry shell, disabling the shim.""#,
        ),
        String::from(r#"        export PATH="${PATH//${PYCORS_HOME}/}""#),
        String::from(r#"    fi"#),
        String::from(r#"fi"#),
        String::from(r#"source "${PYCORS_HOME}/pycors.bash-completion""#),
        String::from(r#"source "/Users/nbigaouette/.pycors/pycors.bash-completion""#),
    ];

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
            let f = fs::File::open(&bash_config_file)?;
            let f = BufReader::new(f);
            !file_contains(f, &lines_to_append[0])?
        };

        if do_edit_file {
            let mut file = fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(&bash_config_file)?;
            let lines = &[
                String::from(""),
                String::from("#################################################"),
                format!("# These lines were added by {}.", EXECUTABLE_NAME),
                format!("# See {}", env!("CARGO_PKG_HOMEPAGE")),
                format!("# WARNING: Those lines _need_ to be at the end of"),
                format!("#          the file: pycors needs to appear as soon"),
                format!("#          as possible in the $PATH environment"),
                format!("#          variable to function properly."),
                lines_to_append.join("\n"),
                format!(r#"source "{}""#, autocomplete_file.display()),
                String::from("#################################################"),
            ];
            for line in lines {
                // debug!("    {}", line);
                writeln!(file, "{}", line)?;
            }
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
