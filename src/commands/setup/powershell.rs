use std::{fs, io::Write, path::PathBuf};

use anyhow::Context;
use structopt::{clap::Shell, StructOpt};

use crate::{
    constants::EXECUTABLE_NAME,
    utils::directory::{
        shell::{Powershell, ShellPathProvider},
        PycorsHomeProviderTrait, PycorsPathsProvider,
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

    let autocomplete_file = project_home.join(Powershell::new().autocomplete());
    let mut f = fs::File::create(&autocomplete_file)
        .with_context(|| format!("Failed to create file {:?}", autocomplete_file))?;
    Opt::clap().gen_completions_to(EXECUTABLE_NAME, Shell::PowerShell, &mut f);

    // Lets run powershell to get the '$profile' variable value
    // On macOS, it resolves to '${HOME}/.config/powershell/Microsoft.PowerShell_profile.ps1'
    // while on Windows it's _either_ one of these:
    //      '${HOME}\Documents\WindowsPowerShell\Microsoft.PowerShell_profile.ps1'
    //      '${HOME}\Documents\PowerShell\Microsoft.PowerShell_profile.ps1'
    // Let's call powershell directly to get the proper value.
    let powershell_commands = &["pwsh", "powershell"];
    let powershell_args = &["-Command", "'echo $profile'"];
    let profile: PathBuf = match powershell_commands
        .iter()
        .filter_map(|powershell_command| {
            match std::process::Command::new(powershell_command)
                .args(powershell_args)
                .output()
            {
                Ok(output) => {
                    // FIXME: This assumes paths are UTF-8, that's not true :(
                    Some(String::from_utf8_lossy(&output.stdout).to_string().into())
                }
                Err(err) => {
                    log::warn!(
                        "Command '{}' could not be run: {:?}",
                        powershell_command,
                        err
                    );
                    None
                }
            }
        })
        .next()
    {
        Some(profile) => profile,
        None => {
            log::warn!("We failed to run powershell to extract its '$profile' variable.");
            log::warn!("Falling back to building custom value...");
            // We failed to run powershell to extract its '$profile' variable.
            // Let's fallback to building the path ourselves. Note that
            // some systems use
            match paths_provider.document() {
                None => {
                    anyhow::bail!("Could not get Document directory");
                }
                Some(document_dir) => {
                    // WARNING: Sometimes it's 'WindowsPowerShell', sometimes it's 'PowerShell'...
                    let ps_dir = document_dir.join("WindowsPowerShell");
                    if !ps_dir.exists() {
                        fs::create_dir_all(&ps_dir)
                            .with_context(|| format!("Failed to create directory {:?}", ps_dir))?;
                    }
                    ps_dir.join("Microsoft.PowerShell_profile.ps1")
                }
            }
        }
    };

    log::info!("Adding configuration to file {}", profile.display());

    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(&profile)
        .with_context(|| format!("Failed to open file {}", profile.display()))?;
    // FIXME: This appends, we want prepends
    let line = format!(r#"$env:PATH += ";{}""#, paths_provider.shims().display());
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

    Ok(())
}
