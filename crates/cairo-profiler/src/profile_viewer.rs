use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use flate2::read::GzDecoder;
use prettytable::{Table, format};
use prost::Message;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::num::NonZeroUsize;

use crate::profile_builder::pprof::{Function, Location, Profile};

#[derive(Debug, Default)]
struct FunctionProfile {
    flat: i64,
    flat_p: f64,
    cumulative: i64,
    cumulative_p: f64,
    sum_p: f64,
}

// we only care about two decimal places, so we do not really care about potential precision loss
#[expect(clippy::cast_precision_loss)]
fn get_profile_data(
    profile: &Profile,
    sample_name: &str,
) -> Result<Vec<(String, FunctionProfile)>> {
    // Labels in string_table are prefixed with a whitespace
    let sample_label = format!(" {sample_name}");

    let sample_type_idx = profile
        .sample_type
        .iter()
        .position(|sample| {
            profile.string_table[usize::try_from(sample.unit)
                .expect("Overflow while converting samples id to usize")]
                == sample_label
        })
        .context("Failed to find sample in provided profile")?;

    let mut profile_map = HashMap::<String, FunctionProfile>::new();

    let location_map: HashMap<u64, &Location> = profile
        .location
        .iter()
        .map(|location| (location.id, location))
        .collect();

    let function_map: HashMap<u64, &Function> = profile
        .function
        .iter()
        .map(|function| (function.id, function))
        .collect();

    for sample in &profile.sample {
        let sample_value = sample.value[sample_type_idx];

        sample
            .location_id
            .iter()
            .filter_map(|&loc_id| {
                let line = location_map.get(&loc_id)?.line.first()?;
                let function = function_map.get(&line.function_id)?;
                Some(
                    &profile.string_table[usize::try_from(function.name)
                        .expect("Overflow while converting function id to usize")],
                )
            })
            .enumerate()
            .for_each(|(idx, function_name)| {
                let entry = profile_map.entry(function_name.clone()).or_default();
                entry.cumulative += sample_value;
                if idx == 0 {
                    entry.flat += sample_value;
                }
            });
    }

    let total_resource_count = profile_map
        .values()
        .max_by_key(|function| function.cumulative)
        .map(|function_profile| function_profile.cumulative)
        .context("Failed to obtain total resource count from cumulative stats")?;

    // sum_p depends on the correct order of data
    let mut sorted_profile_map: Vec<(String, FunctionProfile)> = profile_map.into_iter().collect();
    sorted_profile_map.sort_by(|a, b| b.1.flat.cmp(&a.1.flat));

    let mut sum_p: f64 = 0.0;

    for (_, profile) in &mut sorted_profile_map {
        sum_p += profile.flat as f64;

        profile.flat_p = (profile.flat as f64 / total_resource_count as f64) * 100.0;
        profile.cumulative_p = (profile.cumulative as f64 / total_resource_count as f64) * 100.0;
        profile.sum_p = (sum_p / total_resource_count as f64) * 100.0;
    }

    Ok(sorted_profile_map)
}

pub fn get_samples(profile: &Profile) -> Vec<&str> {
    profile
        .sample_type
        .iter()
        .map(|sample| {
            profile.string_table[usize::try_from(sample.unit)
                .expect("Overflow while converting samples id to usize")]
            .as_str()
        })
        .collect()
}

pub fn print_profile(profile: &Profile, sample: &str, limit: NonZeroUsize) -> Result<()> {
    let data = get_profile_data(profile, sample).context("Failed to get data from profile")?;

    let total_resource_count = data
        .iter()
        .max_by_key(|(_, profile)| profile.cumulative)
        .map(|(_, profile)| profile.cumulative)
        .context("Failed to obtain total resource count from profile data")?;

    let profile_length = data.len();
    let effective_limit = std::cmp::min(limit.get(), profile_length);
    let sliced = data.iter().take(effective_limit).collect::<Vec<_>>();

    let summary_resource_cost: i64 = sliced.iter().map(|(_key, profile)| profile.flat).sum();
    let cost_percentage = format!(
        "{:.2}%",
        &sliced
            .last()
            .map(|(_key, profile)| profile.sum_p)
            .context("Failed to get current percentage from profile data")?
    );

    println!(
        "\nShowing nodes accounting for {summary_resource_cost} {sample}, {cost_percentage} of {total_resource_count} {sample} total"
    );
    println!("Showing top {effective_limit} nodes out of {profile_length}\n");

    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(row![r->"flat", r->"flat%", r->"sum%", r->"cum", r->"cum%", ""]);

    for (name, profile) in sliced {
        table.add_row(row![
            r->format!("{} {}", profile.flat, &sample),
            r->format!("{:.2}%", profile.flat_p),
            r->format!("{:.2}%", profile.sum_p),
            r->format!("{} {}", profile.cumulative, &sample),
            r->format!("{:.2}%", profile.cumulative_p),
            l->serde_json::to_string(&name).unwrap()
        ]);
    }

    table.printstd();
    Ok(())
}

pub fn load_profile(path: &Utf8PathBuf) -> Result<Profile> {
    let profile_data = fs::read(path).context("Failed to read call trace from a file")?;

    let mut decoder = GzDecoder::new(&profile_data[..]);
    let mut decoded_buffer = vec![];
    decoder.read_to_end(&mut decoded_buffer)?;

    Profile::decode(&*decoded_buffer).context("Failed to decode profile data")
}
