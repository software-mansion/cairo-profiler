use crate::cli::build_profile::BuildProfile;
use crate::cli::view::ViewProfile;
use clap::{Parser, Subcommand};

pub(crate) mod build_profile;
pub(crate) mod view;

#[derive(Parser)]
#[command(version, args_conflicts_with_subcommands = true)]
#[clap(name = "cairo-profiler")]
pub struct Cli {
    #[clap(flatten)]
    pub build_profile_args: Option<BuildProfile>,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Build the profile from provided trace data
    BuildProfile(BuildProfile),
    /// View built profile
    View(ViewProfile),
}
