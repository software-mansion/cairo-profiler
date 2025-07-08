use assert_fs::fixture::PathCopy;
use indoc::indoc;
use snapbox::cmd::{Command as SnapboxCommand, cargo_bin};
use std::str;
use test_case::test_case;

#[test]
fn output_path() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join("crates/cairo-profiler/tests/data/"),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("./call.json")
        .args(["-o", "my/output/dir/my_file.pb.gz"])
        .assert()
        .success();

    assert!(temp_dir.join("my/output/dir/my_file.pb.gz").exists());
}

#[test_case(&["call.json", "--show-inlined-functions"]; "with inlined functions")]
#[test_case(&["call.json", "--split-generics"]; "with split generics")]
#[test_case(&["call.json", "--max-function-stack-trace-depth", "5"]; "with max function trace depth")]
#[test_case(&["call.json", "--show-details"]; "with details")]
#[test_case(&["call.json"]; "without details")]
#[test_case(&["call.json", "--versioned-constants-path", "test_versioned_constants.json"]; "with custom versioned constants file")]
fn simple_package(args: &[&str]) {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join("crates/cairo-profiler/tests/data/"),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .args(args)
        .assert()
        .success();

    assert!(temp_dir.join("profile.pb.gz").exists());

    // TODO run pprof here
}

#[test]
fn missing_syscall_from_versioned_constants_file() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join("crates/cairo-profiler/tests/data/"),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .args([
            "call.json",
            "--versioned-constants-path",
            "missing_syscall_versioned_constants.json",
        ])
        .assert()
        .failure()
        .stderr_eq(indoc!(
            r"
            Error: Failed to get resource map from versioned constants file

            Caused by:
                Missing libfuncs cost in versioned constants file: [CallContract].
                Make sure to include costs of these libfuncs in the aforementioned file.
            "
        ));
}

#[test]
fn missing_os_constants_from_versioned_constants_file() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join("crates/cairo-profiler/tests/data/"),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .args([
            "call.json",
            "--versioned-constants-path",
            "invalid_versioned_constants.json",
        ])
        .assert()
        .failure()
        .stderr_eq(indoc!(
            r"
            Error: Failed to get resource map from versioned constants file

            Caused by:
                Invalid versioned constants file format: field 'os_constants' not found in versioned constants file
            "
        ));
}

#[test]
fn view_samples() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join(
                "crates/cairo-profiler/tests/contracts/balance_simple/precompiled_cairo_steps/",
            ),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("trace_balance_simple.json")
        .assert()
        .success();

    let output = SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("view")
        .arg("profile.pb.gz")
        .arg("--list-samples")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output_str = str::from_utf8(&output).expect("Output was not valid utf-8");

    assert!(
        output_str.contains("steps"),
        "Output contains: {}, missing 'steps'",
        &output_str
    );
    assert!(
        output_str.contains("calls"),
        "Output contains: {}, missing 'calls'",
        &output_str
    );
    assert!(
        output_str.contains("memory holes"),
        "Output contains: {}, missing 'memory holes'",
        &output_str
    );
    assert!(
        output_str.contains("range check builtin"),
        "Output contains: {}, missing 'range check builtin'",
        &output_str
    );
}

