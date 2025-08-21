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
        .success()
        .stderr_eq(indoc!(
            r"
            [WARNING] Missing calldata_factors for scaled syscalls - resource estimations may not be accurate. Consider using snforge 0.48+ for trace generation.
            "
        ));

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

            Showing nodes accounting for 1503 steps, 100.00% of 1503 steps total
            Showing top 15 nodes out of 15
            
                  flat |  flat% |    sum% |        cum |    cum% |  
            -----------+--------+---------+------------+---------+--------------------------------------------------------------------------------------------------------------
             903 steps | 60.08% |  60.08% | 1031 steps |  68.60% | "CallContract" 
             102 steps |  6.79% |  66.87% |  179 steps |  11.91% | "core::result::ResultSerde::deserialize" 
              90 steps |  5.99% |  72.85% |   90 steps |   5.99% | "StorageRead" 
              87 steps |  5.79% |  78.64% |   87 steps |   5.79% | "snforge_std::cheatcode::execute_cheatcode" 
              64 steps |  4.26% |  82.90% | 1479 steps |  98.40% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value_return_wrapper" 
              39 steps |  2.59% |  85.50% |   39 steps |   2.59% | "core::array::SpanFelt252Serde::deserialize" 
              38 steps |  2.53% |  88.02% |   38 steps |   2.53% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
              37 steps |  2.46% |  90.49% |  127 steps |   8.45% | "balance_simple::HelloStarknet::__wrapper__HelloStarknetImpl__get_balance" 
              34 steps |  2.26% |  92.75% |  183 steps |  12.18% | "snforge_std::cheatcodes::contract_class::ContractClassImpl::deploy" 
              34 steps |  2.26% |  95.01% |  150 steps |   9.98% | "snforge_std::cheatcodes::contract_class::declare" 
              28 steps |  1.86% |  96.87% |   28 steps |   1.86% | "core::array::serialize_array_helper" 
              23 steps |  1.53% |  98.40% | 1502 steps |  99.93% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value" 
              22 steps |  1.46% |  99.87% |   51 steps |   3.39% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
               1 steps |  0.07% |  99.93% |  128 steps |   8.52% | "Contract: HelloStarknet\nFunction: get_balance\n" 
               1 steps |  0.07% | 100.00% | 1503 steps | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
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
        .arg("4")
        .assert()
        .success()
        .stdout_eq(indoc!(
            r#"

            Showing nodes accounting for 42 range check builtin, 100.00% of 42 range check builtin total
            Showing top 4 nodes out of 15
            
                               flat |  flat% |    sum% |                    cum |    cum% |  
            ------------------------+--------+---------+------------------------+---------+-----------------------------------------------------------------------
             21 range check builtin | 50.00% |  50.00% | 42 range check builtin | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
             18 range check builtin | 42.86% |  92.86% | 21 range check builtin |  50.00% | "CallContract" 
              2 range check builtin |  4.76% |  97.62% |  3 range check builtin |   7.14% | "Contract: HelloStarknet\nFunction: get_balance\n" 
              1 range check builtin |  2.38% | 100.00% |  1 range check builtin |   2.38% | "StorageRead" 
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

            Showing nodes accounting for 1503 steps, 100.00% of 1503 steps total
            Showing top 12 nodes out of 12

                  flat |  flat% |    sum% |        cum |    cum% |  
            -----------+--------+---------+------------+---------+--------------------------------------------------------------------------------------------------------------
             903 steps | 60.08% |  60.08% | 1031 steps |  68.60% | "CallContract" 
             154 steps | 10.25% |  70.33% |  183 steps |  12.18% | "snforge_std::cheatcodes::contract_class::ContractClassImpl::deploy" 
              90 steps |  5.99% |  76.31% |   90 steps |   5.99% | "StorageRead" 
              87 steps |  5.79% |  82.10% |   87 steps |   5.79% | "snforge_std::cheatcode::execute_cheatcode" 
              83 steps |  5.52% |  87.62% |  150 steps |   9.98% | "snforge_std::cheatcodes::contract_class::declare" 
              64 steps |  4.26% |  91.88% | 1479 steps |  98.40% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value_return_wrapper" 
              38 steps |  2.53% |  94.41% |   38 steps |   2.53% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
              37 steps |  2.46% |  96.87% |  127 steps |   8.45% | "balance_simple::HelloStarknet::__wrapper__HelloStarknetImpl__get_balance" 
              23 steps |  1.53% |  98.40% | 1502 steps |  99.93% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value" 
              22 steps |  1.46% |  99.87% |   51 steps |   3.39% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
               1 steps |  0.07% |  99.93% |  128 steps |   8.52% | "Contract: HelloStarknet\nFunction: get_balance\n" 
               1 steps |  0.07% | 100.00% | 1503 steps | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
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

            Showing nodes accounting for 1503 steps, 100.00% of 1503 steps total
            Showing top 12 nodes out of 12

                  flat |  flat% |    sum% |        cum |    cum% |  
            -----------+--------+---------+------------+---------+--------------------------------------------------------------------------------------------------------------
             903 steps | 60.08% |  60.08% | 1031 steps |  68.60% | "CallContract" 
             154 steps | 10.25% |  70.33% |  183 steps |  12.18% | "snforge_std::cheatcodes::contract_class::ContractClassImpl::deploy" 
              90 steps |  5.99% |  76.31% |   90 steps |   5.99% | "StorageRead" 
              87 steps |  5.79% |  82.10% |   87 steps |   5.79% | "snforge_std::cheatcode::execute_cheatcode" 
              83 steps |  5.52% |  87.62% |  150 steps |   9.98% | "snforge_std::cheatcodes::contract_class::declare" 
              64 steps |  4.26% |  91.88% | 1479 steps |  98.40% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value_return_wrapper" 
              38 steps |  2.53% |  94.41% |   38 steps |   2.53% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
              37 steps |  2.46% |  96.87% |  127 steps |   8.45% | "balance_simple::HelloStarknet::__wrapper__HelloStarknetImpl__get_balance" 
              23 steps |  1.53% |  98.40% | 1502 steps |  99.93% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value" 
              22 steps |  1.46% |  99.87% |   51 steps |   3.39% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
               1 steps |  0.07% |  99.93% |  128 steps |   8.52% | "Contract: HelloStarknet\nFunction: get_balance\n" 
               1 steps |  0.07% | 100.00% | 1503 steps | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
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

            Showing nodes accounting for 151388 sierra gas, 100.00% of 151388 sierra gas total
            Showing top 15 nodes out of 15
            
                         flat |  flat% |    sum% |               cum |    cum% |  
            ------------------+--------+---------+-------------------+---------+--------------------------------------------------------------------------------------------------------------
             90388 sierra gas | 59.71% |  59.71% | 104188 sierra gas |  68.82% | "CallContract" 
             10200 sierra gas |  6.74% |  66.44% |  17900 sierra gas |  11.82% | "core::result::ResultSerde::deserialize" 
             10000 sierra gas |  6.61% |  73.05% |  10000 sierra gas |   6.61% | "StorageRead" 
              8700 sierra gas |  5.75% |  78.80% |   8700 sierra gas |   5.75% | "snforge_std::cheatcode::execute_cheatcode" 
              6400 sierra gas |  4.23% |  83.02% | 148988 sierra gas |  98.41% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value_return_wrapper" 
              3900 sierra gas |  2.58% |  85.60% |   3900 sierra gas |   2.58% | "core::array::SpanFelt252Serde::deserialize" 
              3800 sierra gas |  2.51% |  88.11% |   3800 sierra gas |   2.51% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
              3700 sierra gas |  2.44% |  90.55% |  13700 sierra gas |   9.05% | "balance_simple::HelloStarknet::__wrapper__HelloStarknetImpl__get_balance" 
              3400 sierra gas |  2.25% |  92.80% |  18300 sierra gas |  12.09% | "snforge_std::cheatcodes::contract_class::ContractClassImpl::deploy" 
              3400 sierra gas |  2.25% |  95.05% |  15000 sierra gas |   9.91% | "snforge_std::cheatcodes::contract_class::declare" 
              2800 sierra gas |  1.85% |  96.90% |   2800 sierra gas |   1.85% | "core::array::serialize_array_helper" 
              2300 sierra gas |  1.52% |  98.41% | 151288 sierra gas |  99.93% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value" 
              2200 sierra gas |  1.45% |  99.87% |   5100 sierra gas |   3.37% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
               100 sierra gas |  0.07% |  99.93% |  13800 sierra gas |   9.12% | "Contract: HelloStarknet\nFunction: get_balance\n" 
               100 sierra gas |  0.07% | 100.00% | 151388 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
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
            
            Showing nodes accounting for 13450 sierra gas, 100.00% of 13450 sierra gas total
            Showing top 4 nodes out of 4
            
                        flat |  flat% |    sum% |              cum |    cum% |  
            -----------------+--------+---------+------------------+---------+-----------------------------------------------------------------------
             8250 sierra gas | 61.34% |  61.34% | 13350 sierra gas |  99.26% | "builtins_simple::tests::pedersen_cost" 
             2900 sierra gas | 21.56% |  82.90% |  2900 sierra gas |  21.56% | "snforge_std::cheatcode::execute_cheatcode" 
             2200 sierra gas | 16.36% |  99.26% |  5100 sierra gas |  37.92% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
              100 sierra gas |  0.74% | 100.00% | 13450 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
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
            
            Showing nodes accounting for 9983 sierra gas, 100.00% of 9983 sierra gas total
            Showing top 17 nodes out of 17
            
                        flat |  flat% |    sum% |             cum |    cum% |  
            -----------------+--------+---------+-----------------+---------+-----------------------------------------------------------------------
             5200 sierra gas | 52.09% |  52.09% | 5200 sierra gas |  52.09% | "store_temp" 
              783 sierra gas |  7.84% |  59.93% |  783 sierra gas |   7.84% | "u8_bitwise" 
              500 sierra gas |  5.01% |  64.94% |  500 sierra gas |   5.01% | "array_slice" 
              500 sierra gas |  5.01% |  69.95% |  500 sierra gas |   5.01% | "withdraw_gas_all" 
              400 sierra gas |  4.01% |  73.96% |  400 sierra gas |   4.01% | "array_snapshot_pop_front" 
              300 sierra gas |  3.01% |  76.96% |  300 sierra gas |   3.01% | "enum_match" 
              300 sierra gas |  3.01% |  79.97% |  300 sierra gas |   3.01% | "felt252_is_zero" 
              300 sierra gas |  3.01% |  82.97% |  300 sierra gas |   3.01% | "u32_overflowing_sub" 
              300 sierra gas |  3.01% |  85.98% |  300 sierra gas |   3.01% | "withdraw_gas" 
              200 sierra gas |  2.00% |  87.98% |  200 sierra gas |   2.00% | "array_new" 
              200 sierra gas |  2.00% |  89.98% | 9883 sierra gas |  99.00% | "builtins_simple::tests::bitwise_cost" 
              200 sierra gas |  2.00% |  91.99% |  200 sierra gas |   2.00% | "get_builtin_costs" 
              200 sierra gas |  2.00% |  93.99% |  200 sierra gas |   2.00% | "jump" 
              200 sierra gas |  2.00% |  95.99% | 2900 sierra gas |  29.05% | "snforge_std::cheatcode::execute_cheatcode" 
              200 sierra gas |  2.00% |  98.00% | 5100 sierra gas |  51.09% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
              100 sierra gas |  1.00% |  99.00% | 9983 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
              100 sierra gas |  1.00% | 100.00% |  100 sierra gas |   1.00% | "bool_not_impl" 
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
            
                        flat |  flat% |    sum% |             cum |    cum% |  
            -----------------+--------+---------+-----------------+---------+----------------
             1 syscall usage | 50.00% |  50.00% | 2 syscall usage | 100.00% | "CallContract" 
             1 syscall usage | 50.00% | 100.00% | 1 syscall usage |  50.00% | "StorageRead" 
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

                        flat |  flat% |    sum% |             cum |    cum% |  
            -----------------+--------+---------+-----------------+---------+----------------
             1 syscall usage | 50.00% |  50.00% | 2 syscall usage | 100.00% | "CallContract" 
             1 syscall usage | 50.00% | 100.00% | 1 syscall usage |  50.00% | "StorageRead" 
            "#
        ));
}

