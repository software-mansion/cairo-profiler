use std::{
    fs,
    io::{Read, Write},
    path::Path,
};

use bytes::{Buf, BytesMut};
use camino::Utf8PathBuf;
use clap::Parser;
use flate2::{bufread::GzEncoder, Compression};
use profile_builder::build_profile;
use prost::Message;
use trace_data::CallTrace;
use trace_reader::collect_samples_from_trace;

mod profile_builder;
mod trace_data;
mod trace_reader;

#[derive(Parser, Debug)]
#[command(version)]
#[clap(name = "cairo-profiler")]
struct Cli {
    /// Path to .json with trace data
    path_to_trace_data: Utf8PathBuf,
}

fn main() {
    let cli = Cli::parse();

    let data =
        fs::read_to_string(cli.path_to_trace_data).expect("Failed to read call trace from a file");
    let serialized_trace: CallTrace =
        serde_json::from_str(&data).expect("Failed to deserialize call trace");

    let samples = collect_samples_from_trace(&serialized_trace);

    let profile = build_profile(samples);

    let path = Path::new("profile.pb.gz");
    let mut file = fs::File::create(path).unwrap();

    let mut buffer = BytesMut::new();
    profile.encode(&mut buffer).unwrap();

    let mut buffer_reader = buffer.reader();
    let mut encoder = GzEncoder::new(&mut buffer_reader, Compression::default());

    let mut encoded = vec![];
    encoder.read_to_end(&mut encoded).unwrap();
    file.write_all(&encoded).unwrap();
}
