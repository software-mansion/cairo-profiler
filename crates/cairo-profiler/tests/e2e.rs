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

            Showing nodes accounting for 152560 sierra gas, 100.00% of 152560 sierra gas total
            Showing top 15 nodes out of 15
            
                         flat |  flat% |    sum% |               cum |    cum% |  
            ------------------+--------+---------+-------------------+---------+--------------------------------------------------------------------------------------------------------------
             91560 sierra gas | 60.02% |  60.02% | 105360 sierra gas |  69.06% | "CallContract" 
             10200 sierra gas |  6.69% |  66.70% |  17900 sierra gas |  11.73% | "core::result::ResultSerde::deserialize" 
             10000 sierra gas |  6.55% |  73.26% |  10000 sierra gas |   6.55% | "StorageRead" 
              8700 sierra gas |  5.70% |  78.96% |   8700 sierra gas |   5.70% | "snforge_std::cheatcode::execute_cheatcode" 
              6400 sierra gas |  4.20% |  83.15% | 150160 sierra gas |  98.43% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value_return_wrapper" 
              3900 sierra gas |  2.56% |  85.71% |   3900 sierra gas |   2.56% | "core::array::SpanFelt252Serde::deserialize" 
              3800 sierra gas |  2.49% |  88.20% |   3800 sierra gas |   2.49% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
              3700 sierra gas |  2.43% |  90.63% |  13700 sierra gas |   8.98% | "balance_simple::HelloStarknet::__wrapper__HelloStarknetImpl__get_balance" 
              3400 sierra gas |  2.23% |  92.86% |  18300 sierra gas |  12.00% | "snforge_std::cheatcodes::contract_class::ContractClassImpl::deploy" 
              3400 sierra gas |  2.23% |  95.08% |  15000 sierra gas |   9.83% | "snforge_std::cheatcodes::contract_class::declare" 
              2800 sierra gas |  1.84% |  96.92% |   2800 sierra gas |   1.84% | "core::array::serialize_array_helper" 
              2300 sierra gas |  1.51% |  98.43% | 152460 sierra gas |  99.93% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value" 
              2200 sierra gas |  1.44% |  99.87% |   5100 sierra gas |   3.34% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
               100 sierra gas |  0.07% |  99.93% |  13800 sierra gas |   9.05% | "Contract: HelloStarknet\nFunction: get_balance\n" 
               100 sierra gas |  0.07% | 100.00% | 152560 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
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
            Showing top 1 nodes out of 38
            
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
            Showing top 6 nodes out of 25
            
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
            
            thread 'main' panicked at crates/cairo-profiler/src/trace_reader.rs:267:13:
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
            
            thread 'main' panicked at crates/cairo-profiler/src/trace_reader.rs:213:17:
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
            
            thread 'main' panicked at crates/cairo-profiler/src/trace_reader.rs:257:17:
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
            
            Showing nodes accounting for 583440 sierra gas, 100.00% of 583440 sierra gas total
            Showing top 21 nodes out of 21
            
                          flat |  flat% |    sum% |               cum |    cum% |  
            -------------------+--------+---------+-------------------+---------+-----------------------------------------------------------------------------
             313640 sierra gas | 53.76% |  53.76% | 533240 sierra gas |  91.40% | "Deploy" 
             107600 sierra gas | 18.44% |  72.20% | 107600 sierra gas |  18.44% | "core::keccak::finalize_padding" 
              40000 sierra gas |  6.86% |  79.06% |  40000 sierra gas |   6.86% | "Keccak" 
              36000 sierra gas |  6.17% |  85.23% |  36000 sierra gas |   6.17% | "core::keccak::keccak_u256s_le_inputs[637-804]" 
              13200 sierra gas |  2.26% |  87.49% | 120800 sierra gas |  20.70% | "core::keccak::add_padding" 
               8700 sierra gas |  1.49% |  88.98% |   8700 sierra gas |   1.49% | "snforge_std::cheatcode::execute_cheatcode" 
               7600 sierra gas |  1.30% |  90.28% |   7600 sierra gas |   1.30% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
               7100 sierra gas |  1.22% |  91.50% | 105500 sierra gas |  18.08% | "scaled_syscall::GasConstructorChecker::constructor" 
               7100 sierra gas |  1.22% |  92.72% | 105500 sierra gas |  18.08% | "scaled_syscall::GasConstructorCheckerButDifferent::constructor" 
               7000 sierra gas |  1.20% |  93.92% |  14600 sierra gas |   2.50% | "core::result::ResultSerde::deserialize" 
               6800 sierra gas |  1.17% |  95.08% |  31700 sierra gas |   5.43% | "snforge_std::cheatcodes::contract_class::declare" 
               6700 sierra gas |  1.15% |  96.23% | 581040 sierra gas |  99.59% | "scaled_syscall::deploy_syscall_cost_return_wrapper" 
               4900 sierra gas |  0.84% |  97.07% | 110400 sierra gas |  18.92% | "scaled_syscall::GasConstructorChecker::__wrapper__constructor" 
               4500 sierra gas |  0.77% |  97.84% |   4500 sierra gas |   0.77% | "core::array::serialize_array_helper" 
               4300 sierra gas |  0.74% |  98.58% | 282070 sierra gas |  48.35% | "scaled_syscall::declare_deploy_a_contract" 
               3500 sierra gas |  0.60% |  99.18% | 109000 sierra gas |  18.68% | "scaled_syscall::GasConstructorCheckerButDifferent::__wrapper__constructor" 
               2300 sierra gas |  0.39% |  99.57% | 583340 sierra gas |  99.98% | "scaled_syscall::deploy_syscall_cost" 
               2200 sierra gas |  0.38% |  99.95% |   5100 sierra gas |   0.87% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
                100 sierra gas |  0.02% |  99.97% | 110500 sierra gas |  18.94% | "Contract: GasConstructorChecker\nFunction: constructor\n" 
                100 sierra gas |  0.02% |  99.98% | 109100 sierra gas |  18.70% | "Contract: GasConstructorCheckerButDifferent\nFunction: constructor\n" 
                100 sierra gas |  0.02% | 100.00% | 583440 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
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
            
            Showing nodes accounting for 291970 sierra gas, 100.00% of 291970 sierra gas total
            Showing top 17 nodes out of 17
            
                          flat |  flat% |    sum% |               cum |    cum% |  
            -------------------+--------+---------+-------------------+---------+-----------------------------------------------------------------------------
             151970 sierra gas | 52.05% |  52.05% | 261070 sierra gas |  89.42% | "Deploy" 
              53800 sierra gas | 18.43% |  70.48% |  53800 sierra gas |  18.43% | "core::keccak::finalize_padding" 
              20000 sierra gas |  6.85% |  77.33% |  20000 sierra gas |   6.85% | "Keccak" 
              18000 sierra gas |  6.17% |  83.49% |  18000 sierra gas |   6.17% | "core::keccak::keccak_u256s_le_inputs[637-804]" 
               7100 sierra gas |  2.43% |  85.92% | 105500 sierra gas |  36.13% | "scaled_syscall::GasConstructorCheckerButDifferent::constructor" 
               6700 sierra gas |  2.29% |  88.22% | 289570 sierra gas |  99.18% | "scaled_syscall::deploy_syscall_cost_but_different_return_wrapper" 
               6600 sierra gas |  2.26% |  90.48% |  60400 sierra gas |  20.69% | "core::keccak::add_padding" 
               5800 sierra gas |  1.99% |  92.46% |   5800 sierra gas |   1.99% | "snforge_std::cheatcode::execute_cheatcode" 
               3800 sierra gas |  1.30% |  93.77% |   3800 sierra gas |   1.30% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
               3500 sierra gas |  1.20% |  94.97% |   7300 sierra gas |   2.50% | "core::result::ResultSerde::deserialize" 
               3500 sierra gas |  1.20% |  96.16% | 109000 sierra gas |  37.33% | "scaled_syscall::GasConstructorCheckerButDifferent::__wrapper__constructor" 
               3400 sierra gas |  1.16% |  97.33% |  16700 sierra gas |   5.72% | "snforge_std::cheatcodes::contract_class::declare" 
               3100 sierra gas |  1.06% |  98.39% |   3100 sierra gas |   1.06% | "core::array::serialize_array_helper" 
               2300 sierra gas |  0.79% |  99.18% | 291870 sierra gas |  99.97% | "scaled_syscall::deploy_syscall_cost_but_different" 
               2200 sierra gas |  0.75% |  99.93% |   5100 sierra gas |   1.75% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
                100 sierra gas |  0.03% |  99.97% | 109100 sierra gas |  37.37% | "Contract: GasConstructorCheckerButDifferent\nFunction: constructor\n" 
                100 sierra gas |  0.03% | 100.00% | 291970 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
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
            
            Showing nodes accounting for 123360 sierra gas, 100.00% of 123360 sierra gas total
            Showing top 9 nodes out of 9
            
                         flat |  flat% |    sum% |               cum |    cum% |  
            ------------------+--------+---------+-------------------+---------+----------------------------------------------------------------------------------------------------------------------------------
             91560 sierra gas | 74.22% |  74.22% | 111560 sierra gas |  90.43% | "CallContract" 
             10000 sierra gas |  8.11% |  82.33% |  10000 sierra gas |   8.11% | "StorageRead" 
             10000 sierra gas |  8.11% |  90.43% |  10000 sierra gas |   8.11% | "StorageWrite" 
              4300 sierra gas |  3.49% |  93.92% | 120960 sierra gas |  98.05% | "scaled_syscall::test_increase_balance_return_wrapper" 
              2900 sierra gas |  2.35% |  96.27% |   2900 sierra gas |   2.35% | "snforge_std::cheatcode::execute_cheatcode" 
              2300 sierra gas |  1.86% |  98.14% | 123260 sierra gas |  99.92% | "scaled_syscall::test_increase_balance" 
              2200 sierra gas |  1.78% |  99.92% |   5100 sierra gas |   4.13% | "snforge_std::cheatcode::execute_cheatcode_and_deserialize" 
               100 sierra gas |  0.08% | 100.00% | 123360 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
                 0 sierra gas |  0.00% | 100.00% |  20000 sierra gas |  16.21% | "Contract: <unknown>\nAddress: 0x000fa8e78a86a612746455cfeb98012e67ec3426b41a20278d5e7237bcab7413\nFunction: increase_balance\n" 
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
            
            Showing nodes accounting for 1010060 l2 gas, 100.00% of 1010060 l2 gas total
            Showing top 42 nodes out of 42
            
                      flat |  flat% |    sum% |            cum |    cum% |  
            ---------------+--------+---------+----------------+---------+---------------------------------------------------------------------------------
             220000 l2 gas | 21.78% |  21.78% |  220000 l2 gas |  21.78% | "StorageRead" 
             203600 l2 gas | 20.16% |  41.94% |  203600 l2 gas |  20.16% | "EmitEvent" 
             190000 l2 gas | 18.81% |  60.75% |  190000 l2 gas |  18.81% | "StorageWrite" 
              69800 l2 gas |  6.91% |  67.66% |  324440 l2 gas |  32.12% | "l2_verification::erc20::ERC20::StorageImpl::transfer_helper" 
              37920 l2 gas |  3.75% |  71.41% |   37920 l2 gas |   3.75% | "GetExecutionInfo" 
              31400 l2 gas |  3.11% |  74.52% |  152840 l2 gas |  15.13% | "l2_verification::erc20::ERC20::StorageImpl::approve_helper" 
              30800 l2 gas |  3.05% |  77.57% |   77400 l2 gas |   7.66% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__allowance" 
              29250 l2 gas |  2.90% |  80.47% |   99150 l2 gas |   9.82% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__balance_of" 
              20480 l2 gas |  2.03% |  82.50% |   97880 l2 gas |   9.69% | "Contract: ERC20\nFunction: allowance\n" 
              20480 l2 gas |  2.03% |  84.52% |  317660 l2 gas |  31.45% | "Contract: ERC20\nFunction: transfer_from\n" 
              19800 l2 gas |  1.96% |  86.48% |  139800 l2 gas |  13.84% | "core::starknet::storage::MutableStorableStoragePointer0OffsetReadAccess::read" 
              17050 l2 gas |  1.69% |  88.17% |  127770 l2 gas |  12.65% | "l2_verification::erc20::ERC20::constructor" 
              16500 l2 gas |  1.63% |  89.80% |  116500 l2 gas |  11.53% | "core::starknet::storage::StorableStoragePointer0OffsetReadAccess::read" 
              15360 l2 gas |  1.52% |  91.33% |  114510 l2 gas |  11.34% | "Contract: ERC20\nFunction: balance_of\n" 
              15360 l2 gas |  1.52% |  92.85% |  147620 l2 gas |  14.61% | "Contract: ERC20\nFunction: increase_allowance\n" 
              15360 l2 gas |  1.52% |  94.37% |  196820 l2 gas |  19.49% | "Contract: ERC20\nFunction: transfer\n" 
              14000 l2 gas |  1.39% |  95.75% |  126360 l2 gas |  12.51% | "l2_verification::erc20::ERC20::IERC20Impl::increase_allowance" 
              13800 l2 gas |  1.37% |  97.12% |  113520 l2 gas |  11.24% | "l2_verification::erc20::ERC20::StorageImpl::spend_allowance" 
               8800 l2 gas |  0.87% |  97.99% |  297180 l2 gas |  29.42% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__transfer_from" 
               7800 l2 gas |  0.77% |  98.76% |  135570 l2 gas |  13.42% | "l2_verification::erc20::ERC20::__wrapper__constructor" 
               6600 l2 gas |  0.65% |  99.42% |  181460 l2 gas |  17.97% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__transfer" 
               5900 l2 gas |  0.58% | 100.00% |  132260 l2 gas |  13.09% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__increase_allowance" 
                  0 l2 gas |  0.00% | 100.00% |  874490 l2 gas |  86.58% | "CallContract" 
                  0 l2 gas |  0.00% | 100.00% |  135570 l2 gas |  13.42% | "Contract: ERC20\nFunction: constructor\n" 
                  0 l2 gas |  0.00% | 100.00% | 1010060 l2 gas | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "core::array::ArrayImpl" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "core::array::SpanFelt252Serde::deserialize" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "core::array::serialize_array_helper" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "core::result::ResultSerde::deserialize" 
                  0 l2 gas |  0.00% | 100.00% |   97880 l2 gas |   9.69% | "l2_verification::erc20::IERC20DispatcherImpl::allowance" 
                  0 l2 gas |  0.00% | 100.00% |  114510 l2 gas |  11.34% | "l2_verification::erc20::IERC20DispatcherImpl::balance_of" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "l2_verification_integrationtest::test_l2::deploy_erc20" 
                  0 l2 gas |  0.00% | 100.00% |  874490 l2 gas |  86.58% | "l2_verification_integrationtest::test_l2::without_signature" 
                  0 l2 gas |  0.00% | 100.00% |  874490 l2 gas |  86.58% | "l2_verification_integrationtest::test_l2::without_signature_return_wrapper" 
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
            
            Showing nodes accounting for 1051020 l2 gas, 100.00% of 1051020 l2 gas total
            Showing top 42 nodes out of 42
            
                      flat |  flat% |    sum% |            cum |    cum% |  
            ---------------+--------+---------+----------------+---------+---------------------------------------------------------------------------------
             220000 l2 gas | 20.93% |  20.93% |  220000 l2 gas |  20.93% | "StorageRead" 
             203600 l2 gas | 19.37% |  40.30% |  203600 l2 gas |  19.37% | "EmitEvent" 
             190000 l2 gas | 18.08% |  58.38% |  190000 l2 gas |  18.08% | "StorageWrite" 
              69800 l2 gas |  6.64% |  65.02% |  324440 l2 gas |  30.87% | "l2_verification::erc20::ERC20::StorageImpl::transfer_helper" 
              37920 l2 gas |  3.61% |  68.63% |   37920 l2 gas |   3.61% | "GetExecutionInfo" 
              31400 l2 gas |  2.99% |  71.62% |  152840 l2 gas |  14.54% | "l2_verification::erc20::ERC20::StorageImpl::approve_helper" 
              30800 l2 gas |  2.93% |  74.55% |   77400 l2 gas |   7.36% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__allowance" 
              30720 l2 gas |  2.92% |  77.47% |  108120 l2 gas |  10.29% | "Contract: ERC20\nFunction: allowance\n" 
              30720 l2 gas |  2.92% |  80.39% |  129870 l2 gas |  12.36% | "Contract: ERC20\nFunction: balance_of\n" 
              29250 l2 gas |  2.78% |  83.18% |   99150 l2 gas |   9.43% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__balance_of" 
              25600 l2 gas |  2.44% |  85.61% |  322780 l2 gas |  30.71% | "Contract: ERC20\nFunction: transfer_from\n" 
              20480 l2 gas |  1.95% |  87.56% |  152740 l2 gas |  14.53% | "Contract: ERC20\nFunction: increase_allowance\n" 
              20480 l2 gas |  1.95% |  89.51% |  201940 l2 gas |  19.21% | "Contract: ERC20\nFunction: transfer\n" 
              19800 l2 gas |  1.88% |  91.39% |  139800 l2 gas |  13.30% | "core::starknet::storage::MutableStorableStoragePointer0OffsetReadAccess::read" 
              17050 l2 gas |  1.62% |  93.02% |  127770 l2 gas |  12.16% | "l2_verification::erc20::ERC20::constructor" 
              16500 l2 gas |  1.57% |  94.59% |  116500 l2 gas |  11.08% | "core::starknet::storage::StorableStoragePointer0OffsetReadAccess::read" 
              14000 l2 gas |  1.33% |  95.92% |  126360 l2 gas |  12.02% | "l2_verification::erc20::ERC20::IERC20Impl::increase_allowance" 
              13800 l2 gas |  1.31% |  97.23% |  113520 l2 gas |  10.80% | "l2_verification::erc20::ERC20::StorageImpl::spend_allowance" 
               8800 l2 gas |  0.84% |  98.07% |  297180 l2 gas |  28.28% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__transfer_from" 
               7800 l2 gas |  0.74% |  98.81% |  135570 l2 gas |  12.90% | "l2_verification::erc20::ERC20::__wrapper__constructor" 
               6600 l2 gas |  0.63% |  99.44% |  181460 l2 gas |  17.27% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__transfer" 
               5900 l2 gas |  0.56% | 100.00% |  132260 l2 gas |  12.58% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__increase_allowance" 
                  0 l2 gas |  0.00% | 100.00% |  915450 l2 gas |  87.10% | "CallContract" 
                  0 l2 gas |  0.00% | 100.00% |  135570 l2 gas |  12.90% | "Contract: ERC20\nFunction: constructor\n" 
                  0 l2 gas |  0.00% | 100.00% | 1051020 l2 gas | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "core::array::ArrayImpl" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "core::array::SpanFelt252Serde::deserialize" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "core::array::serialize_array_helper" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "core::result::ResultSerde::deserialize" 
                  0 l2 gas |  0.00% | 100.00% |  108120 l2 gas |  10.29% | "l2_verification::erc20::IERC20DispatcherImpl::allowance" 
                  0 l2 gas |  0.00% | 100.00% |  129870 l2 gas |  12.36% | "l2_verification::erc20::IERC20DispatcherImpl::balance_of" 
                  0 l2 gas |  0.00% | 100.00% |       0 l2 gas |   0.00% | "l2_verification_integrationtest::test_l2::deploy_erc20" 
                  0 l2 gas |  0.00% | 100.00% |  915450 l2 gas |  87.10% | "l2_verification_integrationtest::test_l2::with_signature" 
                  0 l2 gas |  0.00% | 100.00% |  915450 l2 gas |  87.10% | "l2_verification_integrationtest::test_l2::with_signature_return_wrapper" 
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
