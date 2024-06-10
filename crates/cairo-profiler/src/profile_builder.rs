mod perftools {
    #[allow(clippy::doc_link_with_quotes)]
    #[allow(clippy::doc_markdown)]
    pub mod profiles {
        include!(concat!(env!("OUT_DIR"), "/perftools.profiles.rs"));
    }
}

use std::collections::{HashMap, HashSet};

pub use perftools::profiles as pprof;

use crate::trace_reader::function_name::FunctionName;
use crate::trace_reader::{Function, InternalFunction, MeasurementUnit, MeasurementValue, Sample};

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
struct StringId(u64);

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

struct ProfilerContext {
    strings: HashMap<String, StringId>,
    functions: HashMap<FunctionName, pprof::Function>,
    locations: HashMap<FunctionName, pprof::Location>,
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
            let string_id = StringId(self.strings.len() as u64);

            self.strings.insert(string.clone(), string_id);
            string_id
        }
    }

    fn location_id(&mut self, location: &Function) -> LocationId {
        let location = match location {
            Function::Entrypoint(function_name)
            | Function::InternalFunction(InternalFunction::NonInlined(function_name)) => {
                function_name
            }
            Function::InternalFunction(InternalFunction::_Inlined(_)) => {
                todo!("Unused, logic for it will be added in the next PR")
            }
        };

        if let Some(loc) = self.locations.get(location) {
            LocationId(loc.id)
        } else {
            let line = pprof::Line {
                function_id: self.function_id(location).into(),
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

fn build_value_types(
    measurements_units: &Vec<MeasurementUnit>,
    context: &mut ProfilerContext,
) -> Vec<pprof::ValueType> {
    let mut value_types = vec![];

    for unit in measurements_units {
        let unit_without_underscores = unit.0.replace('_', " ");
        let unit_without_prefix = if unit_without_underscores.starts_with("n ") {
            unit_without_underscores.strip_prefix("n ").unwrap()
        } else {
            &unit_without_underscores
        };
        let unit_string = format!(" {unit_without_prefix}");

        value_types.push(pprof::ValueType {
            r#type: context.string_id(&unit.0).into(),
            unit: context.string_id(&unit_string).into(),
        });
    }
    value_types
}

fn build_samples(
    context: &mut ProfilerContext,
    samples: &[Sample],
    all_measurements_units: &[MeasurementUnit],
) -> Vec<pprof::Sample> {
    let samples = samples
        .iter()
        .map(|s| pprof::Sample {
            location_id: s
                .call_stack
                .iter()
                .map(|loc| context.location_id(loc).into())
                .rev() // pprof format represents callstack from the least meaningful element
                .collect(),
            value: all_measurements_units
                .iter()
                .map(|un| {
                    s.measurements
                        .get(un)
                        .cloned()
                        .unwrap_or(MeasurementValue(0))
                        .0
                })
                .collect(),
            label: vec![],
        })
        .collect();

    samples
}

fn collect_all_measurements_units(samples: &[Sample]) -> Vec<MeasurementUnit> {
    let units_set: HashSet<&MeasurementUnit> =
        samples.iter().flat_map(|m| m.measurements.keys()).collect();
    units_set.into_iter().cloned().collect()
}

pub fn build_profile(samples: &[Sample]) -> pprof::Profile {
    let mut context = ProfilerContext::new();
    let all_measurements_units = collect_all_measurements_units(samples);
    let value_types = build_value_types(&all_measurements_units, &mut context);
    let pprof_samples = build_samples(&mut context, samples, &all_measurements_units);
    let (string_table, functions, locations) = context.context_data();

    pprof::Profile {
        sample_type: value_types,
        sample: pprof_samples,
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
