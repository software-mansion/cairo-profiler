use std::{
    fs,
    io::{Read, Write},
};

use crate::trace_reader::collect_samples_from_trace;
use anyhow::{Context, Result};
use bytes::{Buf, BytesMut};
use cairo_lang_sierra::program::Program;
use cairo_lang_starknet_classes::contract_class::ContractClass;
use camino::Utf8PathBuf;
use clap::Parser;
use flate2::{bufread::GzEncoder, Compression};
use profile_builder::build_profile;
use prost::Message;
use serde_json::Value;
use trace_data::CallTrace;

mod profile_builder;
mod trace_reader;

#[derive(Parser, Debug)]
#[command(version)]
#[clap(name = "cairo-profiler")]
struct Cli {
    /// Path to .json with trace data
    path_to_trace_data: Utf8PathBuf,

    /// Path to the output file
    #[arg(short, long, default_value = "profile.pb.gz")]
    output_path: Utf8PathBuf,

    /// Show contract addresses and function selectors in a trace tree
    #[arg(long)]
    show_details: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let data = fs::read_to_string(cli.path_to_trace_data)
        .context("Failed to read call trace from a file")?;
    let serialized_trace: CallTrace =
        serde_json::from_str(&data).context("Failed to deserialize call trace")?;

    let raw_sierra_code: Value = serde_json::from_str(&fs::read_to_string(
        "target/dev/snforge/uwu.snforge_sierra.json",
    )?)?;
    let sierra_code =
        serde_json::from_str::<Program>(&raw_sierra_code[1]["sierra_program"].to_string())?;
    let mut sierra_contracts = vec![];

    for entry in fs::read_dir("target/dev")? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.is_file() && entry_path.ends_with(".contract_class.json") {
            let raw_sierra = fs::read_to_string(entry_path)?;
            let parsed_sierra = serde_json::from_str::<ContractClass>(&raw_sierra)
                .expect("Failed to parse sierra contract code");
            let sierra_program = parsed_sierra.extract_sierra_program().unwrap();
            sierra_contracts.push(sierra_program);
        }
    }

    let samples = collect_samples_from_trace(
        &serialized_trace,
        cli.show_details,
        &sierra_code,
        &sierra_contracts,
    );

    let profile = build_profile(&samples);

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
