use std::{
    env, fs,
    fs::{create_dir_all, File, OpenOptions},
    io::Write,
};

use structopt::clap::Shell;

use crate::{
    commands,
    utils::{self, directory::PycorsPathsProviderFromEnv},
    Result, EXECUTABLE_NAME, EXTRA_PACKAGES_FILENAME_CONTENT,
};

pub mod bash;

pub fn run(shell: Shell) -> Result<()> {
    log::info!("Setting up the shim...");

    let paths_provider = PycorsPathsProviderFromEnv::new();

    // Create all required directories
    for dir in &[
        paths_provider.cache(),
        paths_provider.installed(),
        paths_provider
            .config_home()
            .join(utils::directory::shell::bash::config::dir_relative()),
        paths_provider.shims(),
    ] {
        if !utils::path_exists(&dir) {
            log::debug!("Directory {:?} does not exists, creating.", dir);
            fs::create_dir_all(&dir)?;
        }
    }

    // Copy itself into ~/.EXECUTABLE_NAME/shim
    let config_home_dir = paths_provider.config_home();
    let shims_dir = paths_provider.shims();
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
    let output_filename = PycorsPathsProviderFromEnv::new().default_extra_package_file();
    log::debug!(
        "Writing list of default packages to install to {:?}",
        output_filename
    );
    let mut file = File::create(output_filename)?;
    file.write_all(extra_packages_file_default_content.as_bytes())?;

    // Add ~/.EXECUTABLE_NAME/shims to $PATH in ~/.bashrc and ~/.bash_profile and install autocomplete
    match shell {
        Shell::Bash => {
            let home =
                dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Error getting home directory"))?;

            bash::setup_bash(&home)?;
        }
        Shell::PowerShell => {
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
