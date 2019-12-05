use std::{
    env, fs,
    fs::{create_dir_all, File, OpenOptions},
    io::{BufRead, BufReader, Write},
};

use anyhow::{anyhow, Result};
use structopt::clap::Shell;

use crate::{commands, utils, EXECUTABLE_NAME, EXTRA_PACKAGES_FILENAME_CONTENT};

pub fn run(shell: Shell) -> Result<()> {
    log::info!("Setting up the shim...");

    // Copy itself into ~/.EXECUTABLE_NAME/shim
    let config_home_dir = utils::directory::config_home()?;
    let shims_dir = utils::directory::shims()?;
    if !utils::path_exists(&shims_dir) {
        log::debug!("Directory {:?} does not exists, creating.", shims_dir);
        fs::create_dir_all(&shims_dir)?;
    }
    let copy_from = env::current_exe()?;
    let copy_to = {
        #[cfg_attr(not(windows), allow(unused_mut))]
        let mut tmp = shims_dir.join(EXECUTABLE_NAME);

        #[cfg(windows)]
        tmp.set_extension("exe");

        tmp
    };
    log::debug!("Copying {:?} into {:?}...", copy_from, copy_to);
    utils::copy_file(&copy_from, &copy_to)?;

    #[cfg(windows)]
    let bin_extension = ".exe";
    #[cfg(not(windows))]
    let bin_extension = "";

    // Once the shim is in place, create hard links to it.
    let hardlinks_version_suffix = &[
        format!("python###{}", bin_extension),
        format!("idle###{}", bin_extension),
        format!("pip###{}", bin_extension),
        format!("pydoc###{}", bin_extension),
        // Internals
        format!("python###-config{}", bin_extension),
        format!("python###dm-config{}", bin_extension),
        // Extras
        format!("pipenv###{}", bin_extension),
        format!("poetry###{}", bin_extension),
        format!("pytest###{}", bin_extension),
    ];
    let hardlinks_dash_version_suffix = &[
        format!("2to3###{}", bin_extension),
        format!("easy_install###{}", bin_extension),
        format!("pyvenv###{}", bin_extension),
    ];

    // Create simple hardlinks: `EXECUTABLE_NAME` --> `bin`
    utils::create_hard_links(&copy_to, hardlinks_version_suffix, &shims_dir, "")?;
    utils::create_hard_links(&copy_to, hardlinks_dash_version_suffix, &shims_dir, "")?;

    // Create major version hardlinks: `EXECUTABLE_NAME` --> `bin3` and `EXECUTABLE_NAME` --> `bin2`
    for major in &["2", "3"] {
        utils::create_hard_links(&copy_to, hardlinks_version_suffix, &shims_dir, major)?;
        utils::create_hard_links(
            &copy_to,
            hardlinks_dash_version_suffix,
            &shims_dir,
            &format!("-{}", major),
        )?;
    }

    let extra_packages_file_default_content = EXTRA_PACKAGES_FILENAME_CONTENT;
    let output_filename = utils::default_extra_package_file()?;
    log::debug!(
        "Writing list of default packages to install to {:?}",
        output_filename
    );
    let mut file = File::create(output_filename)?;
    file.write_all(extra_packages_file_default_content.as_bytes())?;

    // Add ~/.EXECUTABLE_NAME/shims to $PATH in ~/.bashrc and ~/.bash_profile and install autocomplete
    match shell {
        structopt::clap::Shell::Bash => {
            let home = dirs::home_dir().ok_or_else(|| anyhow!("Error getting home directory"))?;

            let bash_config_files = &[home.join(".bashrc"), home.join(".bash_profile")];

            // Add the autocomplete too
            let autocomplete_file =
                config_home_dir.join(&format!("{}.bash-completion", EXECUTABLE_NAME));
            let mut f = fs::File::create(&autocomplete_file)?;
            commands::autocomplete::run(shell, &mut f)?;

            let lines_to_append: Vec<String> = vec![
                format!(
                    r#"export PYCORS_HOME="{}""#,
                    utils::directory::config_home()?.display()
                ),
                String::from(r#"# Add the shims directory to path, removing all other"#),
                String::from(r#"# occurrences of it from current $PATH."#),
                String::from(r#"if [ -z ${PYCORS_INITIALIZED+x} ]; then"#),
                String::from(r#"    # Setup pycors: prepend the shims directory to PATH"#),
                String::from(r#"    export PATH="${PYCORS_HOME}/shims:${PATH//${PYCORS_HOME}/}""#),
                String::from(r#"    export PYCORS_INITIALIZED=1"#),
                String::from(r#"else"#),
                String::from(r#"    # Shell already setup for pycors."#),
                String::from(r#"    # Disable in case we enter a 'poetry shell'"#),
                String::from(r#"    if [ -z ${POETRY_ACTIVE+x} ]; then"#),
                String::from(r#"        # Not in a 'poetry shell', activating."#),
                String::from(
                    r#"        export PATH="${PYCORS_HOME}/shims:${PATH//${PYCORS_HOME}/}""#,
                ),
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
                    let mut line_found = false;
                    for line in f.lines() {
                        match line {
                            Err(e) => log::error!(
                                "Failed to read line from file {:?}: {:?}",
                                bash_config_file,
                                e,
                            ),
                            Ok(line) => {
                                if line == lines_to_append[0] {
                                    log::debug!(
                                        "File {:?} already contains pycors setup. Skipping.",
                                        bash_config_file
                                    );
                                    line_found = true;
                                    break;
                                }
                            }
                        }
                    }

                    !line_found
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
        }
        structopt::clap::Shell::PowerShell => {
            // Add the autocomplete too
            let autocomplete_file = config_home_dir.join(&format!("_{}.ps1", EXECUTABLE_NAME));
            let mut f = fs::File::create(&autocomplete_file)?;
            commands::autocomplete::run(shell, &mut f)?;

            match dirs::document_dir() {
                None => {
                    anyhow::bail!("Could not get Document directory");
                }
                Some(document_dir) => {
                    let ps_dir = document_dir.join("WindowsPowerShell");
                    if !ps_dir.exists() {
                        create_dir_all(&ps_dir)?;
                    }
                    // Should match the value of PowerShell's '$profile' automatic variable
                    let profile = ps_dir.join("Microsoft.PowerShell_profile.ps1");

                    let mut file = OpenOptions::new()
                        .create(true)
                        .write(true)
                        .append(true)
                        .open(&profile)?;
                    // FIXME: This appends, we want prepends
                    writeln!(file, r#"$env:Path += ";{}""#, shims_dir.display())?;
                    writeln!(file, ". {}", autocomplete_file.display())?;
                }
            }
        }
        _ => anyhow::bail!("Unsupported shell: {}", shell),
    }

    log::info!("Done!");
    Ok(())
}
