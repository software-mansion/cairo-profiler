mod perftools {
    #[expect(clippy::doc_link_with_quotes)]
    #[expect(clippy::doc_markdown)]
    pub mod profiles {
        include!(concat!(env!("OUT_DIR"), "/perftools.profiles.rs"));
    }
}

use anyhow::{Context, Result};
use bytes::{Buf, BytesMut};
use camino::Utf8PathBuf;
use flate2::{Compression, bufread::GzEncoder};
use prost::Message;
use std::collections::{HashMap, HashSet};
use std::{fs, io::Read};

pub use perftools::profiles as pprof;

use crate::trace_reader::sample::{
    FunctionCall, InternalFunctionCall, MeasurementUnit, MeasurementValue, Sample,
};
use cairo_annotations::annotations::profiler::FunctionName;

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
    locations: HashMap<Vec<FunctionCall>, pprof::Location>,
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

    // This function aggregates function calls and extracts locations from them.
    // Aggregation is done because locations with multiple lines are the only way to tell pprof
    // about inlined functions (each line after the first one symbolizes inlined function).
    // E.g. a sample with stack [A, B_inlined, C, D, E_inlined, F_inlined] has 3 pprof locations
    // (instead of 6 if all function calls were not inlined) corresponding to the following stacks:
    // [A, B_inlined], [C] and [D, E_inlined, F_inlined].
    fn locations_ids(&mut self, call_stack: &[FunctionCall]) -> Vec<LocationId> {
        let mut locations_ids = vec![];

        // This vector represent stacks of functions corresponding to single locations.
        // It contains tuples of form (start_index, end_index).
        // A single stack is `&call_stack[start_index..=end_index]`.
        let mut function_stacks_indexes = vec![];

        let mut current_function_stack_start_index = 0;
        for (index, function) in call_stack.iter().enumerate() {
            match function {
                FunctionCall::InternalFunctionCall(
                    InternalFunctionCall::NonInlined(_)
                    | InternalFunctionCall::Syscall(_)
                    | InternalFunctionCall::Libfunc(_),
                )
                | FunctionCall::EntrypointCall(_) => {
                    if index != 0 {
                        function_stacks_indexes
                            .push((current_function_stack_start_index, index - 1));
                    }
                    current_function_stack_start_index = index;
                }
                FunctionCall::InternalFunctionCall(InternalFunctionCall::Inlined(_)) => {}
            }
        }
        function_stacks_indexes.push((current_function_stack_start_index, call_stack.len() - 1));

        for (start_index, end_index) in function_stacks_indexes {
            let function_stack = &call_stack[start_index..=end_index];
            if let Some(location) = self.locations.get(function_stack) {
                locations_ids.push(LocationId(location.id));
            } else {
                let mut location = match &function_stack[0] {
                    FunctionCall::EntrypointCall(function_name)
                    | FunctionCall::InternalFunctionCall(
                        InternalFunctionCall::NonInlined(function_name)
                        | InternalFunctionCall::Syscall(function_name)
                        | InternalFunctionCall::Libfunc(function_name),
                    ) => {
                        let line = pprof::Line {
                            function_id: self.function_id(function_name).into(),
                            line: 0,
                        };
                        pprof::Location {
                            id: (self.locations.len() + 1) as u64,
                            mapping_id: 0,
                            address: 0,
                            line: vec![line],
                            is_folded: true,
                        }
                    }
                    FunctionCall::InternalFunctionCall(InternalFunctionCall::Inlined(_)) => {
                        unreachable!(
                            "First function in a function stack corresponding to a single location cannot be inlined"
                        )
                    }
                };

                for function in function_stack.get(1..).unwrap_or_default() {
                    match function {
                        FunctionCall::InternalFunctionCall(InternalFunctionCall::Inlined(
                            function_name,
                        )) => {
                            let line = pprof::Line {
                                function_id: self.function_id(function_name).into(),
                                line: 0,
                            };
                            location.line.push(line);
                        }
                        FunctionCall::EntrypointCall(_)
                        | FunctionCall::InternalFunctionCall(
                            InternalFunctionCall::NonInlined(_)
                            | InternalFunctionCall::Syscall(_)
                            | InternalFunctionCall::Libfunc(_),
                        ) => {
                            unreachable!(
                                "Only first function in a function stack corresponding to a single location can be not inlined"
                            )
                        }
                    }
                }

                // pprof format represents callstack from the least meaningful elements
                location.line.reverse();
                locations_ids.push(LocationId(location.id));

                self.locations.insert(function_stack.to_vec(), location);
            }
        }

        locations_ids
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
    samples
        .iter()
        .map(|s| pprof::Sample {
            location_id: context
                .locations_ids(&s.call_stack)
                .into_iter()
                .map(Into::into)
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
        .collect()
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

pub fn save_profile(target_path: &Utf8PathBuf, profile: &pprof::Profile) -> Result<()> {
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .context("Failed to create parent directories for the output file")?;
    }

    let mut buffer = BytesMut::new();
    profile
        .encode(&mut buffer)
        .expect("Failed to encode the profile to the buffer");

    let mut buffer_reader = buffer.reader();
    let mut encoder = GzEncoder::new(&mut buffer_reader, Compression::default());

    let mut encoded_buffer = vec![];
    encoder
        .read_to_end(&mut encoded_buffer)
        .context("Failed to read bytes from the encoder")?;
    fs::write(target_path, &encoded_buffer)?;

    Ok(())
}
