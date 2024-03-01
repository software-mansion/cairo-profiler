use std::{
    fs,
    io::{Read, Write},
};

use crate::trace_reader::{collect_resources_keys, collect_samples_from_trace};
use anyhow::{Context, Result};
use bytes::{Buf, BytesMut};
use camino::Utf8PathBuf;
use clap::Parser;
use flate2::{bufread::GzEncoder, Compression};
use profile_builder::build_profile;
use prost::Message;
use trace_data::CallTrace;

mod profile_builder;
mod trace_reader;

#[derive(Parser, Debug)]
#[command(version)]
#[clap(name = "cairo-cairo-profiler")]
struct Cli {
    /// Path to .json with trace data
    path_to_trace_data: Utf8PathBuf,

    /// Path to the output file
    #[arg(short, long, default_value = "profile.pb.gz")]
    output_path: Utf8PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let data = fs::read_to_string(cli.path_to_trace_data)
        .context("Failed to read call trace from a file")?;
    let serialized_trace: CallTrace =
        serde_json::from_str(&data).context("Failed to deserialize call trace")?;
    let samples = collect_samples_from_trace(&serialized_trace);
    let resources_keys = collect_resources_keys(&samples);

    let profile = build_profile(&samples, &resources_keys);

    if let Some(parent) = cli.output_path.parent() {
        fs::create_dir_all(parent)
            .context("Failed to create parent directories for the output file")?;
    }
    let mut file = fs::File::create(cli.output_path).context("Failed to create the output file")?;

    let mut buffer = BytesMut::new();
    profile
        .encode(&mut buffer)
        .expect("Failed to encode the profile to the buffer");

    let mut buffer_reader = buffer.reader();
    let mut encoder = GzEncoder::new(&mut buffer_reader, Compression::default());

    let mut encoded = vec![];
    encoder
        .read_to_end(&mut encoded)
        .context("Failed to read bytes from the encoder")?;
    file.write_all(&encoded).unwrap();

    Ok(())
}
