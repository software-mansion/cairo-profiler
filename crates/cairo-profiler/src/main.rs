use crate::cli::build_profile::run_build_profile;
use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

mod cli;
mod profile_builder;
mod profiler_config;
mod sierra_loader;
mod trace_reader;
mod versioned_constants_reader;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::BuildProfile(build_cli)) => run_build_profile(&build_cli),
        None => run_build_profile(&cli.build_profile_args.expect("Failed to parse arguments")),
        Some(_) => todo!("new subcommands will be added here"),
    }
}
