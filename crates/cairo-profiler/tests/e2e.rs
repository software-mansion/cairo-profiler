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

            Showing nodes accounting for 1463 steps, 100.00% of 1463 steps total
            Showing top 15 nodes out of 15
            
                  flat |  flat% |    sum% |        cum |    cum% |  
            -----------+--------+---------+------------+---------+--------------------------------------------------------------------------------------------------------------
             866 steps | 59.19% |  59.19% |  866 steps |  59.19% | "CallContract" 
             102 steps |  6.97% |  66.17% |  179 steps |  12.24% | "core::result::ResultSerde::deserialize" 
              87 steps |  5.95% |  72.11% |   87 steps |   5.95% | "StorageRead" 
              87 steps |  5.95% |  78.06% |   87 steps |   5.95% | "snforge_std::cheatcode::execute_cheatcode" 
              64 steps |  4.37% |  82.43% | 1314 steps |  89.82% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value_return_wrapper" 
              39 steps |  2.67% |  85.10% |   39 steps |   2.67% | "core::array::SpanFelt252Serde::deserialize" 
              38 steps |  2.60% |  87.70% |   38 steps |   2.60% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
              37 steps |  2.53% |  90.23% |  124 steps |   8.48% | "balance_simple::HelloStarknet::__wrapper__HelloStarknetImpl__get_balance" 
              34 steps |  2.32% |  92.55% |  183 steps |  12.51% | "snforge_std::cheatcodes::contract_class::ContractClassImpl::deploy" 
              34 steps |  2.32% |  94.87% |  150 steps |  10.25% | "snforge_std::cheatcodes::contract_class::declare" 
              28 steps |  1.91% |  96.79% |   28 steps |   1.91% | "core::array::serialize_array_helper" 
              23 steps |  1.57% |  98.36% | 1337 steps |  91.39% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value" 
              22 steps |  1.50% |  99.86% |   51 steps |   3.49% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
               1 steps |  0.07% |  99.93% |  125 steps |   8.54% | "Contract: HelloStarknet\nFunction: get_balance\n" 
               1 steps |  0.07% | 100.00% | 1463 steps | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
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

            Showing nodes accounting for 38 range check builtin, 97.44% of 39 range check builtin total
            Showing top 3 nodes out of 15
            
                               flat |  flat% |   sum% |                    cum |    cum% |  
            ------------------------+--------+--------+------------------------+---------+-----------------------------------------------------------------------
             21 range check builtin | 53.85% | 53.85% | 39 range check builtin | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
             15 range check builtin | 38.46% | 92.31% | 15 range check builtin |  38.46% | "CallContract" 
              2 range check builtin |  5.13% | 97.44% |  3 range check builtin |   7.69% | "Contract: HelloStarknet\nFunction: get_balance\n" 
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

            Showing nodes accounting for 1463 steps, 100.00% of 1463 steps total
            Showing top 12 nodes out of 12

                  flat |  flat% |    sum% |        cum |    cum% |  
            -----------+--------+---------+------------+---------+--------------------------------------------------------------------------------------------------------------
             866 steps | 59.19% |  59.19% |  866 steps |  59.19% | "CallContract" 
             154 steps | 10.53% |  69.72% |  183 steps |  12.51% | "snforge_std::cheatcodes::contract_class::ContractClassImpl::deploy" 
              87 steps |  5.95% |  75.67% |   87 steps |   5.95% | "StorageRead" 
              87 steps |  5.95% |  81.61% |   87 steps |   5.95% | "snforge_std::cheatcode::execute_cheatcode" 
              83 steps |  5.67% |  87.29% |  150 steps |  10.25% | "snforge_std::cheatcodes::contract_class::declare" 
              64 steps |  4.37% |  91.66% | 1314 steps |  89.82% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value_return_wrapper" 
              38 steps |  2.60% |  94.26% |   38 steps |   2.60% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
              37 steps |  2.53% |  96.79% |  124 steps |   8.48% | "balance_simple::HelloStarknet::__wrapper__HelloStarknetImpl__get_balance" 
              23 steps |  1.57% |  98.36% | 1337 steps |  91.39% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value" 
              22 steps |  1.50% |  99.86% |   51 steps |   3.49% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
               1 steps |  0.07% |  99.93% |  125 steps |   8.54% | "Contract: HelloStarknet\nFunction: get_balance\n" 
               1 steps |  0.07% | 100.00% | 1463 steps | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
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

            Showing nodes accounting for 1463 steps, 100.00% of 1463 steps total
            Showing top 12 nodes out of 12

                  flat |  flat% |    sum% |        cum |    cum% |  
            -----------+--------+---------+------------+---------+--------------------------------------------------------------------------------------------------------------
             866 steps | 59.19% |  59.19% |  866 steps |  59.19% | "CallContract" 
             154 steps | 10.53% |  69.72% |  183 steps |  12.51% | "snforge_std::cheatcodes::contract_class::ContractClassImpl::deploy" 
              87 steps |  5.95% |  75.67% |   87 steps |   5.95% | "StorageRead" 
              87 steps |  5.95% |  81.61% |   87 steps |   5.95% | "snforge_std::cheatcode::execute_cheatcode" 
              83 steps |  5.67% |  87.29% |  150 steps |  10.25% | "snforge_std::cheatcodes::contract_class::declare" 
              64 steps |  4.37% |  91.66% | 1314 steps |  89.82% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value_return_wrapper" 
              38 steps |  2.60% |  94.26% |   38 steps |   2.60% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
              37 steps |  2.53% |  96.79% |  124 steps |   8.48% | "balance_simple::HelloStarknet::__wrapper__HelloStarknetImpl__get_balance" 
              23 steps |  1.57% |  98.36% | 1337 steps |  91.39% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value" 
              22 steps |  1.50% |  99.86% |   51 steps |   3.49% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
               1 steps |  0.07% |  99.93% |  125 steps |   8.54% | "Contract: HelloStarknet\nFunction: get_balance\n" 
               1 steps |  0.07% | 100.00% | 1463 steps | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
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

            Showing nodes accounting for 148525 sierra gas, 100.00% of 148525 sierra gas total
            Showing top 15 nodes out of 15
            
                         flat |  flat% |    sum% |               cum |    cum% |  
            ------------------+--------+---------+-------------------+---------+--------------------------------------------------------------------------------------------------------------
             86685 sierra gas | 58.36% |  58.36% |  86685 sierra gas |  58.36% | "CallContract" 
             10200 sierra gas |  6.87% |  65.23% |  18320 sierra gas |  12.33% | "core::result::ResultSerde::deserialize" 
             10000 sierra gas |  6.73% |  71.96% |  10000 sierra gas |   6.73% | "StorageRead" 
              9120 sierra gas |  6.14% |  78.10% |   9120 sierra gas |   6.14% | "snforge_std::cheatcode::execute_cheatcode" 
              6400 sierra gas |  4.31% |  82.41% | 132325 sierra gas |  89.09% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value_return_wrapper" 
              4320 sierra gas |  2.91% |  85.32% |   4320 sierra gas |   2.91% | "core::array::SpanFelt252Serde::deserialize" 
              3800 sierra gas |  2.56% |  87.88% |   3800 sierra gas |   2.56% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
              3700 sierra gas |  2.49% |  90.37% |  13700 sierra gas |   9.22% | "balance_simple::HelloStarknet::__wrapper__HelloStarknetImpl__get_balance" 
              3400 sierra gas |  2.29% |  92.66% |  18860 sierra gas |  12.70% | "snforge_std::cheatcodes::contract_class::ContractClassImpl::deploy" 
              3400 sierra gas |  2.29% |  94.95% |  15140 sierra gas |  10.19% | "snforge_std::cheatcodes::contract_class::declare" 
              2800 sierra gas |  1.89% |  96.84% |   2800 sierra gas |   1.89% | "core::array::serialize_array_helper" 
              2300 sierra gas |  1.55% |  98.38% | 134625 sierra gas |  90.64% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value" 
              2200 sierra gas |  1.48% |  99.87% |   5240 sierra gas |   3.53% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
               100 sierra gas |  0.07% |  99.93% |  13800 sierra gas |   9.29% | "Contract: HelloStarknet\nFunction: get_balance\n" 
               100 sierra gas |  0.07% | 100.00% | 148525 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
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
            
            Showing nodes accounting for 13590 sierra gas, 100.00% of 13590 sierra gas total
            Showing top 4 nodes out of 4
            
                        flat |  flat% |    sum% |              cum |    cum% |  
            -----------------+--------+---------+------------------+---------+-----------------------------------------------------------------------
             8250 sierra gas | 60.71% |  60.71% | 13490 sierra gas |  99.26% | "builtins_simple::tests::pedersen_cost" 
             3040 sierra gas | 22.37% |  83.08% |  3040 sierra gas |  22.37% | "snforge_std::cheatcode::execute_cheatcode" 
             2200 sierra gas | 16.19% |  99.26% |  5240 sierra gas |  38.56% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
              100 sierra gas |  0.74% | 100.00% | 13590 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
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
            
            Showing nodes accounting for 10123 sierra gas, 100.00% of 10123 sierra gas total
            Showing top 17 nodes out of 17
            
                        flat |  flat% |    sum% |              cum |    cum% |  
            -----------------+--------+---------+------------------+---------+-----------------------------------------------------------------------
             5200 sierra gas | 51.37% |  51.37% |  5200 sierra gas |  51.37% | "store_temp" 
              783 sierra gas |  7.73% |  59.10% |   783 sierra gas |   7.73% | "u8_bitwise" 
              570 sierra gas |  5.63% |  64.73% |   570 sierra gas |   5.63% | "array_slice" 
              500 sierra gas |  4.94% |  69.67% |   500 sierra gas |   4.94% | "withdraw_gas_all" 
              400 sierra gas |  3.95% |  73.62% |   400 sierra gas |   3.95% | "array_snapshot_pop_front" 
              370 sierra gas |  3.66% |  77.28% |   370 sierra gas |   3.66% | "u32_overflowing_sub" 
              300 sierra gas |  2.96% |  80.24% |   300 sierra gas |   2.96% | "enum_match" 
              300 sierra gas |  2.96% |  83.21% |   300 sierra gas |   2.96% | "felt252_is_zero" 
              300 sierra gas |  2.96% |  86.17% |   300 sierra gas |   2.96% | "withdraw_gas" 
              200 sierra gas |  1.98% |  88.15% |   200 sierra gas |   1.98% | "array_new" 
              200 sierra gas |  1.98% |  90.12% | 10023 sierra gas |  99.01% | "builtins_simple::tests::bitwise_cost" 
              200 sierra gas |  1.98% |  92.10% |   200 sierra gas |   1.98% | "get_builtin_costs" 
              200 sierra gas |  1.98% |  94.07% |   200 sierra gas |   1.98% | "jump" 
              200 sierra gas |  1.98% |  96.05% |  3040 sierra gas |  30.03% | "snforge_std::cheatcode::execute_cheatcode" 
              200 sierra gas |  1.98% |  98.02% |  5240 sierra gas |  51.76% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
              100 sierra gas |  0.99% |  99.01% | 10123 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
              100 sierra gas |  0.99% | 100.00% |   100 sierra gas |   0.99% | "bool_not_impl" 
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
            
            Showing nodes accounting for 144 casm size, 100.00% of 144 casm size total
            Showing top 4 nodes out of 4
            
                     flat |  flat% |    sum% |           cum |    cum% |  
            --------------+--------+---------+---------------+---------+-----------------------------------------------------------------------
             68 casm size | 47.22% |  47.22% | 144 casm size | 100.00% | "builtins_simple::tests::poseidon_cost" 
             41 casm size | 28.47% |  75.69% |  76 casm size |  52.78% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
             35 casm size | 24.31% | 100.00% |  35 casm size |  24.31% | "snforge_std::cheatcode::execute_cheatcode" 
              0 casm size |  0.00% | 100.00% | 144 casm size | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
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
            
            Showing nodes accounting for 144 casm size, 100.00% of 144 casm size total
            Showing top 27 nodes out of 27
            
                     flat |  flat% |    sum% |           cum |    cum% |  
            --------------+--------+---------+---------------+---------+-----------------------------------------------------------------------
             66 casm size | 45.83% |  45.83% |  66 casm size |  45.83% | "store_temp" 
             25 casm size | 17.36% |  63.19% |  25 casm size |  17.36% | "jump" 
             10 casm size |  6.94% |  70.14% |  10 casm size |   6.94% | "withdraw_gas_all" 
              8 casm size |  5.56% |  75.69% |   8 casm size |   5.56% | "array_snapshot_pop_front" 
              6 casm size |  4.17% |  79.86% |   6 casm size |   4.17% | "enum_match" 
              6 casm size |  4.17% |  84.03% |   6 casm size |   4.17% | "withdraw_gas" 
              5 casm size |  3.47% |  87.50% |   5 casm size |   3.47% | "array_slice" 
              4 casm size |  2.78% |  90.28% |   4 casm size |   2.78% | "array_new" 
              4 casm size |  2.78% |  93.06% |   4 casm size |   2.78% | "get_builtin_costs" 
              3 casm size |  2.08% |  95.14% |   3 casm size |   2.08% | "hades_permutation" 
              3 casm size |  2.08% |  97.22% |   3 casm size |   2.08% | "u32_overflowing_sub" 
              2 casm size |  1.39% |  98.61% |  41 casm size |  28.47% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
              2 casm size |  1.39% | 100.00% |  84 casm size |  58.33% | "snforge_std::cheatcode::is_config_run" 
              0 casm size |  0.00% | 100.00% | 144 casm size | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
              0 casm size |  0.00% | 100.00% |   0 casm size |   0.00% | "bool_not_impl" 
              0 casm size |  0.00% | 100.00% |  31 casm size |  21.53% | "builtins_simple::tests::poseidon_cost" 
              0 casm size |  0.00% | 100.00% |  11 casm size |   7.64% | "builtins_simple::tests::poseidon_cost_return_wrapper" 
              0 casm size |  0.00% | 100.00% |   0 casm size |   0.00% | "core::BoolNot::not" 
              0 casm size |  0.00% | 100.00% |  53 casm size |  36.81% | "core::Felt252PartialEq::eq" 
              0 casm size |  0.00% | 100.00% |   0 casm size |   0.00% | "core::Felt252Sub::sub" 
              0 casm size |  0.00% | 100.00% |   4 casm size |   2.78% | "core::array::ArrayImpl::new" 
              0 casm size |  0.00% | 100.00% |   6 casm size |   4.17% | "core::array::SpanImpl::pop_front" 
              0 casm size |  0.00% | 100.00% |   5 casm size |   3.47% | "core::array::SpanImpl::slice" 
              0 casm size |  0.00% | 100.00% |   4 casm size |   2.78% | "core::array::array_at" 
              0 casm size |  0.00% | 100.00% |   9 casm size |   6.25% | "core::integer::U32Sub::sub" 
              0 casm size |  0.00% | 100.00% |   0 casm size |   0.00% | "felt252_is_zero" 
              0 casm size |  0.00% | 100.00% |   7 casm size |   4.86% | "snforge_std::cheatcode::execute_cheatcode" 
            "#
        ));
}