#[test]
fn tree_more_deploys_without_constructor() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root
                .join("crates/cairo-profiler/tests/contracts/tree_verification/precompiled/"),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("mega_package_integrationtest_test_calls_test_call.json")
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
        .arg("1")
        .arg("--sample")
        .arg("syscall usage")
        .assert()
        .success()
        // snforge adds DeployWithoutConstructor types to nested_calls - we must make sure they are properly matched
        // there are 14 deploys in the nested calls, and there are 3 snforge deploys (syscall-less) in the code
        // which means there should be 11 deploy syscalls
        .stdout_eq(indoc!(
            r#"
            
            Showing nodes accounting for 11 syscall usage, 10.19% of 108 syscall usage total
            Showing top 1 nodes out of 35
            
                         flat |  flat% |   sum% |              cum |   cum% |  
            ------------------+--------+--------+------------------+--------+----------
             11 syscall usage | 10.19% | 10.19% | 28 syscall usage | 25.93% | "Deploy" 
            "#
        ));
}

#[test]
fn tree_more_nested_calls_than_triggers_happy() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root
                .join("crates/cairo-profiler/tests/contracts/tree_verification/precompiled/"),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("mega_package_integrationtest_test_erc20_test.json")
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
        .arg("6")
        .arg("--sample")
        .arg("syscall usage")
        .assert()
        .success()
        // snforge cheats some deploys - deploy will appear in nested_calls, but there won't be a syscall
        .stdout_eq(indoc!(
            r#"
            
            Showing nodes accounting for 19 syscall usage, 100.00% of 19 syscall usage total
            Showing top 6 nodes out of 22
            
                         flat |  flat% |    sum% |              cum |   cum% |  
            ------------------+--------+---------+------------------+--------+--------------------------------------------
             11 syscall usage | 57.89% |  57.89% | 11 syscall usage | 57.89% | "StorageWrite" 
              4 syscall usage | 21.05% |  78.95% |  4 syscall usage | 21.05% | "StorageRead" 
              2 syscall usage | 10.53% |  89.47% |  2 syscall usage | 10.53% | "EmitEvent" 
              1 syscall usage |  5.26% |  94.74% | 11 syscall usage | 57.89% | "CallContract" 
              1 syscall usage |  5.26% | 100.00% |  1 syscall usage |  5.26% | "GetExecutionInfo" 
              0 syscall usage |  0.00% | 100.00% |  8 syscall usage | 42.11% | "Contract: ERC20\nFunction: constructor\n" 
            "#
        ));
}

