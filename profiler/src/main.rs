use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

use bytes::{Buf, BytesMut};
use flate2::{bufread::GzEncoder, Compression};
use profile_builder::build_profile;
use prost::Message;
use trace_data::CallTrace;
use trace_reader::collect_samples_from_trace;

mod profile_builder;
mod trace_data;
mod trace_reader;

fn main() {
    let trace_path = Path::new("./trace");

    let data = fs::read_to_string(trace_path).expect("Failed to write call trace to a file");
    let serialized_trace: CallTrace =
        serde_json::from_str(&data).expect("Failed to read call trace");

    let samples = collect_samples_from_trace(&serialized_trace);

    let profile = build_profile(samples);

    let path = Path::new("profile.pb.gz");
    let mut file = File::create(path).unwrap();

    let mut buffer = BytesMut::new();
    profile.encode(&mut buffer).unwrap();

    let mut buffer_reader = buffer.reader();
    let mut encoder = GzEncoder::new(&mut buffer_reader, Compression::default());

    let mut encoded = vec![];
    encoder.read_to_end(&mut encoded).unwrap();
    file.write_all(&encoded).unwrap();
}
