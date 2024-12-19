use crate::profile_viewer::{get_samples, load_profile, print_profile};
use anyhow::Result;
use camino::Utf8PathBuf;
use clap::Args;
use std::num::NonZeroUsize;

#[derive(Args)]
pub struct ViewProfile {
    /// Path to .pb.gz file with profile data.
    pub path_to_profile: Utf8PathBuf,

    /// Show the sample in the top view.
    /// To get the list of available samples use `--list-samples`.
    #[arg(long, default_value = "steps", conflicts_with = "list_samples")]
    pub sample: String,

    /// List all the samples included in the profile.
    #[arg(short, long)]
    pub list_samples: bool,

    /// Set a limit of nodes showed in the top view.
    #[arg(long, default_value = "10", conflicts_with = "list_samples")]
    pub limit: NonZeroUsize,
}

pub fn run_view(args: &ViewProfile) -> Result<()> {
    let profile = load_profile(&args.path_to_profile)?;
    if args.list_samples {
        let samples = get_samples(&profile);
        println!("{}", samples.join("\n"));
        return Ok(());
    }
    print_profile(&profile, &args.sample, args.limit)?;
    Ok(())
}
