use std::io::Write;

use structopt::{clap::Shell, StructOpt};

use crate::{constants::EXECUTABLE_NAME, Opt, Result};

pub fn run<W>(shell: Shell, buf: &mut W) -> Result<()>
where
    W: Write,
{
    Opt::clap().gen_completions_to(EXECUTABLE_NAME, shell, buf);
    Ok(())
}
