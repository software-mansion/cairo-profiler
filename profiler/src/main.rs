use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

use bytes::{Buf, BytesMut};
use flate2::{bufread::GzEncoder, Compression};
use profile_builder::build_profile;
use prost::Message;

mod profile_builder;

#[derive(Clone, Hash, Eq, PartialEq)]
struct FunctionName(String);
#[derive(Clone, Hash, Eq, PartialEq)]
struct Location(Vec<FunctionName>);

impl Location {
    #[inline]
    fn from(s: &[&str]) -> Location {
        Location(s.iter().map(|s| FunctionName(s.to_string())).collect())
    }
}

enum SampleType {
    ContractCall,
}

pub struct Sample {
    location: Location,
    sample_type: SampleType,
}

fn main() {
    let samples = vec![
        Sample {
            location: Location::from(&["A"]),
            sample_type: SampleType::ContractCall,
        },
        Sample {
            location: Location::from(&["A", "B"]),
            sample_type: SampleType::ContractCall,
        },
    ];

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
