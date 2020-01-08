use std::{fs, io::Write};

use anyhow::Context;
use structopt::{clap::Shell, StructOpt};

use crate::{
    constants::EXECUTABLE_NAME,
    utils::{
        self,
        directory::{PycorsHomeProviderTrait, PycorsPathsProvider},
    },
    Opt, Result,
};

pub fn setup_powershell<P>(paths_provider: &PycorsPathsProvider<P>) -> Result<()>
where
    P: PycorsHomeProviderTrait,
{
    let project_home = paths_provider.project_home();
    if !project_home.exists() {
        fs::create_dir_all(&project_home)
            .with_context(|| format!("Failed to create directory {:?}", project_home))?;
    }

    let autocomplete_file =
        project_home.join(utils::directory::shell::powershell::config::autocomplete());
    let mut f = fs::File::create(&autocomplete_file)
        .with_context(|| format!("Failed to create file {:?}", autocomplete_file))?;
    Opt::clap().gen_completions_to(EXECUTABLE_NAME, Shell::PowerShell, &mut f);

    match dirs::document_dir() {
        None => {
            anyhow::bail!("Could not get Document directory");
        }
        Some(document_dir) => {
            let ps_dir = document_dir.join("WindowsPowerShell");
            if !ps_dir.exists() {
                fs::create_dir_all(&ps_dir)
                    .with_context(|| format!("Failed to create directory {:?}", ps_dir))?;
            }
            // Should match the value of PowerShell's '$profile' automatic variable
            let profile = ps_dir.join("Microsoft.PowerShell_profile.ps1");

            log::info!("Adding configuration to file {}", profile.display());

            let mut file = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .append(true)
                .open(&profile)
                .with_context(|| format!("Failed to open file {}", profile.display()))?;
            // FIXME: This appends, we want prepends
            let line = format!(r#"$env:Path += ";{}""#, paths_provider.shims().display());
            writeln!(file, "{}", line).with_context(|| {
                format!(
                    "Failed to write line {:?} to file {}",
                    line,
                    profile.display()
                )
            })?;
            let line = format!(". {}", autocomplete_file.display());
            writeln!(file, "{}", line).with_context(|| {
                format!(
                    "Failed to write line {:?} to file {}",
                    line,
                    profile.display()
                )
            })?;
        }
    }

    Ok(())
}