#[test]
fn tree_more_nested_calls_than_triggers_missing_call_contract() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root
                .join("crates/cairo-profiler/tests/contracts/tree_verification/precompiled/"),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("mega_package_more_calls_than_triggers.json")
        .assert()
        .failure()
        .stderr_eq(indoc!(
            r#"
            [WARNING] The trace file does not contain either one of calldata_len, signature_len or events_summary. This may lead to inaccurate l2 gas measurements. Consider using `snforge` >= `0.49.0`.
            [ERROR] There are no syscalls left in the program trace, but at least one unhandled call in trace file CallEntryPoint { class_hash: Some(ClassHash(0x117)), entry_point_type: External, entry_point_selector: EntryPointSelector(0x17340c6779204ea2a91c87d1c2226a3aebda65c64da3672a36893c4330ea27b), contract_address: ContractAddress(0x1724987234973219347210837402), call_type: Call, contract_name: Some("SNFORGE_TEST_CODE"), function_name: Some("SNFORGE_TEST_CODE_FUNCTION"), calldata_len: Some(0), events_summary: None, signature_len: None }!
            
            thread 'main' panicked at crates/cairo-profiler/src/trace_reader.rs:260:13:
            Too many EntryPointCalls for triggers
            note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
            "#
        ));
}

#[test]
fn tree_more_triggers_than_nested_calls() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root
                .join("crates/cairo-profiler/tests/contracts/tree_verification/precompiled/"),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("mega_package_more_triggers_than_calls.json")
        .assert()
        .failure()
        .stderr_eq(indoc!(
            "
            [WARNING] The trace file does not contain either one of calldata_len, signature_len or events_summary. This may lead to inaccurate l2 gas measurements. Consider using `snforge` >= `0.49.0`.
            [ERROR] Found syscall CallContract in the program trace, that do not have corresponding calls in trace file!
            
            thread 'main' panicked at crates/cairo-profiler/src/trace_reader.rs:209:17:
            Too few EntryPointCalls for triggers
            note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
            "
        ));
}