#[test]
fn view_steps() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join(
                "crates/cairo-profiler/tests/contracts/balance_simple/precompiled_cairo_steps/",
            ),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("trace_balance_simple.json")
        .assert()
        .success();

    // stdout asserts were generated using `go tool pprof -top profile.pb.gz` command
    // when changing any view_* tests please always generate expected output using this tool
    // formatting was changed manually, since it differs a bit between pprof and cairo-profiler view

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("view")
        .arg("profile.pb.gz")
        .arg("--limit")
        .arg("2137")
        .arg("--sample")
        .arg("steps")
        .assert()
        .success()
        .stdout_eq(indoc!(
            r#"

            Showing nodes accounting for 1418 steps, 100.00% of 1418 steps total
            Showing top 15 nodes out of 15
            
                  flat |  flat% |    sum% |        cum |    cum% |  
            -----------+--------+---------+------------+---------+-----------------------------------------------------------------------------------------------
             866 steps | 61.07% |  61.07% |  866 steps |  61.07% | "CallContract" 
              91 steps |  6.42% |  67.49% |  168 steps |  11.85% | "core::result::ResultSerde::::deserialize" 
              87 steps |  6.14% |  73.62% |   87 steps |   6.14% | "StorageRead" 
              78 steps |  5.50% |  79.13% |   78 steps |   5.50% | "snforge_std::_cheatcode::execute_cheatcode::" 
              43 steps |  3.03% |  82.16% | 1293 steps |  91.18% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value" 
              39 steps |  2.75% |  84.91% |   39 steps |   2.75% | "core::array::SpanFelt252Serde::deserialize" 
              38 steps |  2.68% |  87.59% |   38 steps |   2.68% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
              33 steps |  2.33% |  89.92% |  120 steps |   8.46% | "balance_simple::HelloStarknet::__wrapper__HelloStarknetImpl__get_balance" 
              33 steps |  2.33% |  92.24% |  170 steps |  11.99% | "snforge_std::cheatcodes::contract_class::ContractClassImpl::deploy" 
              33 steps |  2.33% |  94.57% |  142 steps |  10.01% | "snforge_std::cheatcodes::contract_class::declare" 
              27 steps |  1.90% |  96.47% |  339 steps |  23.91% | "balance_simple_integrationtest::test_contract::deploy_contract" 
              26 steps |  1.83% |  98.31% |   26 steps |   1.83% | "core::array::serialize_array_helper::" 
              19 steps |  1.34% |  99.65% |   45 steps |   3.17% | "snforge_std::_cheatcode::execute_cheatcode_and_deserialize::" 
               4 steps |  0.28% |  99.93% | 1418 steps | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
               1 steps |  0.07% | 100.00% |  121 steps |   8.53% | "Contract: HelloStarknet\nFunction: get_balance\n" 
            "#
        ));
}

#[test]
fn view_range_check_builtin() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join(
                "crates/cairo-profiler/tests/contracts/balance_simple/precompiled_cairo_steps/",
            ),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("trace_balance_simple.json")
        .arg("--view")
        .arg("--sample")
        .arg("range check builtin")
        .arg("--limit")
        .arg("3")
        .assert()
        .success()
        .stdout_eq(indoc!(
            r#"
            
            Showing nodes accounting for 36 range check builtin, 97.30% of 37 range check builtin total
            Showing top 3 nodes out of 15
            
                               flat |  flat% |   sum% |                    cum |    cum% |  
            ------------------------+--------+--------+------------------------+---------+-----------------------------------------------------------------------
             19 range check builtin | 51.35% | 51.35% | 37 range check builtin | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
             15 range check builtin | 40.54% | 91.89% | 15 range check builtin |  40.54% | "CallContract" 
              2 range check builtin |  5.41% | 97.30% |  3 range check builtin |   8.11% | "Contract: HelloStarknet\nFunction: get_balance\n" 
            "#
        ));
}

#[test]
fn view_hide_invalid_regex() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join(
                "crates/cairo-profiler/tests/contracts/balance_simple/precompiled_cairo_steps/",
            ),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("trace_balance_simple.json")
        .assert()
        .success();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("view")
        .arg("profile.pb.gz")
        .arg("--limit")
        .arg("2137")
        .arg("--sample")
        .arg("steps")
        .arg("--hide")
        .arg("[core")
        .assert()
        .failure()
        .stderr_eq(indoc!(
            r"
            Error: Failed to get data from profile
            
            Caused by:
                0: Invalid regular expression passed
                1: regex parse error:
                       [core
                       ^
                   error: unclosed character class
            "
        ));
}

