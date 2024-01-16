use std::{path::Path, io::{BufReader, BufWriter, Write, BufRead, Read}, fs::File};

use bytes::{BytesMut, BufMut, Buf};
use flate2::{bufread::GzEncoder, Compression};
use profile_builder::{build_profile, perftools::profiles::Profile};
use prost::Message;

mod profile_builder;


enum SampleType {
    ContractCall
}

#[derive(Clone, Hash, Eq, PartialEq)]
struct FunctionName(String);
#[derive(Clone, Hash, Eq, PartialEq)]
struct Location(Vec<FunctionName>);

impl Location {
    #[inline]
    fn from(s: &Vec<&str>) -> Location {
        Location(s.iter().map(|s| FunctionName(s.to_string())).collect())
    }
}

pub struct Sample {
    location: Location,
    sample_type: SampleType,
    count: u64,
}


fn main() {
    let samples = vec![
        Sample {
            location: Location::from(&vec!["A"]),
            sample_type: SampleType::ContractCall,
            count: 1
        },
        Sample {
            location: Location::from(&vec!["A", "B"]),
            sample_type: SampleType::ContractCall,
            count: 1
        }
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