#[test]
fn tree_mismatched_syscall_with_entrypoint() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root
                .join("crates/cairo-profiler/tests/contracts/tree_verification/precompiled/"),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("mega_package_mismatched.json")
        .assert()
        .failure()
        .stderr_eq(indoc!(
            r#"
            [WARNING] The trace file does not contain either one of calldata_len, signature_len or events_summary. This may lead to inaccurate l2 gas measurements. Consider using `snforge` >= `0.49.0`.
            [ERROR] Found syscall CallContract in the program trace, that do not corresponds to the next call from trace file CallEntryPoint { class_hash: Some(ClassHash(0x117)), entry_point_type: External, entry_point_selector: EntryPointSelector(0x17340c6779204ea2a91c87d1c2226a3aebda65c64da3672a36893c4330ea27b), contract_address: ContractAddress(0x1724987234973219347210837402), call_type: Call, contract_name: Some("SNFORGE_TEST_CODE"), function_name: Some("SNFORGE_TEST_CODE_FUNCTION"), calldata_len: Some(0), events_summary: None, signature_len: None }!
            
            thread 'main' panicked at crates/cairo-profiler/src/trace_reader.rs:250:17:
            Trigger does not match entrypoint
            note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
            "#
        ));
}

#[test]
fn view_syscall_with_calldata_factor_multiple() {
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
            
            Showing nodes accounting for 515900 sierra gas, 100.00% of 515900 sierra gas total
            Showing top 21 nodes out of 21
            
                          flat |  flat% |    sum% |               cum |    cum% |  
            -------------------+--------+---------+-------------------+---------+-----------------------------------------------------------------------------
             246100 sierra gas | 47.70% |  47.70% | 465700 sierra gas |  90.27% | "Deploy" 
             107600 sierra gas | 20.86% |  68.56% | 107600 sierra gas |  20.86% | "core::keccak::finalize_padding" 
              40000 sierra gas |  7.75% |  76.31% |  40000 sierra gas |   7.75% | "Keccak" 
              36000 sierra gas |  6.98% |  83.29% |  36000 sierra gas |   6.98% | "core::keccak::keccak_u256s_le_inputs[637-804]" 
              13200 sierra gas |  2.56% |  85.85% | 120800 sierra gas |  23.42% | "core::keccak::add_padding" 
               8700 sierra gas |  1.69% |  87.54% |   8700 sierra gas |   1.69% | "snforge_std::cheatcode::execute_cheatcode" 
               7600 sierra gas |  1.47% |  89.01% |   7600 sierra gas |   1.47% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
               7100 sierra gas |  1.38% |  90.39% | 105500 sierra gas |  20.45% | "scaled_syscall::GasConstructorChecker::constructor" 
               7100 sierra gas |  1.38% |  91.76% | 105500 sierra gas |  20.45% | "scaled_syscall::GasConstructorCheckerButDifferent::constructor" 
               7000 sierra gas |  1.36% |  93.12% |  14600 sierra gas |   2.83% | "core::result::ResultSerde::deserialize" 
               6800 sierra gas |  1.32% |  94.44% |  31700 sierra gas |   6.14% | "snforge_std::cheatcodes::contract_class::declare" 
               6700 sierra gas |  1.30% |  95.74% | 513500 sierra gas |  99.53% | "scaled_syscall::deploy_syscall_cost_return_wrapper" 
               4900 sierra gas |  0.95% |  96.69% | 110400 sierra gas |  21.40% | "scaled_syscall::GasConstructorChecker::__wrapper__constructor" 
               4500 sierra gas |  0.87% |  97.56% |   4500 sierra gas |   0.87% | "core::array::serialize_array_helper" 
               4300 sierra gas |  0.83% |  98.39% | 252349 sierra gas |  48.91% | "scaled_syscall::declare_deploy_a_contract" 
               3500 sierra gas |  0.68% |  99.07% | 109000 sierra gas |  21.13% | "scaled_syscall::GasConstructorCheckerButDifferent::__wrapper__constructor" 
               2300 sierra gas |  0.45% |  99.52% | 515800 sierra gas |  99.98% | "scaled_syscall::deploy_syscall_cost" 
               2200 sierra gas |  0.43% |  99.94% |   5100 sierra gas |   0.99% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
                100 sierra gas |  0.02% |  99.96% | 110500 sierra gas |  21.42% | "Contract: GasConstructorChecker\nFunction: constructor\n" 
                100 sierra gas |  0.02% |  99.98% | 109100 sierra gas |  21.15% | "Contract: GasConstructorCheckerButDifferent\nFunction: constructor\n" 
                100 sierra gas |  0.02% | 100.00% | 515900 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
            "#
        ));
}

