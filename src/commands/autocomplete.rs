use std::io::Write;

use structopt::{clap::Shell, StructOpt};

use crate::{Opt, Result, EXECUTABLE_NAME};

pub fn run<W>(shell: Shell, buf: &mut W) -> Result<()>
where
    W: Write,
{
    Opt::clap().gen_completions_to(EXECUTABLE_NAME, shell, buf);
    Ok(())
}
