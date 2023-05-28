use std::env;
use std::fs;
use std::io;

use clap::{Command, CommandFactory};
use clap_complete::{shells, Generator};

include!("src/args.rs");

fn gen_completions<G>(gen: G, cmd: &mut Command, dir: &str) -> Result<(), io::Error>
where
    G: Generator,
{
    clap_complete::generate_to(gen, cmd, "tldr", dir)?;
    Ok(())
}

fn main() -> Result<(), io::Error> {
    let completion_dir = match env::var("COMPLETION_DIR").ok() {
        Some(val) => val,
        None => return Ok(()),
    };
    let cmd = &mut Cli::command();

    fs::create_dir_all(&completion_dir)?;

    gen_completions(shells::Bash, cmd, &completion_dir)?;
    gen_completions(shells::Zsh, cmd, &completion_dir)?;
    gen_completions(shells::Fish, cmd, &completion_dir)?;
    gen_completions(shells::PowerShell, cmd, &completion_dir)?;

    Ok(())
}