#[test]
fn view_syscall_with_calldata_factor_single() {
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
        .arg("scaled_syscall_deploy_syscall_cost_but_different.json")
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
            
            Showing nodes accounting for 262249 sierra gas, 100.00% of 262249 sierra gas total
            Showing top 17 nodes out of 17
            
                          flat |  flat% |    sum% |               cum |    cum% |  
            -------------------+--------+---------+-------------------+---------+-----------------------------------------------------------------------------
             122249 sierra gas | 46.62% |  46.62% | 231349 sierra gas |  88.22% | "Deploy" 
              53800 sierra gas | 20.51% |  67.13% |  53800 sierra gas |  20.51% | "core::keccak::finalize_padding" 
              20000 sierra gas |  7.63% |  74.76% |  20000 sierra gas |   7.63% | "Keccak" 
              18000 sierra gas |  6.86% |  81.62% |  18000 sierra gas |   6.86% | "core::keccak::keccak_u256s_le_inputs[637-804]" 
               7100 sierra gas |  2.71% |  84.33% | 105500 sierra gas |  40.23% | "scaled_syscall::GasConstructorCheckerButDifferent::constructor" 
               6700 sierra gas |  2.55% |  86.88% | 259849 sierra gas |  99.08% | "scaled_syscall::deploy_syscall_cost_but_different_return_wrapper" 
               6600 sierra gas |  2.52% |  89.40% |  60400 sierra gas |  23.03% | "core::keccak::add_padding" 
               5800 sierra gas |  2.21% |  91.61% |   5800 sierra gas |   2.21% | "snforge_std::cheatcode::execute_cheatcode" 
               3800 sierra gas |  1.45% |  93.06% |   3800 sierra gas |   1.45% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
               3500 sierra gas |  1.33% |  94.39% |   7300 sierra gas |   2.78% | "core::result::ResultSerde::deserialize" 
               3500 sierra gas |  1.33% |  95.73% | 109000 sierra gas |  41.56% | "scaled_syscall::GasConstructorCheckerButDifferent::__wrapper__constructor" 
               3400 sierra gas |  1.30% |  97.03% |  16700 sierra gas |   6.37% | "snforge_std::cheatcodes::contract_class::declare" 
               3100 sierra gas |  1.18% |  98.21% |   3100 sierra gas |   1.18% | "core::array::serialize_array_helper" 
               2300 sierra gas |  0.88% |  99.08% | 262149 sierra gas |  99.96% | "scaled_syscall::deploy_syscall_cost_but_different" 
               2200 sierra gas |  0.84% |  99.92% |   5100 sierra gas |   1.94% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
                100 sierra gas |  0.04% |  99.96% | 109100 sierra gas |  41.60% | "Contract: GasConstructorCheckerButDifferent\nFunction: constructor\n" 
                100 sierra gas |  0.04% | 100.00% | 262249 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
            "#
        ));
}