#[test]
fn view_hide_in_view() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join(
                "crates/cairo-profiler/tests/contracts/balance_simple/precompiled_cairo_steps/",
            ),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("trace_balance_simple.json")
        .assert()
        .success();

    // stdout asserts were generated using `go tool pprof -hide "core::*" -top profile.pb.gz` command
    // as well as `go tool pprof -hide "core" -top profile.pb.gz` command (they should be the same!)
    // when changing any view_* tests please always generate expected output using this tool
    // formatting was changed manually, since it differs a bit between pprof and cairo-profiler view

    let expected_output = indoc!(
        r#"

            Active filter:
            hide=core

            Showing nodes accounting for 1418 steps, 100.00% of 1418 steps total
            Showing top 12 nodes out of 12

                  flat |  flat% |    sum% |        cum |    cum% |  
            -----------+--------+---------+------------+---------+-----------------------------------------------------------------------------------------------
             866 steps | 61.07% |  61.07% |  866 steps |  61.07% | "CallContract" 
             144 steps | 10.16% |  71.23% |  170 steps |  11.99% | "snforge_std::cheatcodes::contract_class::ContractClassImpl::deploy" 
              87 steps |  6.14% |  77.36% |   87 steps |   6.14% | "StorageRead" 
              78 steps |  5.50% |  82.86% |   78 steps |   5.50% | "snforge_std::_cheatcode::execute_cheatcode::" 
              78 steps |  5.50% |  88.36% |  142 steps |  10.01% | "snforge_std::cheatcodes::contract_class::declare" 
              43 steps |  3.03% |  91.40% | 1293 steps |  91.18% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value" 
              38 steps |  2.68% |  94.08% |   38 steps |   2.68% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
              33 steps |  2.33% |  96.40% |  120 steps |   8.46% | "balance_simple::HelloStarknet::__wrapper__HelloStarknetImpl__get_balance" 
              27 steps |  1.90% |  98.31% |  339 steps |  23.91% | "balance_simple_integrationtest::test_contract::deploy_contract" 
              19 steps |  1.34% |  99.65% |   45 steps |   3.17% | "snforge_std::_cheatcode::execute_cheatcode_and_deserialize::" 
               4 steps |  0.28% |  99.93% | 1418 steps | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
               1 steps |  0.07% | 100.00% |  121 steps |   8.53% | "Contract: HelloStarknet\nFunction: get_balance\n" 
            "#
    );

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("view")
        .arg("profile.pb.gz")
        .arg("--limit")
        .arg("2137")
        .arg("--sample")
        .arg("steps")
        .arg("--hide")
        .arg("core")
        .assert()
        .success()
        .stdout_eq(expected_output);
}

#[test]
fn view_hide_in_build() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join(
                "crates/cairo-profiler/tests/contracts/balance_simple/precompiled_cairo_steps/",
            ),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("trace_balance_simple.json")
        .assert()
        .success();

    // stdout asserts were generated using `go tool pprof -hide "core::*" -top profile.pb.gz` command
    // as well as `go tool pprof -hide "core" -top profile.pb.gz` command (they should be the same!)
    // when changing any view_* tests please always generate expected output using this tool
    // formatting was changed manually, since it differs a bit between pprof and cairo-profiler view

    let expected_output = indoc!(
        r#"

            Active filter:
            hide=core::*

            Showing nodes accounting for 1418 steps, 100.00% of 1418 steps total
            Showing top 12 nodes out of 12

                  flat |  flat% |    sum% |        cum |    cum% |  
            -----------+--------+---------+------------+---------+-----------------------------------------------------------------------------------------------
             866 steps | 61.07% |  61.07% |  866 steps |  61.07% | "CallContract" 
             144 steps | 10.16% |  71.23% |  170 steps |  11.99% | "snforge_std::cheatcodes::contract_class::ContractClassImpl::deploy" 
              87 steps |  6.14% |  77.36% |   87 steps |   6.14% | "StorageRead" 
              78 steps |  5.50% |  82.86% |   78 steps |   5.50% | "snforge_std::_cheatcode::execute_cheatcode::" 
              78 steps |  5.50% |  88.36% |  142 steps |  10.01% | "snforge_std::cheatcodes::contract_class::declare" 
              43 steps |  3.03% |  91.40% | 1293 steps |  91.18% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value" 
              38 steps |  2.68% |  94.08% |   38 steps |   2.68% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
              33 steps |  2.33% |  96.40% |  120 steps |   8.46% | "balance_simple::HelloStarknet::__wrapper__HelloStarknetImpl__get_balance" 
              27 steps |  1.90% |  98.31% |  339 steps |  23.91% | "balance_simple_integrationtest::test_contract::deploy_contract" 
              19 steps |  1.34% |  99.65% |   45 steps |   3.17% | "snforge_std::_cheatcode::execute_cheatcode_and_deserialize::" 
               4 steps |  0.28% |  99.93% | 1418 steps | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
               1 steps |  0.07% | 100.00% |  121 steps |   8.53% | "Contract: HelloStarknet\nFunction: get_balance\n" 
            "#
    );

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("view")
        .arg("profile.pb.gz")
        .arg("--limit")
        .arg("2137")
        .arg("--sample")
        .arg("steps")
        .arg("--hide")
        .arg("core::*")
        .assert()
        .success()
        .stdout_eq(expected_output);
}