#[test_case("cairo_steps", "trace_balance_simple.json"; "cairo_steps")]
#[test_case("sierra_gas", "trace_balance_simple.json"; "sierra_gas")]
fn view_syscall_counts(resource: &str, trace_name: &str) {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join(format!(
                "crates/cairo-profiler/tests/contracts/balance_simple/precompiled_{resource}/"
            )),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg(trace_name)
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
            Showing top 2 nodes out of 15
            
                        flat |  flat% |    sum% |             cum |   cum% |  
            -----------------+--------+---------+-----------------+--------+----------------
             1 syscall usage | 50.00% |  50.00% | 1 syscall usage | 50.00% | "CallContract" 
             1 syscall usage | 50.00% | 100.00% | 1 syscall usage | 50.00% | "StorageRead" 
            "#
        ));
}

#[test_case("cairo_steps", "trace_balance_simple_fork.json"; "cairo_steps_fork")]
#[test_case("sierra_gas", "trace_balance_simple_fork.json"; "sierra_gas_fork")]
fn view_syscall_counts_fork(resource: &str, trace_name: &str) {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join(format!(
                "crates/cairo-profiler/tests/contracts/balance_simple/precompiled_{resource}/"
            )),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg(trace_name)
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
            Showing top 2 nodes out of 8

                        flat |  flat% |    sum% |             cum |   cum% |  
            -----------------+--------+---------+-----------------+--------+----------------
             1 syscall usage | 50.00% |  50.00% | 1 syscall usage | 50.00% | "CallContract" 
             1 syscall usage | 50.00% | 100.00% | 1 syscall usage | 50.00% | "StorageRead" 
            "#
        ));
}