#[test]
fn view_syscall_with_no_calldata_factor() {
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
        .arg("scaled_syscall_test_increase_balance.json")
        .assert()
        .success()
        .stderr_eq(indoc!(
            r"
            [WARNING] The trace file does not contain either one of calldata_len, signature_len or events_summary. This may lead to inaccurate l2 gas measurements. Consider using `snforge` >= `0.49.0`.
            "
        ));

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
            
            Showing nodes accounting for 122188 sierra gas, 100.00% of 122188 sierra gas total
            Showing top 9 nodes out of 9
            
                         flat |  flat% |    sum% |               cum |    cum% |  
            ------------------+--------+---------+-------------------+---------+----------------------------------------------------------------------------------------------------------------------------------
             90388 sierra gas | 73.97% |  73.97% | 110388 sierra gas |  90.34% | "CallContract" 
             10000 sierra gas |  8.18% |  82.16% |  10000 sierra gas |   8.18% | "StorageRead" 
             10000 sierra gas |  8.18% |  90.34% |  10000 sierra gas |   8.18% | "StorageWrite" 
              4300 sierra gas |  3.52% |  93.86% | 119788 sierra gas |  98.04% | "scaled_syscall::test_increase_balance_return_wrapper" 
              2900 sierra gas |  2.37% |  96.24% |   2900 sierra gas |   2.37% | "snforge_std::cheatcode::execute_cheatcode" 
              2300 sierra gas |  1.88% |  98.12% | 122088 sierra gas |  99.92% | "scaled_syscall::test_increase_balance" 
              2200 sierra gas |  1.80% |  99.92% |   5100 sierra gas |   4.17% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
               100 sierra gas |  0.08% | 100.00% | 122188 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
                 0 sierra gas |  0.00% | 100.00% |  20000 sierra gas |  16.37% | "Contract: <unknown>\nAddress: 0x000fa8e78a86a612746455cfeb98012e67ec3426b41a20278d5e7237bcab7413\nFunction: increase_balance\n" 
            "#
        ));
}

#[test]
fn view_no_l2_gas_sample_for_steps() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root
                .join("crates/cairo-profiler/tests/contracts/l2_gas/precompiled_cairo_steps/"),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("l2_verification_integrationtest_test_l2_without_signature.json")
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
        !output_str.contains("l2 gas"),
        "Output contains: {}, 'l2 gas' is wrongly here",
        &output_str
    );

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("l2_verification_integrationtest_test_l2_with_signature.json")
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
        !output_str.contains("l2 gas"),
        "Output contains: {}, 'l2 gas' is wrongly here",
        &output_str
    );
}