#[test]
fn view_sierra_gas() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join(
                "crates/cairo-profiler/tests/contracts/balance_simple/precompiled_sierra_gas/",
            ),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("trace_balance_simple.json")
        .assert()
        .success();

    // stdout asserts were generated using `go tool pprof -top profile.pb.gz` command
    // when changing any view_* tests please always generate expected output using this tool
    // formatting was changed manually, since it differs a bit between pprof and cairo-profiler view

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("view")
        .arg("profile.pb.gz")
        .arg("--limit")
        .arg("2137")
        .arg("--sample")
        .arg("sierra gas")
        .assert()
        .success()
        .stdout_eq(indoc!(
            r#"

            Showing nodes accounting for 146525 sierra gas, 100.00% of 146525 sierra gas total
            Showing top 14 nodes out of 14
            
                         flat |  flat% |    sum% |               cum |    cum% |  
            ------------------+--------+---------+-------------------+---------+-----------------------------------------------------------------------------------------------
             86685 sierra gas | 59.16% |  59.16% |  86685 sierra gas |  59.16% | "CallContract" 
             10200 sierra gas |  6.96% |  66.12% |  18320 sierra gas |  12.50% | "core::result::ResultSerde::::deserialize" 
             10000 sierra gas |  6.82% |  72.95% |  10000 sierra gas |   6.82% | "StorageRead" 
              9120 sierra gas |  6.22% |  79.17% |   9120 sierra gas |   6.22% | "snforge_std::_cheatcode::execute_cheatcode::" 
              6400 sierra gas |  4.37% |  83.54% | 132325 sierra gas |  90.31% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value" 
              4320 sierra gas |  2.95% |  86.49% |   4320 sierra gas |   2.95% | "core::array::SpanFelt252Serde::deserialize" 
              3800 sierra gas |  2.59% |  89.08% |   3800 sierra gas |   2.59% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
              3700 sierra gas |  2.53% |  91.61% |  13700 sierra gas |   9.35% | "balance_simple::HelloStarknet::__wrapper__HelloStarknetImpl__get_balance" 
              3400 sierra gas |  2.32% |  93.93% |  18860 sierra gas |  12.87% | "snforge_std::cheatcodes::contract_class::ContractClassImpl::deploy" 
              3400 sierra gas |  2.32% |  96.25% |  15140 sierra gas |  10.33% | "snforge_std::cheatcodes::contract_class::declare" 
              2800 sierra gas |  1.91% |  98.16% |   2800 sierra gas |   1.91% | "core::array::serialize_array_helper::" 
              2200 sierra gas |  1.50% |  99.66% |   5240 sierra gas |   3.58% | "snforge_std::_cheatcode::execute_cheatcode_and_deserialize::" 
               400 sierra gas |  0.27% |  99.93% | 146525 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
               100 sierra gas |  0.07% | 100.00% |  13800 sierra gas |   9.42% | "Contract: HelloStarknet\nFunction: get_balance\n" 
            "#
        ));
}

#[test]
fn view_builtins_factored_in() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join("crates/cairo-profiler/tests/contracts/builtins_simple/precompiled/"),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("builtins_simple_tests_pedersen_cost.json")
        .assert()
        .success();

    // stdout asserts were generated using `go tool pprof -top profile.pb.gz` command
    // when changing any view_* tests please always generate expected output using this tool
    // formatting was changed manually, since it differs a bit between pprof and cairo-profiler view

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("view")
        .arg("profile.pb.gz")
        .arg("--limit")
        .arg("2137")
        .arg("--sample")
        .arg("sierra gas")
        .assert()
        .success()
        .stdout_eq(indoc!(
            r#"
            
            Showing nodes accounting for 12290 sierra gas, 100.00% of 12290 sierra gas total
            Showing top 4 nodes out of 4
            
                        flat |  flat% |    sum% |              cum |    cum% |  
            -----------------+--------+---------+------------------+---------+-----------------------------------------------------------------------
             6550 sierra gas | 53.30% |  53.30% | 11790 sierra gas |  95.93% | "builtins_simple::tests::pedersen_cost" 
             3040 sierra gas | 24.74% |  78.03% |  3040 sierra gas |  24.74% | "snforge_std::_cheatcode::execute_cheatcode::" 
             2200 sierra gas | 17.90% |  95.93% |  5240 sierra gas |  42.64% | "snforge_std::_cheatcode::execute_cheatcode_and_deserialize::" 
              500 sierra gas |  4.07% | 100.00% | 12290 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
            "#
        ));
}

