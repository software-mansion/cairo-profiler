pub mod perftools {
    pub mod profiles {
        include!(concat!(env!("OUT_DIR"), "/perftools.profiles.rs"));
    }
}

use std::collections::HashMap;

use perftools::profiles as pprof;

use crate::trace_reader::{FunctionName, Location, ResourcesKeys, Sample, SampleType};

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct StringId(pub u64);

#[derive(Clone, Copy, Eq, PartialEq)]
struct LocationId(u64);

#[derive(Clone, Copy, Eq, PartialEq)]
struct FunctionId(u64);

impl From<StringId> for i64 {
    fn from(v: StringId) -> i64 {
        i64::try_from(v.0).unwrap()
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

pub struct ProfilerContext {
    strings: HashMap<String, StringId>,
    id_to_string: HashMap<StringId, String>,
    functions: HashMap<FunctionName, pprof::Function>,
    locations: HashMap<Location, pprof::Location>,
}

impl ProfilerContext {
    fn new() -> Self {
        ProfilerContext {
            strings: vec![(String::new(), StringId(0))].into_iter().collect(),
            id_to_string: vec![(StringId(0), String::new())].into_iter().collect(),
            functions: HashMap::new(),
            locations: HashMap::new(),
        }
    }

    pub fn string_from_string_id(&self, string_id: StringId) -> &str {
        self.id_to_string
            .get(&string_id)
            .unwrap_or_else(|| panic!("String with string id {string_id:?} not found"))
    }

    pub fn string_id(&mut self, string: &String) -> StringId {
        if let Some(id) = self.strings.get(string) {
            *id
        } else {
            let string_id = StringId(self.strings.len() as u64);

            self.strings.insert(string.clone(), string_id);
            self.id_to_string.insert(string_id, string.clone());

            string_id
        }
    }

    fn location_id(&mut self, location: &Location) -> LocationId {
        if let Some(loc) = self.locations.get(location) {
            LocationId(loc.id)
        } else {
            let line = pprof::Line {
                function_id: self.function_id(&location.0).into(),
                line: 0,
            };
            let location_data = pprof::Location {
                id: (self.locations.len() + 1) as u64,
                mapping_id: 0,
                address: 0,
                line: vec![line],
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
    resources_keys: &ResourcesKeys,
) -> (Vec<pprof::ValueType>, Vec<pprof::Sample>) {
    assert!(samples
        .iter()
        .all(|x| matches!(x.sample_type, SampleType::ContractCall)));

    let mut measurement_types = vec![
        pprof::ValueType {
            r#type: context.string_id(&String::from("calls")).into(),
            unit: context.string_id(&String::from(" calls")).into(),
        },
        pprof::ValueType {
            r#type: context.string_id(&String::from("n_steps")).into(),
            unit: context.string_id(&String::from(" steps")).into(),
        },
        pprof::ValueType {
            r#type: context.string_id(&String::from("n_memory_holes")).into(),
            unit: context.string_id(&String::from(" memory holes")).into(),
        },
    ];
    measurement_types.append(&mut resources_keys.measurement_types(context));

    let samples = samples
        .iter()
        .map(|s| pprof::Sample {
            location_id: s
                .locations
                .iter()
                .map(|loc| context.location_id(loc).into())
                .rev() // pprof format represents callstack from the least meaningful element
                .collect(),
            value: s.extract_measurements(&measurement_types, context),
            label: vec![],
        })
        .collect();

    (measurement_types, samples)
}

pub fn build_profile(samples: &[Sample], resources_keys: &ResourcesKeys) -> pprof::Profile {
    let mut context = ProfilerContext::new();
    let (measurement_types, samples) = build_samples(&mut context, samples, resources_keys);
    let (string_table, functions, locations) = context.context_data();

    pprof::Profile {
        sample_type: measurement_types,
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