#[test]
fn view_l2_gas_no_signature() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root
                .join("crates/cairo-profiler/tests/contracts/l2_gas/precompiled_sierra_gas/"),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("l2_verification_integrationtest_test_l2_without_signature.json")
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
        .arg("l2 gas")
        .assert()
        .success()
        .stdout_eq(indoc!(
            r#"
            
            Showing nodes accounting for 1461460 l2 gas, 100.00% of 1461460 l2 gas total
            Showing top 42 nodes out of 42
            
                      flat |  flat% |    sum% |            cum |    cum% |  
            ---------------+--------+---------+----------------+---------+---------------------------------------------------------------------------------
             723104 l2 gas | 49.48% |  49.48% | 1350740 l2 gas |  92.42% | "CallContract" 
             220000 l2 gas | 15.05% |  64.53% |  220000 l2 gas |  15.05% | "StorageRead" 
             203600 l2 gas | 13.93% |  78.46% |  203600 l2 gas |  13.93% | "EmitEvent" 
             190000 l2 gas | 13.00% |  91.46% |  190000 l2 gas |  13.00% | "StorageWrite" 
              37716 l2 gas |  2.58% |  94.04% |   37716 l2 gas |   2.58% | "GetExecutionInfo" 
              20480 l2 gas |  1.40% |  95.45% |   60480 l2 gas |   4.14% | "Contract: ERC20\nFunction: allowance\n" 
              20480 l2 gas |  1.40% |  96.85% |  234492 l2 gas |  16.05% | "Contract: ERC20\nFunction: transfer_from\n" 
              15360 l2 gas |  1.05% |  97.90% |   75360 l2 gas |   5.16% | "Contract: ERC20\nFunction: balance_of\n" 
              15360 l2 gas |  1.05% |  98.95% |  108652 l2 gas |   7.43% | "Contract: ERC20\nFunction: increase_allowance\n" 
              15360 l2 gas |  1.05% | 100.00% |  148652 l2 gas |  10.17% | "Contract: ERC20\nFunction: transfer\n" 
                  0 l2 gas |  0.00% | 100.00% |  110720 l2 gas |   7.58% | "Contract: ERC20\nFunction: constructor\n" 
                  0 l2 gas |  0.00% | 100.00% | 1461460 l2 gas | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "core::array::ArrayImpl" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "core::array::SpanFelt252Serde::deserialize" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "core::array::serialize_array_helper" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "core::result::ResultSerde::deserialize" 
                  0 l2 gas |  0.00% | 100.00% |  120000 l2 gas |   8.21% | "core::starknet::storage::MutableStorableStoragePointer0OffsetReadAccess::read" 
                  0 l2 gas |  0.00% | 100.00% |  100000 l2 gas |   6.84% | "core::starknet::storage::StorableStoragePointer0OffsetReadAccess::read" 
                  0 l2 gas |  0.00% | 100.00% |   93292 l2 gas |   6.38% | "l2_verification::erc20::ERC20::IERC20Impl::increase_allowance" 
                  0 l2 gas |  0.00% | 100.00% |  121440 l2 gas |   8.31% | "l2_verification::erc20::ERC20::StorageImpl::approve_helper" 
                  0 l2 gas |  0.00% | 100.00% |   80720 l2 gas |   5.52% | "l2_verification::erc20::ERC20::StorageImpl::spend_allowance" 
                  0 l2 gas |  0.00% | 100.00% |  241440 l2 gas |  16.52% | "l2_verification::erc20::ERC20::StorageImpl::transfer_helper" 
                  0 l2 gas |  0.00% | 100.00% |   40000 l2 gas |   2.74% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__allowance" 
                  0 l2 gas |  0.00% | 100.00% |   60000 l2 gas |   4.11% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__balance_of" 
                  0 l2 gas |  0.00% | 100.00% |   93292 l2 gas |   6.38% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__increase_allowance" 
                  0 l2 gas |  0.00% | 100.00% |  133292 l2 gas |   9.12% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__transfer" 
                  0 l2 gas |  0.00% | 100.00% |  214012 l2 gas |  14.64% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__transfer_from" 
                  0 l2 gas |  0.00% | 100.00% |  110720 l2 gas |   7.58% | "l2_verification::erc20::ERC20::__wrapper__constructor" 
                  0 l2 gas |  0.00% | 100.00% |  110720 l2 gas |   7.58% | "l2_verification::erc20::ERC20::constructor" 
                  0 l2 gas |  0.00% | 100.00% |  241256 l2 gas |  16.51% | "l2_verification::erc20::IERC20DispatcherImpl::allowance" 
                  0 l2 gas |  0.00% | 100.00% |  346524 l2 gas |  23.71% | "l2_verification::erc20::IERC20DispatcherImpl::balance_of" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "l2_verification_integrationtest::test_l2::deploy_erc20" 
                  0 l2 gas |  0.00% | 100.00% | 1350740 l2 gas |  92.42% | "l2_verification_integrationtest::test_l2::without_signature" 
                  0 l2 gas |  0.00% | 100.00% | 1350740 l2 gas |  92.42% | "l2_verification_integrationtest::test_l2::without_signature_return_wrapper" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "snforge_std::cheatcodes::execution_info::BlockInfoMockSerde::serialize" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "snforge_std::cheatcodes::execution_info::ExecutionInfoMockImpl::default" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "snforge_std::cheatcodes::execution_info::ExecutionInfoMockSerde::serialize" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "snforge_std::cheatcodes::execution_info::TxInfoMockImpl::default" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "snforge_std::cheatcodes::execution_info::TxInfoMockSerde::serialize" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "snforge_std::cheatcodes::execution_info::cheat_execution_info" 
            "#
        ));
}

