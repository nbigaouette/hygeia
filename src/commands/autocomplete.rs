use structopt::{clap::Shell, StructOpt};

use crate::{Opt, Result};

pub fn print_autocomplete_to_stdout(shell: Shell) -> Result<()> {
    Opt::clap().gen_completions_to("pycors", shell, &mut std::io::stdout());
    Ok(())
}
