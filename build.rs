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
    let completion_dir = env::var("COMPLETION_DIR")
        .or_else(|_| env::var("OUT_DIR"))
        .unwrap();

    fs::create_dir_all(&completion_dir)?;

    let cmd = &mut Cli::command();
    gen_completions(shells::Bash, cmd, &completion_dir)?;
    gen_completions(shells::Zsh, cmd, &completion_dir)?;
    gen_completions(shells::Fish, cmd, &completion_dir)?;
    gen_completions(shells::PowerShell, cmd, &completion_dir)?;

    Ok(())
}