#[test]
fn view_all_libfuncs() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join("crates/cairo-profiler/tests/contracts/builtins_simple/precompiled/"),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("builtins_simple_tests_bitwise_cost.json")
        .arg("--show-libfuncs")
        .assert()
        .success();

    // stdout asserts were generated using `go tool pprof -top profile.pb.gz` command
    // when changing any view_* tests please always generate expected output using this tool
    // formatting was changed manually, since it differs a bit between pprof and cairo-profiler view

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("view")
        .arg("profile.pb.gz")
        .arg("--limit")
        .arg("2137")
        .arg("--sample")
        .arg("sierra gas")
        .assert()
        .success()
        .stdout_eq(indoc!(
            r#"
            
            Showing nodes accounting for 8823 sierra gas, 100.00% of 8823 sierra gas total
            Showing top 14 nodes out of 14
            
                        flat |  flat% |    sum% |             cum |    cum% |  
            -----------------+--------+---------+-----------------+---------+-----------------------------------------------------------------------
             4700 sierra gas | 53.27% |  53.27% | 4700 sierra gas |  53.27% | "store_temp" 
              783 sierra gas |  8.87% |  62.14% |  783 sierra gas |   8.87% | "u8_bitwise" 
              570 sierra gas |  6.46% |  68.60% |  570 sierra gas |   6.46% | "array_slice" 
              500 sierra gas |  5.67% |  74.27% | 8823 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
              400 sierra gas |  4.53% |  78.81% |  400 sierra gas |   4.53% | "array_snapshot_pop_front" 
              370 sierra gas |  4.19% |  83.00% |  370 sierra gas |   4.19% | "u32_overflowing_sub" 
              300 sierra gas |  3.40% |  86.40% |  300 sierra gas |   3.40% | "enum_match" 
              300 sierra gas |  3.40% |  89.80% |  300 sierra gas |   3.40% | "felt252_is_zero" 
              200 sierra gas |  2.27% |  92.07% | 8323 sierra gas |  94.33% | "builtins_simple::tests::bitwise_cost" 
              200 sierra gas |  2.27% |  94.33% | 3040 sierra gas |  34.46% | "snforge_std::_cheatcode::execute_cheatcode::" 
              200 sierra gas |  2.27% |  96.60% | 5240 sierra gas |  59.39% | "snforge_std::_cheatcode::execute_cheatcode_and_deserialize::" 
              100 sierra gas |  1.13% |  97.73% |  100 sierra gas |   1.13% | "array_new" 
              100 sierra gas |  1.13% |  98.87% |  100 sierra gas |   1.13% | "bool_not_impl" 
              100 sierra gas |  1.13% | 100.00% |  100 sierra gas |   1.13% | "jump" 
            "#
        ));
}

#[test]
fn view_casm_sizes_minimal() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join("crates/cairo-profiler/tests/contracts/builtins_simple/precompiled/"),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("builtins_simple_tests_poseidon_cost.json")
        .assert()
        .success();

    // stdout asserts were generated using `go tool pprof -top profile.pb.gz` command
    // when changing any view_* tests please always generate expected output using this tool
    // formatting was changed manually, since it differs a bit between pprof and cairo-profiler view

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("view")
        .arg("profile.pb.gz")
        .arg("--limit")
        .arg("2137")
        .arg("--sample")
        .arg("casm size")
        .assert()
        .success()
        .stdout_eq(indoc!(
            r#"
            
            Showing nodes accounting for 94 casm size, 100.00% of 94 casm size total
            Showing top 4 nodes out of 4
            
                     flat |  flat% |    sum% |          cum |    cum% |  
            --------------+--------+---------+--------------+---------+-----------------------------------------------------------------------
             33 casm size | 35.11% |  35.11% | 94 casm size | 100.00% | "builtins_simple::tests::poseidon_cost" 
             33 casm size | 35.11% |  70.21% | 61 casm size |  64.89% | "snforge_std::_cheatcode::execute_cheatcode_and_deserialize::" 
             28 casm size | 29.79% | 100.00% | 28 casm size |  29.79% | "snforge_std::_cheatcode::execute_cheatcode::" 
              0 casm size |  0.00% | 100.00% | 94 casm size | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
            "#
        ));
}

