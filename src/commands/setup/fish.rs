use std::{fs, process::Command};

use anyhow::Context;
use structopt::{clap::Shell, StructOpt};

use crate::{
    constants::EXECUTABLE_NAME,
    utils::directory::{
        shell::{Fish, ShellPathProvider},
        PycorsHomeProviderTrait, PycorsPathsProvider,
    },
    Opt, Result,
};

pub fn setup_fish<P>(paths_provider: &PycorsPathsProvider<P>) -> Result<()>
where
    P: PycorsHomeProviderTrait,
{
    let autocomplete_file = paths_provider
        .project_home()
        .join(Fish::new().autocomplete());
    let mut f = fs::File::create(&autocomplete_file)
        .with_context(|| format!("Failed creating file {:?}", autocomplete_file))?;
    Opt::clap().gen_completions_to(EXECUTABLE_NAME, Shell::Fish, &mut f);

    Command::new("/usr/bin/fish")
        .arg("-c")
        .arg("fish_add_path")
        .arg(format!("{}/shims", paths_provider.project_home().display()))
        .output()?;

    Ok(())
}
