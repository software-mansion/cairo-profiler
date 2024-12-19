use camino::Utf8PathBuf;
use clap::Args;

#[derive(Args)]
pub struct ViewProfile {
    /// Path to .pb.gz file with profile data.
    pub path_to_profile: Utf8PathBuf,

    /// Show the sample in the top view.
    /// To get the list of available samples use `--list-samples`.
    #[arg(long, default_value = "steps")]
    pub sample: String,

    /// List all the samples included in the profile.
    #[arg(short, long)]
    pub list_samples: bool,

    /// Set a limit of nodes showed in the top view.
    #[arg(long, default_value = "10")]
    pub limit: usize,
}
