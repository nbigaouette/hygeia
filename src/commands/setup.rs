use std::{env, fs, io::Write};

use structopt::clap::Shell;

use crate::{
    constants::{EXECUTABLE_NAME, EXTRA_PACKAGES_FILENAME_CONTENT},
    utils::{
        self,
        directory::{shell::ShellPathProvider, PycorsPathsProviderFromEnv},
    },
    Result,
};

pub mod powershell;
pub mod sh;

pub fn run(shell: Shell) -> Result<()> {
    log::info!("Setting up the shim...");

    let paths_provider = PycorsPathsProviderFromEnv::new();

    let shims_dir = paths_provider.shims();

    // Create all required directories
    for dir in &[
        paths_provider.cache(),
        paths_provider.installed(),
        paths_provider
            .project_home()
            .join(utils::directory::shell::Bash::new().dir_relative()),
        paths_provider
            .project_home()
            .join(utils::directory::shell::Zsh::new().dir_relative()),
        paths_provider
            .project_home()
            .join(utils::directory::shell::Powershell::new().dir_relative()),
        paths_provider.shims(),
    ] {
        if !utils::path_exists(&dir) {
            log::debug!("Directory {:?} does not exists, creating.", dir);
            fs::create_dir_all(&dir)?;
        }
    }

    // Create an dummy file that will be recognized when searching the PATH for
    // python interpreters. We don't want to "find" the shims we install here.
    let mut file = fs::File::create(paths_provider.shims_directory_identifier_file())?;
    writeln!(
        file,
        concat!(
            "This file's job is to tell {} the directory contains shims, not real Python interpreters.\n",
            "Please do not delete!"
        ),
        EXECUTABLE_NAME
    )?;

    // Add ~/.EXECUTABLE_NAME/shims to $PATH in ~/.bashrc and ~/.bash_profile and install autocomplete
    match shell {
        Shell::Bash => sh::setup_shell(&paths_provider, utils::directory::shell::Bash::new()),
        Shell::Zsh => sh::setup_shell(&paths_provider, utils::directory::shell::Zsh::new()),
        Shell::PowerShell => powershell::setup_powershell(&paths_provider),
        _ => anyhow::bail!("Unsupported shell: {}", shell),
    }?;

    // Copy itself into ~/.EXECUTABLE_NAME/shim
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
    let paths_provider = PycorsPathsProviderFromEnv::new();
    let output_filename = paths_provider.default_extra_package_file();
    log::debug!(
        "Writing list of default packages to install to {:?}",
        output_filename
    );

    let mut file = fs::File::create(&output_filename)?;
    file.write_all(extra_packages_file_default_content.as_bytes())?;

    println!();
    println!("🐚 {} successfully configured!", shell);
    println!();
    println!("📦 The file {}", output_filename.display());
    println!("   contains a list of packages that will automatically be \"pip install\"ed");
    println!("   when installing a Python toolchain with");
    println!("   the -e/--extra or -f/--extra-from command line flags.");
    println!(
        "   You can modify this file to your liking. Note that future \"{} setup --extra\" command",
        EXECUTABLE_NAME
    );
    println!(
        "   will overwrite that file. Copy the file and use --extra-from to keep your changes."
    );
    println!();
    println!("🐍 To install a new toolchain, use the 'install' command. For example:");
    println!("      {} install --extra ~3.8", EXECUTABLE_NAME);
    println!("   will install the latest 3.8 release and install the extra pip packages");
    println!("   provided in the file above.");
    println!();
    println!("❔ Run");
    println!("      {} --help", EXECUTABLE_NAME);
    println!("   for more information.");
    println!();

    Ok(())
}