#[test]
fn view_casm_sizes_with_libfuncs_and_inlines() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join("crates/cairo-profiler/tests/contracts/builtins_simple/precompiled/"),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("builtins_simple_tests_poseidon_cost.json")
        .arg("--show-libfuncs")
        .arg("--show-inlined-functions")
        .assert()
        .success();

    // stdout asserts were generated using `go tool pprof -top profile.pb.gz` command
    // when changing any view_* tests please always generate expected output using this tool
    // formatting was changed manually, since it differs a bit between pprof and cairo-profiler view

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("view")
        .arg("profile.pb.gz")
        .arg("--limit")
        .arg("2137")
        .arg("--sample")
        .arg("casm size")
        .assert()
        .success()
        .stdout_eq(indoc!(
            r#"
            
            Showing nodes accounting for 94 casm size, 100.00% of 94 casm size total
            Showing top 24 nodes out of 24
            
                     flat |  flat% |    sum% |          cum |    cum% |  
            --------------+--------+---------+--------------+---------+-----------------------------------------------------------------------
             58 casm size | 61.70% |  61.70% | 58 casm size |  61.70% | "store_temp" 
              9 casm size |  9.57% |  71.28% |  9 casm size |   9.57% | "felt252_is_zero" 
              9 casm size |  9.57% |  80.85% |  9 casm size |   9.57% | "jump" 
              8 casm size |  8.51% |  89.36% |  8 casm size |   8.51% | "array_snapshot_pop_front" 
              8 casm size |  8.51% |  97.87% | 71 casm size |  75.53% | "snforge_std::_cheatcode::_is_config_run" 
              1 casm size |  1.06% |  98.94% |  1 casm size |   1.06% | "array_new" 
              1 casm size |  1.06% | 100.00% |  1 casm size |   1.06% | "enum_match" 
              0 casm size |  0.00% | 100.00% | 94 casm size | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
              0 casm size |  0.00% | 100.00% |  0 casm size |   0.00% | "array_slice" 
              0 casm size |  0.00% | 100.00% |  0 casm size |   0.00% | "bool_not_impl" 
              0 casm size |  0.00% | 100.00% |  6 casm size |   6.38% | "builtins_simple::tests::poseidon_cost" 
              0 casm size |  0.00% | 100.00% |  0 casm size |   0.00% | "core::BoolNot::not" 
              0 casm size |  0.00% | 100.00% | 32 casm size |  34.04% | "core::Felt252PartialEq::eq" 
              0 casm size |  0.00% | 100.00% |  2 casm size |   2.13% | "core::Felt252Sub::sub" 
              0 casm size |  0.00% | 100.00% |  1 casm size |   1.06% | "core::array::ArrayImpl::new" 
              0 casm size |  0.00% | 100.00% |  3 casm size |   3.19% | "core::array::SpanImpl::pop_front" 
              0 casm size |  0.00% | 100.00% |  4 casm size |   4.26% | "core::array::SpanImpl::slice" 
              0 casm size |  0.00% | 100.00% |  8 casm size |   8.51% | "core::array::array_at" 
              0 casm size |  0.00% | 100.00% |  9 casm size |   9.57% | "core::assert" 
              0 casm size |  0.00% | 100.00% |  8 casm size |   8.51% | "core::integer::U32Sub::sub" 
              0 casm size |  0.00% | 100.00% |  0 casm size |   0.00% | "hades_permutation" 
              0 casm size |  0.00% | 100.00% |  0 casm size |   0.00% | "snforge_std::_cheatcode::execute_cheatcode" 
              0 casm size |  0.00% | 100.00% | 39 casm size |  41.49% | "snforge_std::_cheatcode::execute_cheatcode_and_deserialize" 
              0 casm size |  0.00% | 100.00% |  0 casm size |   0.00% | "u32_overflowing_sub" 
            "#
        ));
}

#[test]
fn view_syscall_counts() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join(
                "crates/cairo-profiler/tests/contracts/balance_simple/precompiled_sierra_gas/",
            ),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("trace_balance_simple.json")
        .assert()
        .success();

    // stdout asserts were generated using `go tool pprof -top profile.pb.gz` command
    // when changing any view_* tests please always generate expected output using this tool
    // formatting was changed manually, since it differs a bit between pprof and cairo-profiler view

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("view")
        .arg("profile.pb.gz")
        .arg("--limit")
        .arg("2")
        .arg("--sample")
        .arg("syscall usage")
        .assert()
        .success()
        .stdout_eq(indoc!(
            r#"
            
            Showing nodes accounting for 2 syscall usage, 100.00% of 2 syscall usage total
            Showing top 2 nodes out of 14
            
                        flat |  flat% |    sum% |             cum |   cum% |  
            -----------------+--------+---------+-----------------+--------+----------------
             1 syscall usage | 50.00% |  50.00% | 1 syscall usage | 50.00% | "CallContract" 
             1 syscall usage | 50.00% | 100.00% | 1 syscall usage | 50.00% | "StorageRead" 
            "#
        ));
}