#[test]
fn view_deploy_syscall() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join("crates/cairo-profiler/tests/contracts/scaled_syscall/precompiled/"),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("scaled_syscall_deploy_syscall_cost.json")
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

            Showing nodes accounting for 260488 sierra gas, 100.00% of 260488 sierra gas total
            Showing top 16 nodes out of 16

                          flat |  flat% |    sum% |               cum |    cum% |  
            -------------------+--------+---------+-------------------+---------+----------------------------------------------------------------------------
             121448 sierra gas | 46.62% |  46.62% | 121448 sierra gas |  46.62% | "Deploy" 
              55340 sierra gas | 21.24% |  67.87% |  55340 sierra gas |  21.24% | "core::keccak::finalize_padding" 
              20000 sierra gas |  7.68% |  75.55% |  20000 sierra gas |   7.68% | "Keccak" 
              18560 sierra gas |  7.13% |  82.67% |  18560 sierra gas |   7.13% | "core::keccak::keccak_u256s_le_inputs[637-804]" 
               7160 sierra gas |  2.75% |  85.42% |  62500 sierra gas |  23.99% | "core::keccak::add_padding" 
               7100 sierra gas |  2.73% |  88.15% | 108160 sierra gas |  41.52% | "scaled_syscall::GasConstructorChecker::constructor" 
               6500 sierra gas |  2.50% |  90.64% | 148328 sierra gas |  56.94% | "scaled_syscall::deploy_syscall_cost" 
               6080 sierra gas |  2.33% |  92.97% |   6080 sierra gas |   2.33% | "snforge_std::_cheatcode::execute_cheatcode::" 
               3800 sierra gas |  1.46% |  94.43% |   3800 sierra gas |   1.46% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
               3500 sierra gas |  1.34% |  95.78% |   7300 sierra gas |   2.80% | "core::result::ResultSerde::::deserialize" 
               3500 sierra gas |  1.34% |  97.12% | 111660 sierra gas |  42.87% | "scaled_syscall::GasConstructorChecker::__wrapper__constructor" 
               3400 sierra gas |  1.31% |  98.43% |  15140 sierra gas |   5.81% | "snforge_std::cheatcodes::contract_class::declare" 
               2200 sierra gas |  0.84% |  99.27% |   5240 sierra gas |   2.01% | "snforge_std::_cheatcode::execute_cheatcode_and_deserialize::" 
               1400 sierra gas |  0.54% |  99.81% |   1400 sierra gas |   0.54% | "core::array::serialize_array_helper::" 
                400 sierra gas |  0.15% |  99.96% | 260488 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
                100 sierra gas |  0.04% | 100.00% | 111760 sierra gas |  42.90% | "Contract: GasConstructorChecker\nFunction: constructor\n" 
            "#
        ));
}
