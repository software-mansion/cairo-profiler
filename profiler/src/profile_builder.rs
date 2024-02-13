pub mod perftools {
    pub mod profiles {
        include!(concat!(env!("OUT_DIR"), "/perftools.profiles.rs"));
    }
}

use std::collections::HashMap;

use perftools::profiles as pprof;

use crate::trace_reader::{FunctionName, Location, Sample, SampleType};

#[derive(Clone, Copy)]
struct StringId(u64);

#[derive(Clone, Copy, Eq, PartialEq)]
struct LocationId(u64);

#[derive(Clone, Copy, Eq, PartialEq)]
struct FunctionId(u64);

impl From<StringId> for i64 {
    fn from(v: StringId) -> i64 {
        v.0 as i64
    }
}

impl From<LocationId> for u64 {
    fn from(v: LocationId) -> u64 {
        v.0
    }
}

impl From<FunctionId> for u64 {
    fn from(v: FunctionId) -> u64 {
        v.0
    }
}

struct ProfilerContext {
    strings: HashMap<String, StringId>,
    functions: HashMap<FunctionName, pprof::Function>,
    locations: HashMap<Location, pprof::Location>,
}

impl ProfilerContext {
    fn new() -> Self {
        ProfilerContext {
            strings: vec![(String::new(), StringId(0))].into_iter().collect(),
            functions: HashMap::new(),
            locations: HashMap::new(),
        }
    }

    fn string_id(&mut self, string: &String) -> StringId {
        if let Some(id) = self.strings.get(string) {
            *id
        } else {
            self.strings
                .insert(string.clone(), StringId(self.strings.len() as u64));
            StringId((self.strings.len() - 1) as u64)
        }
    }

    fn build_line(&mut self, location: &Location) -> Vec<pprof::Line> {
        location
            .0
            .iter()
            .map(|f_name| pprof::Line {
                function_id: self.function_id(f_name).0,
                line: 0,
            })
            .collect()
    }

    fn location_id(&mut self, location: &Location) -> LocationId {
        if let Some(loc) = self.locations.get(location) {
            LocationId(loc.id)
        } else {
            let mut line = self.build_line(location);
            line.reverse();
            let location_data = pprof::Location {
                id: (self.locations.len() + 1) as u64,
                mapping_id: 0,
                address: 0,
                // pprof format represents callstack from the least meaningful element
                line,
                is_folded: true,
            };
            self.locations.insert(location.clone(), location_data);
            LocationId(self.locations.len() as u64)
        }
    }

    fn function_id(&mut self, function_name: &FunctionName) -> FunctionId {
        if let Some(f) = self.functions.get(function_name) {
            FunctionId(f.id)
        } else {
            let function_data = pprof::Function {
                id: (self.functions.len() + 1) as u64,
                name: self.string_id(&function_name.0).into(),
                system_name: self.string_id(&"system".to_string()).into(),
                filename: self.string_id(&"global".to_string()).into(),
                start_line: 0,
            };
            self.functions.insert(function_name.clone(), function_data);
            FunctionId(self.functions.len() as u64)
        }
    }

    fn context_data(self) -> (Vec<String>, Vec<pprof::Function>, Vec<pprof::Location>) {
        let mut string_table: Vec<String> = self.strings.clone().into_keys().collect();
        for (st, id) in self.strings {
            string_table[usize::try_from(id.0).unwrap()] = st;
        }

        let functions = self.functions.into_values().collect();

        let locations = self.locations.into_values().collect();

        (string_table, functions, locations)
    }
}

fn build_samples(
    context: &mut ProfilerContext,
    samples: &[Sample],
) -> (Vec<pprof::ValueType>, Vec<pprof::Sample>) {
    assert!(samples
        .iter()
        .all(|x| matches!(x.sample_type, SampleType::ContractCall)));

    let sample_types = vec![pprof::ValueType {
        r#type: context.string_id(&String::from("calls")).into(),
        unit: context.string_id(&String::new()).into(),
    }];
    let samples = samples
        .iter()
        .map(|s| pprof::Sample {
            location_id: vec![context.location_id(&s.location).into()],
            value: vec![1],
            label: vec![],
        })
        .collect();
    (sample_types, samples)
}

pub fn build_profile(samples: &[Sample]) -> pprof::Profile {
    let mut context = ProfilerContext::new();
    let (sample_types, samples) = build_samples(&mut context, samples);
    let (string_table, functions, locations) = context.context_data();

    pprof::Profile {
        sample_type: sample_types,
        sample: samples,
        mapping: vec![],
        location: locations,
        function: functions,
        string_table,
        drop_frames: 0,
        keep_frames: 0,
        time_nanos: 0,
        duration_nanos: 0,
        period_type: None,
        period: 0,
        comment: vec![],
        default_sample_type: 0,
    }
}
