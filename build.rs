use std::fs;
use std::io;

use clap::{Command, CommandFactory};
use clap_complete::{shells, Generator};

include!("src/args.rs");

const COMPLETION_DIR: &str = "completions";

fn gen_completions<G>(gen: G, cmd: &mut Command) -> Result<(), io::Error>
where
    G: Generator,
{
    clap_complete::generate_to(gen, cmd, "tldr", COMPLETION_DIR)?;
    Ok(())
}

fn main() -> Result<(), io::Error> {
    let cmd = &mut Cli::command();

    fs::create_dir_all(COMPLETION_DIR)?;

    gen_completions(shells::Bash, cmd)?;
    gen_completions(shells::Zsh, cmd)?;
    gen_completions(shells::Fish, cmd)?;
    gen_completions(shells::PowerShell, cmd)?;

    Ok(())
}