#[test]
fn view_l2_gas_with_signature() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root
                .join("crates/cairo-profiler/tests/contracts/l2_gas/precompiled_sierra_gas/"),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("l2_verification_integrationtest_test_l2_with_signature.json")
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
        .arg("l2 gas")
        .assert()
        .success()
        .stdout_eq(indoc!(
            r#"
            
            Showing nodes accounting for 1502420 l2 gas, 100.00% of 1502420 l2 gas total
            Showing top 42 nodes out of 42
            
                      flat |  flat% |    sum% |            cum |    cum% |  
            ---------------+--------+---------+----------------+---------+---------------------------------------------------------------------------------
             723104 l2 gas | 48.13% |  48.13% | 1391700 l2 gas |  92.63% | "CallContract" 
             220000 l2 gas | 14.64% |  62.77% |  220000 l2 gas |  14.64% | "StorageRead" 
             203600 l2 gas | 13.55% |  76.32% |  203600 l2 gas |  13.55% | "EmitEvent" 
             190000 l2 gas | 12.65% |  88.97% |  190000 l2 gas |  12.65% | "StorageWrite" 
              37716 l2 gas |  2.51% |  91.48% |   37716 l2 gas |   2.51% | "GetExecutionInfo" 
              30720 l2 gas |  2.04% |  93.53% |   70720 l2 gas |   4.71% | "Contract: ERC20\nFunction: allowance\n" 
              30720 l2 gas |  2.04% |  95.57% |   90720 l2 gas |   6.04% | "Contract: ERC20\nFunction: balance_of\n" 
              25600 l2 gas |  1.70% |  97.27% |  239612 l2 gas |  15.95% | "Contract: ERC20\nFunction: transfer_from\n" 
              20480 l2 gas |  1.36% |  98.64% |  113772 l2 gas |   7.57% | "Contract: ERC20\nFunction: increase_allowance\n" 
              20480 l2 gas |  1.36% | 100.00% |  153772 l2 gas |  10.23% | "Contract: ERC20\nFunction: transfer\n" 
                  0 l2 gas |  0.00% | 100.00% |  110720 l2 gas |   7.37% | "Contract: ERC20\nFunction: constructor\n" 
                  0 l2 gas |  0.00% | 100.00% | 1502420 l2 gas | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "core::array::ArrayImpl" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "core::array::SpanFelt252Serde::deserialize" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "core::array::serialize_array_helper" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "core::result::ResultSerde::deserialize" 
                  0 l2 gas |  0.00% | 100.00% |  120000 l2 gas |   7.99% | "core::starknet::storage::MutableStorableStoragePointer0OffsetReadAccess::read" 
                  0 l2 gas |  0.00% | 100.00% |  100000 l2 gas |   6.66% | "core::starknet::storage::StorableStoragePointer0OffsetReadAccess::read" 
                  0 l2 gas |  0.00% | 100.00% |   93292 l2 gas |   6.21% | "l2_verification::erc20::ERC20::IERC20Impl::increase_allowance" 
                  0 l2 gas |  0.00% | 100.00% |  121440 l2 gas |   8.08% | "l2_verification::erc20::ERC20::StorageImpl::approve_helper" 
                  0 l2 gas |  0.00% | 100.00% |   80720 l2 gas |   5.37% | "l2_verification::erc20::ERC20::StorageImpl::spend_allowance" 
                  0 l2 gas |  0.00% | 100.00% |  241440 l2 gas |  16.07% | "l2_verification::erc20::ERC20::StorageImpl::transfer_helper" 
                  0 l2 gas |  0.00% | 100.00% |   40000 l2 gas |   2.66% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__allowance" 
                  0 l2 gas |  0.00% | 100.00% |   60000 l2 gas |   3.99% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__balance_of" 
                  0 l2 gas |  0.00% | 100.00% |   93292 l2 gas |   6.21% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__increase_allowance" 
                  0 l2 gas |  0.00% | 100.00% |  133292 l2 gas |   8.87% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__transfer" 
                  0 l2 gas |  0.00% | 100.00% |  214012 l2 gas |  14.24% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__transfer_from" 
                  0 l2 gas |  0.00% | 100.00% |  110720 l2 gas |   7.37% | "l2_verification::erc20::ERC20::__wrapper__constructor" 
                  0 l2 gas |  0.00% | 100.00% |  110720 l2 gas |   7.37% | "l2_verification::erc20::ERC20::constructor" 
                  0 l2 gas |  0.00% | 100.00% |  251496 l2 gas |  16.74% | "l2_verification::erc20::IERC20DispatcherImpl::allowance" 
                  0 l2 gas |  0.00% | 100.00% |  361884 l2 gas |  24.09% | "l2_verification::erc20::IERC20DispatcherImpl::balance_of" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "l2_verification_integrationtest::test_l2::deploy_erc20" 
                  0 l2 gas |  0.00% | 100.00% | 1391700 l2 gas |  92.63% | "l2_verification_integrationtest::test_l2::with_signature" 
                  0 l2 gas |  0.00% | 100.00% | 1391700 l2 gas |  92.63% | "l2_verification_integrationtest::test_l2::with_signature_return_wrapper" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "snforge_std::cheatcodes::execution_info::BlockInfoMockSerde::serialize" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "snforge_std::cheatcodes::execution_info::ExecutionInfoMockImpl::default" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "snforge_std::cheatcodes::execution_info::ExecutionInfoMockSerde::serialize" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "snforge_std::cheatcodes::execution_info::TxInfoMockImpl::default" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "snforge_std::cheatcodes::execution_info::TxInfoMockSerde::serialize" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "snforge_std::cheatcodes::execution_info::cheat_execution_info" 
            "#
        ));
}
