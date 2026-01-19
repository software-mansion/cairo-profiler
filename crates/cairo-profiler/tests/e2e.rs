use assert_fs::fixture::PathCopy;
use indoc::indoc;
use snapbox::cargo_bin;
use snapbox::cmd::Command as SnapboxCommand;
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
               1 steps |  0.07% |  99.93% |  128 steps |   8.52% | "Contract: HelloStarknet/nFunction: get_balance/n" 
               1 steps |  0.07% | 100.00% | 1503 steps | 100.00% | "Contract: SNFORGE_TEST_CODE/nFunction: SNFORGE_TEST_CODE_FUNCTION/n" 
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
             21 range check builtin | 50.00% |  50.00% | 42 range check builtin | 100.00% | "Contract: SNFORGE_TEST_CODE/nFunction: SNFORGE_TEST_CODE_FUNCTION/n" 
             18 range check builtin | 42.86% |  92.86% | 21 range check builtin |  50.00% | "CallContract" 
              2 range check builtin |  4.76% |  97.62% |  3 range check builtin |   7.14% | "Contract: HelloStarknet/nFunction: get_balance/n" 
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
               1 steps |  0.07% |  99.93% |  128 steps |   8.52% | "Contract: HelloStarknet/nFunction: get_balance/n" 
               1 steps |  0.07% | 100.00% | 1503 steps | 100.00% | "Contract: SNFORGE_TEST_CODE/nFunction: SNFORGE_TEST_CODE_FUNCTION/n" 
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
               1 steps |  0.07% |  99.93% |  128 steps |   8.52% | "Contract: HelloStarknet/nFunction: get_balance/n" 
               1 steps |  0.07% | 100.00% | 1503 steps | 100.00% | "Contract: SNFORGE_TEST_CODE/nFunction: SNFORGE_TEST_CODE_FUNCTION/n" 
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
               100 sierra gas |  0.07% |  99.93% |  13800 sierra gas |   9.05% | "Contract: HelloStarknet/nFunction: get_balance/n" 
               100 sierra gas |  0.07% | 100.00% | 152560 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE/nFunction: SNFORGE_TEST_CODE_FUNCTION/n" 
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
              100 sierra gas |  0.74% | 100.00% | 13450 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE/nFunction: SNFORGE_TEST_CODE_FUNCTION/n" 
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
              100 sierra gas |  1.00% |  99.00% | 9983 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE/nFunction: SNFORGE_TEST_CODE_FUNCTION/n" 
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
              0 casm size |  0.00% | 100.00% | 144 casm size | 100.00% | "Contract: SNFORGE_TEST_CODE/nFunction: SNFORGE_TEST_CODE_FUNCTION/n" 
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
              0 casm size |  0.00% | 100.00% | 144 casm size | 100.00% | "Contract: SNFORGE_TEST_CODE/nFunction: SNFORGE_TEST_CODE_FUNCTION/n" 
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
            
            Showing nodes accounting for 11 syscall usage, 11.34% of 97 syscall usage total
            Showing top 1 nodes out of 38
            
                         flat |  flat% |   sum% |              cum |   cum% |  
            ------------------+--------+--------+------------------+--------+----------
             11 syscall usage | 11.34% | 11.34% | 26 syscall usage | 26.80% | "Deploy" 
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
              0 syscall usage |  0.00% | 100.00% |  8 syscall usage | 42.11% | "Contract: ERC20/nFunction: constructor/n" 
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
            
            thread 'main' panicked at crates/cairo-profiler/src/trace_reader.rs:269:13:
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
            
            thread 'main' panicked at crates/cairo-profiler/src/trace_reader.rs:215:17:
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
            
            thread 'main' panicked at crates/cairo-profiler/src/trace_reader.rs:259:17:
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
                100 sierra gas |  0.02% |  99.97% | 110500 sierra gas |  18.94% | "Contract: GasConstructorChecker/nFunction: constructor/n" 
                100 sierra gas |  0.02% |  99.98% | 109100 sierra gas |  18.70% | "Contract: GasConstructorCheckerButDifferent/nFunction: constructor/n" 
                100 sierra gas |  0.02% | 100.00% | 583440 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE/nFunction: SNFORGE_TEST_CODE_FUNCTION/n" 
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
                100 sierra gas |  0.03% |  99.97% | 109100 sierra gas |  37.37% | "Contract: GasConstructorCheckerButDifferent/nFunction: constructor/n" 
                100 sierra gas |  0.03% | 100.00% | 291970 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE/nFunction: SNFORGE_TEST_CODE_FUNCTION/n" 
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
               100 sierra gas |  0.08% | 100.00% | 123360 sierra gas | 100.00% | "Contract: SNFORGE_TEST_CODE/nFunction: SNFORGE_TEST_CODE_FUNCTION/n" 
                 0 sierra gas |  0.00% | 100.00% |  20000 sierra gas |  16.21% | "Contract: <unknown>/nAddress: 0x000fa8e78a86a612746455cfeb98012e67ec3426b41a20278d5e7237bcab7413/nFunction: increase_balance/n" 
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
              20480 l2 gas |  2.03% |  82.50% |   97880 l2 gas |   9.69% | "Contract: ERC20/nFunction: allowance/n" 
              20480 l2 gas |  2.03% |  84.52% |  317660 l2 gas |  31.45% | "Contract: ERC20/nFunction: transfer_from/n" 
              19800 l2 gas |  1.96% |  86.48% |  139800 l2 gas |  13.84% | "core::starknet::storage::MutableStorableStoragePointer0OffsetReadAccess::read" 
              17050 l2 gas |  1.69% |  88.17% |  127770 l2 gas |  12.65% | "l2_verification::erc20::ERC20::constructor" 
              16500 l2 gas |  1.63% |  89.80% |  116500 l2 gas |  11.53% | "core::starknet::storage::StorableStoragePointer0OffsetReadAccess::read" 
              15360 l2 gas |  1.52% |  91.33% |  114510 l2 gas |  11.34% | "Contract: ERC20/nFunction: balance_of/n" 
              15360 l2 gas |  1.52% |  92.85% |  147620 l2 gas |  14.61% | "Contract: ERC20/nFunction: increase_allowance/n" 
              15360 l2 gas |  1.52% |  94.37% |  196820 l2 gas |  19.49% | "Contract: ERC20/nFunction: transfer/n" 
              14000 l2 gas |  1.39% |  95.75% |  126360 l2 gas |  12.51% | "l2_verification::erc20::ERC20::IERC20Impl::increase_allowance" 
              13800 l2 gas |  1.37% |  97.12% |  113520 l2 gas |  11.24% | "l2_verification::erc20::ERC20::StorageImpl::spend_allowance" 
               8800 l2 gas |  0.87% |  97.99% |  297180 l2 gas |  29.42% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__transfer_from" 
               7800 l2 gas |  0.77% |  98.76% |  135570 l2 gas |  13.42% | "l2_verification::erc20::ERC20::__wrapper__constructor" 
               6600 l2 gas |  0.65% |  99.42% |  181460 l2 gas |  17.97% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__transfer" 
               5900 l2 gas |  0.58% | 100.00% |  132260 l2 gas |  13.09% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__increase_allowance" 
                  0 l2 gas |  0.00% | 100.00% |  874490 l2 gas |  86.58% | "CallContract" 
                  0 l2 gas |  0.00% | 100.00% |  135570 l2 gas |  13.42% | "Contract: ERC20/nFunction: constructor/n" 
                  0 l2 gas |  0.00% | 100.00% | 1010060 l2 gas | 100.00% | "Contract: SNFORGE_TEST_CODE/nFunction: SNFORGE_TEST_CODE_FUNCTION/n" 
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
              30720 l2 gas |  2.92% |  77.47% |  108120 l2 gas |  10.29% | "Contract: ERC20/nFunction: allowance/n" 
              30720 l2 gas |  2.92% |  80.39% |  129870 l2 gas |  12.36% | "Contract: ERC20/nFunction: balance_of/n" 
              29250 l2 gas |  2.78% |  83.18% |   99150 l2 gas |   9.43% | "l2_verification::erc20::ERC20::__wrapper__IERC20Impl__balance_of" 
              25600 l2 gas |  2.44% |  85.61% |  322780 l2 gas |  30.71% | "Contract: ERC20/nFunction: transfer_from/n" 
              20480 l2 gas |  1.95% |  87.56% |  152740 l2 gas |  14.53% | "Contract: ERC20/nFunction: increase_allowance/n" 
              20480 l2 gas |  1.95% |  89.51% |  201940 l2 gas |  19.21% | "Contract: ERC20/nFunction: transfer/n" 
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
                  0 l2 gas |  0.00% | 100.00% |  135570 l2 gas |  12.90% | "Contract: ERC20/nFunction: constructor/n" 
                  0 l2 gas |  0.00% | 100.00% | 1051020 l2 gas | 100.00% | "Contract: SNFORGE_TEST_CODE/nFunction: SNFORGE_TEST_CODE_FUNCTION/n" 
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

#[test]
fn view_execute_simple_standalone_steps() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join(
                "crates/cairo-profiler/tests/executable_programs/simple_nameless/precompiled/",
            ),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("standalone_trace.json")
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
        .arg("steps")
        .assert()
        .success()
        .stdout_eq(indoc!(
            r#"
            
            Showing nodes accounting for 201 steps, 100.00% of 201 steps total
            Showing top 28 nodes out of 28
            
                 flat |  flat% |    sum% |       cum |    cum% |  
            ----------+--------+---------+-----------+---------+------------------------------------------------------------------
             94 steps | 46.77% |  46.77% |  94 steps |  46.77% | "store_temp" 
             13 steps |  6.47% |  53.23% | 201 steps | 100.00% | "SCARB_EXECUTE/nTarget: standalone/nFunction: <unknown>" 
             11 steps |  5.47% |  58.71% |  11 steps |   5.47% | "u8_overflowing_sub" 
             10 steps |  4.98% |  63.68% |  10 steps |   4.98% | "u32_overflowing_sub" 
              7 steps |  3.48% |  67.16% |   7 steps |   3.48% | "enum_match" 
              7 steps |  3.48% |  70.65% |   7 steps |   3.48% | "u8_safe_divmod" 
              6 steps |  2.99% |  73.63% |   6 steps |   2.99% | "array_snapshot_pop_front" 
              5 steps |  2.49% |  76.12% |   5 steps |   2.49% | "array_append" 
              5 steps |  2.49% |  78.61% |   5 steps |   2.49% | "array_snapshot_pop_back" 
              4 steps |  1.99% |  80.60% | 188 steps |  93.53% | "simple_nameless::__executable_wrapper__main" 
              4 steps |  1.99% |  82.59% |   4 steps |   1.99% | "u32_overflowing_add" 
              4 steps |  1.99% |  84.58% |   4 steps |   1.99% | "u8_overflowing_add" 
              3 steps |  1.49% |  86.07% |   3 steps |   1.49% | "array_new" 
              3 steps |  1.49% |  87.56% |   3 steps |   1.49% | "branch_align" 
              3 steps |  1.49% |  89.05% |  91 steps |  45.27% | "core::to_byte_array::append_formatted_to_byte_array" 
              3 steps |  1.49% |  90.55% |  32 steps |  15.92% | "core::to_byte_array::append_formatted_to_byte_array[1509-1662]" 
              3 steps |  1.49% |  92.04% |   3 steps |   1.49% | "downcast" 
              3 steps |  1.49% |  93.53% |   3 steps |   1.49% | "store_local" 
              2 steps |  1.00% |  94.53% |  45 steps |  22.39% | "core::byte_array::ByteArrayImpl::append_word" 
              2 steps |  1.00% |  95.52% |   2 steps |   1.00% | "jump" 
              2 steps |  1.00% |  96.52% |   2 steps |   1.00% | "u32_is_zero" 
              1 steps |  0.50% |  97.01% |   6 steps |   2.99% | "core::array::serialize_array_helper" 
              1 steps |  0.50% |  97.51% |  13 steps |   6.47% | "core::bytes_31::one_shift_left_bytes_u128_nz" 
              1 steps |  0.50% |  98.01% |  20 steps |   9.95% | "core::to_byte_array::append_formatted_to_byte_array[782-1099]" 
              1 steps |  0.50% |  98.51% |   1 steps |   0.50% | "enum_from_bounded_int" 
              1 steps |  0.50% |  99.00% |   1 steps |   0.50% | "finalize_locals" 
              1 steps |  0.50% |  99.50% |   1 steps |   0.50% | "print" 
              1 steps |  0.50% | 100.00% |   1 steps |   0.50% | "u8_is_zero" 
            "#
        ));
}

#[test]
fn view_execute_simple_standalone_mem_holes() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join(
                "crates/cairo-profiler/tests/executable_programs/simple_nameless/precompiled/",
            ),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("standalone_trace.json")
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
        .arg("memory holes")
        .assert()
        .success()
        .stdout_eq(indoc!(
            r#"
            
            Showing nodes accounting for 62 memory holes, 100.00% of 62 memory holes total
            Showing top 2 nodes out of 8
            
                        flat |   flat% |    sum% |             cum |    cum% |  
            -----------------+---------+---------+-----------------+---------+----------------------------------------------------------
             62 memory holes | 100.00% | 100.00% | 62 memory holes | 100.00% | "SCARB_EXECUTE/nTarget: standalone/nFunction: <unknown>" 
              0 memory holes |   0.00% | 100.00% |  0 memory holes |   0.00% | "core::array::serialize_array_helper" 
            "#
        ));
}

#[test]
fn view_execute_simple_standalone_range_check() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join(
                "crates/cairo-profiler/tests/executable_programs/simple_nameless/precompiled/",
            ),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("standalone_trace.json")
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
        .arg("range check builtin")
        .assert()
        .success()
        .stdout_eq(indoc!(
            r#"
            
            Showing nodes accounting for 11 range check builtin, 100.00% of 11 range check builtin total
            Showing top 2 nodes out of 8
            
                               flat |   flat% |    sum% |                    cum |    cum% |  
            ------------------------+---------+---------+------------------------+---------+----------------------------------------------------------
             11 range check builtin | 100.00% | 100.00% | 11 range check builtin | 100.00% | "SCARB_EXECUTE/nTarget: standalone/nFunction: <unknown>" 
              0 range check builtin |   0.00% | 100.00% |  0 range check builtin |   0.00% | "core::array::serialize_array_helper" 
            "#
        ));
}

#[test]
fn view_execute_simple_bootloader_steps() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join(
                "crates/cairo-profiler/tests/executable_programs/simple_nameless/precompiled/",
            ),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("bootloader_trace.json")
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
        .arg("steps")
        .assert()
        .success()
        .stdout_eq(indoc!(
            r#"
            
            Showing nodes accounting for 198 steps, 100.00% of 198 steps total
            Showing top 28 nodes out of 28
            
                 flat |  flat% |    sum% |       cum |    cum% |  
            ----------+--------+---------+-----------+---------+------------------------------------------------------------------
             94 steps | 47.47% |  47.47% |  94 steps |  47.47% | "store_temp" 
             11 steps |  5.56% |  53.03% |  11 steps |   5.56% | "u8_overflowing_sub" 
             10 steps |  5.05% |  58.08% | 198 steps | 100.00% | "SCARB_EXECUTE/nTarget: bootloader/nFunction: <unknown>" 
             10 steps |  5.05% |  63.13% |  10 steps |   5.05% | "u32_overflowing_sub" 
              7 steps |  3.54% |  66.67% |   7 steps |   3.54% | "enum_match" 
              7 steps |  3.54% |  70.20% |   7 steps |   3.54% | "u8_safe_divmod" 
              6 steps |  3.03% |  73.23% |   6 steps |   3.03% | "array_snapshot_pop_front" 
              5 steps |  2.53% |  75.76% |   5 steps |   2.53% | "array_append" 
              5 steps |  2.53% |  78.28% |   5 steps |   2.53% | "array_snapshot_pop_back" 
              4 steps |  2.02% |  80.30% | 188 steps |  94.95% | "simple_nameless::__executable_wrapper__main" 
              4 steps |  2.02% |  82.32% |   4 steps |   2.02% | "u32_overflowing_add" 
              4 steps |  2.02% |  84.34% |   4 steps |   2.02% | "u8_overflowing_add" 
              3 steps |  1.52% |  85.86% |   3 steps |   1.52% | "array_new" 
              3 steps |  1.52% |  87.37% |   3 steps |   1.52% | "branch_align" 
              3 steps |  1.52% |  88.89% |  91 steps |  45.96% | "core::to_byte_array::append_formatted_to_byte_array" 
              3 steps |  1.52% |  90.40% |  32 steps |  16.16% | "core::to_byte_array::append_formatted_to_byte_array[1509-1662]" 
              3 steps |  1.52% |  91.92% |   3 steps |   1.52% | "downcast" 
              3 steps |  1.52% |  93.43% |   3 steps |   1.52% | "store_local" 
              2 steps |  1.01% |  94.44% |  45 steps |  22.73% | "core::byte_array::ByteArrayImpl::append_word" 
              2 steps |  1.01% |  95.45% |   2 steps |   1.01% | "jump" 
              2 steps |  1.01% |  96.46% |   2 steps |   1.01% | "u32_is_zero" 
              1 steps |  0.51% |  96.97% |   6 steps |   3.03% | "core::array::serialize_array_helper" 
              1 steps |  0.51% |  97.47% |  13 steps |   6.57% | "core::bytes_31::one_shift_left_bytes_u128_nz" 
              1 steps |  0.51% |  97.98% |  20 steps |  10.10% | "core::to_byte_array::append_formatted_to_byte_array[782-1099]" 
              1 steps |  0.51% |  98.48% |   1 steps |   0.51% | "enum_from_bounded_int" 
              1 steps |  0.51% |  98.99% |   1 steps |   0.51% | "finalize_locals" 
              1 steps |  0.51% |  99.49% |   1 steps |   0.51% | "print" 
              1 steps |  0.51% | 100.00% |   1 steps |   0.51% | "u8_is_zero" 
            "#
        ));
}

#[test]
fn view_execute_simple_bootloader_mem_holes() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join(
                "crates/cairo-profiler/tests/executable_programs/simple_nameless/precompiled/",
            ),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("bootloader_trace.json")
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
        .arg("memory holes")
        .assert()
        .success()
        .stdout_eq(indoc!(
            r#"
            
            Showing nodes accounting for 61 memory holes, 100.00% of 61 memory holes total
            Showing top 2 nodes out of 8
            
                        flat |   flat% |    sum% |             cum |    cum% |  
            -----------------+---------+---------+-----------------+---------+----------------------------------------------------------
             61 memory holes | 100.00% | 100.00% | 61 memory holes | 100.00% | "SCARB_EXECUTE/nTarget: bootloader/nFunction: <unknown>" 
              0 memory holes |   0.00% | 100.00% |  0 memory holes |   0.00% | "core::array::serialize_array_helper" 
            "#
        ));
}

#[test]
fn view_execute_simple_bootloader_range_check() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join(
                "crates/cairo-profiler/tests/executable_programs/simple_nameless/precompiled/",
            ),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("bootloader_trace.json")
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
        .arg("range check builtin")
        .assert()
        .success()
        .stdout_eq(indoc!(
            r#"
            
            Showing nodes accounting for 11 range check builtin, 100.00% of 11 range check builtin total
            Showing top 2 nodes out of 8
            
                               flat |   flat% |    sum% |                    cum |    cum% |  
            ------------------------+---------+---------+------------------------+---------+----------------------------------------------------------
             11 range check builtin | 100.00% | 100.00% | 11 range check builtin | 100.00% | "SCARB_EXECUTE/nTarget: bootloader/nFunction: <unknown>" 
              0 range check builtin |   0.00% | 100.00% |  0 range check builtin |   0.00% | "core::array::serialize_array_helper" 
            "#
        ));
}

#[test]
fn view_execute_with_syscalls_bootloader_sierra_gas() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join(
                "crates/cairo-profiler/tests/executable_programs/multiple_targets/precompiled/",
            ),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("bootloader_with_syscalls_trace.json")
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
            
            Showing nodes accounting for 69700 sierra gas, 100.00% of 69700 sierra gas total
            Showing top 29 nodes out of 29
            
                         flat |  flat% |    sum% |              cum |    cum% |  
            ------------------+--------+---------+------------------+---------+--------------------------------------------------------------------------------
             20000 sierra gas | 28.69% |  28.69% | 20000 sierra gas |  28.69% | "Keccak" 
             18300 sierra gas | 26.26% |  54.95% | 18300 sierra gas |  26.26% | "store_temp" 
              5800 sierra gas |  8.32% |  63.27% |  5800 sierra gas |   8.32% | "u32_eq" 
              5200 sierra gas |  7.46% |  70.73% | 26800 sierra gas |  38.45% | "core::keccak::finalize_padding" 
              3400 sierra gas |  4.88% |  75.61% |  3400 sierra gas |   4.88% | "array_append" 
              2500 sierra gas |  3.59% |  79.20% |  2500 sierra gas |   3.59% | "bounded_int_trim_min" 
              2000 sierra gas |  2.87% |  82.07% | 65400 sierra gas |  93.83% | "multiple_targets::with_syscalls" 
              1900 sierra gas |  2.73% |  84.79% | 69700 sierra gas | 100.00% | "SCARB_EXECUTE/nTarget: bootloader/nFunction: multiple_targets::with_syscalls" 
              1400 sierra gas |  2.01% |  86.80% |  1400 sierra gas |   2.01% | "bounded_int_div_rem" 
              1400 sierra gas |  2.01% |  88.81% |  1400 sierra gas |   2.01% | "ec_point_from_x_nz" 
              1200 sierra gas |  1.72% |  90.53% |  1200 sierra gas |   1.72% | "ec_state_try_finalize_nz" 
               800 sierra gas |  1.15% |  91.68% |   800 sierra gas |   1.15% | "array_snapshot_pop_front" 
               800 sierra gas |  1.15% |  92.83% |   800 sierra gas |   1.15% | "store_local" 
               700 sierra gas |  1.00% |  93.83% |   700 sierra gas |   1.00% | "ec_state_init" 
               700 sierra gas |  1.00% |  94.84% |   700 sierra gas |   1.00% | "u32_safe_divmod" 
               500 sierra gas |  0.72% |  95.55% |   500 sierra gas |   0.72% | "ec_state_add_mul" 
               400 sierra gas |  0.57% |  96.13% |   400 sierra gas |   0.57% | "u64_overflowing_add" 
               300 sierra gas |  0.43% |  96.56% |  4000 sierra gas |   5.74% | "core::keccak::keccak_u256s_le_inputs[637-804]" 
               300 sierra gas |  0.43% |  96.99% |   300 sierra gas |   0.43% | "enum_match" 
               300 sierra gas |  0.43% |  97.42% |   300 sierra gas |   0.43% | "hades_permutation" 
               300 sierra gas |  0.43% |  97.85% | 67800 sierra gas |  97.27% | "multiple_targets::__executable_wrapper__with_syscalls" 
               300 sierra gas |  0.43% |  98.28% |   300 sierra gas |   0.43% | "u32_overflowing_sub" 
               200 sierra gas |  0.29% |  98.57% |   200 sierra gas |   0.29% | "array_new" 
               200 sierra gas |  0.29% |  98.85% |   200 sierra gas |   0.29% | "const_as_box" 
               200 sierra gas |  0.29% |  99.14% |   200 sierra gas |   0.29% | "jump" 
               200 sierra gas |  0.29% |  99.43% |   200 sierra gas |   0.29% | "pedersen" 
               200 sierra gas |  0.29% |  99.71% |   200 sierra gas |   0.29% | "u8_bitwise" 
               100 sierra gas |  0.14% |  99.86% |   100 sierra gas |   0.14% | "ec_point_is_zero" 
               100 sierra gas |  0.14% | 100.00% |   100 sierra gas |   0.14% | "finalize_locals" 
            "#
        ));
}

#[test]
fn view_execute_with_syscalls_bootloader_syscall_usage() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join(
                "crates/cairo-profiler/tests/executable_programs/multiple_targets/precompiled/",
            ),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("bootloader_with_syscalls_trace.json")
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
        .arg("syscall usage")
        .assert()
        .success()
        .stdout_eq(indoc!(
            r#"
            
            Showing nodes accounting for 2 syscall usage, 100.00% of 2 syscall usage total
            Showing top 6 nodes out of 6
            
                        flat |   flat% |    sum% |             cum |    cum% |  
            -----------------+---------+---------+-----------------+---------+--------------------------------------------------------------------------------
             2 syscall usage | 100.00% | 100.00% | 2 syscall usage | 100.00% | "Keccak" 
             0 syscall usage |   0.00% | 100.00% | 2 syscall usage | 100.00% | "SCARB_EXECUTE/nTarget: bootloader/nFunction: multiple_targets::with_syscalls" 
             0 syscall usage |   0.00% | 100.00% | 0 syscall usage |   0.00% | "core::keccak::finalize_padding" 
             0 syscall usage |   0.00% | 100.00% | 0 syscall usage |   0.00% | "core::keccak::keccak_u256s_le_inputs[637-804]" 
             0 syscall usage |   0.00% | 100.00% | 2 syscall usage | 100.00% | "multiple_targets::__executable_wrapper__with_syscalls" 
             0 syscall usage |   0.00% | 100.00% | 2 syscall usage | 100.00% | "multiple_targets::with_syscalls" 
            "#
        ));
}

#[test]
fn view_execute_with_syscalls_standalone_sierra_gas() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join(
                "crates/cairo-profiler/tests/executable_programs/multiple_targets/precompiled/",
            ),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("standalone_with_syscalls_trace.json")
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
            
            Showing nodes accounting for 70000 sierra gas, 100.00% of 70000 sierra gas total
            Showing top 29 nodes out of 29
            
                         flat |  flat% |    sum% |              cum |    cum% |  
            ------------------+--------+---------+------------------+---------+--------------------------------------------------------------------------------
             20000 sierra gas | 28.57% |  28.57% | 20000 sierra gas |  28.57% | "Keccak" 
             18300 sierra gas | 26.14% |  54.71% | 18300 sierra gas |  26.14% | "store_temp" 
              5800 sierra gas |  8.29% |  63.00% |  5800 sierra gas |   8.29% | "u32_eq" 
              5200 sierra gas |  7.43% |  70.43% | 26800 sierra gas |  38.29% | "core::keccak::finalize_padding" 
              3400 sierra gas |  4.86% |  75.29% |  3400 sierra gas |   4.86% | "array_append" 
              2500 sierra gas |  3.57% |  78.86% |  2500 sierra gas |   3.57% | "bounded_int_trim_min" 
              2200 sierra gas |  3.14% |  82.00% | 70000 sierra gas | 100.00% | "SCARB_EXECUTE/nTarget: standalone/nFunction: multiple_targets::with_syscalls" 
              2000 sierra gas |  2.86% |  84.86% | 65400 sierra gas |  93.43% | "multiple_targets::with_syscalls" 
              1400 sierra gas |  2.00% |  86.86% |  1400 sierra gas |   2.00% | "bounded_int_div_rem" 
              1400 sierra gas |  2.00% |  88.86% |  1400 sierra gas |   2.00% | "ec_point_from_x_nz" 
              1200 sierra gas |  1.71% |  90.57% |  1200 sierra gas |   1.71% | "ec_state_try_finalize_nz" 
               800 sierra gas |  1.14% |  91.71% |   800 sierra gas |   1.14% | "array_snapshot_pop_front" 
               800 sierra gas |  1.14% |  92.86% |   800 sierra gas |   1.14% | "store_local" 
               700 sierra gas |  1.00% |  93.86% |   700 sierra gas |   1.00% | "ec_state_init" 
               700 sierra gas |  1.00% |  94.86% |   700 sierra gas |   1.00% | "u32_safe_divmod" 
               500 sierra gas |  0.71% |  95.57% |   500 sierra gas |   0.71% | "ec_state_add_mul" 
               400 sierra gas |  0.57% |  96.14% |   400 sierra gas |   0.57% | "u64_overflowing_add" 
               300 sierra gas |  0.43% |  96.57% |  4000 sierra gas |   5.71% | "core::keccak::keccak_u256s_le_inputs[637-804]" 
               300 sierra gas |  0.43% |  97.00% |   300 sierra gas |   0.43% | "enum_match" 
               300 sierra gas |  0.43% |  97.43% |   300 sierra gas |   0.43% | "hades_permutation" 
               300 sierra gas |  0.43% |  97.86% | 67800 sierra gas |  96.86% | "multiple_targets::__executable_wrapper__with_syscalls" 
               300 sierra gas |  0.43% |  98.29% |   300 sierra gas |   0.43% | "u32_overflowing_sub" 
               200 sierra gas |  0.29% |  98.57% |   200 sierra gas |   0.29% | "array_new" 
               200 sierra gas |  0.29% |  98.86% |   200 sierra gas |   0.29% | "const_as_box" 
               200 sierra gas |  0.29% |  99.14% |   200 sierra gas |   0.29% | "jump" 
               200 sierra gas |  0.29% |  99.43% |   200 sierra gas |   0.29% | "pedersen" 
               200 sierra gas |  0.29% |  99.71% |   200 sierra gas |   0.29% | "u8_bitwise" 
               100 sierra gas |  0.14% |  99.86% |   100 sierra gas |   0.14% | "ec_point_is_zero" 
               100 sierra gas |  0.14% | 100.00% |   100 sierra gas |   0.14% | "finalize_locals" 
            "#
        ));
}

#[test]
fn view_execute_with_syscalls_standalone_syscall_usage() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join(
                "crates/cairo-profiler/tests/executable_programs/multiple_targets/precompiled/",
            ),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("standalone_with_syscalls_trace.json")
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
        .arg("syscall usage")
        .assert()
        .success()
        .stdout_eq(indoc!(
            r#"
            
            Showing nodes accounting for 2 syscall usage, 100.00% of 2 syscall usage total
            Showing top 6 nodes out of 6
            
                        flat |   flat% |    sum% |             cum |    cum% |  
            -----------------+---------+---------+-----------------+---------+--------------------------------------------------------------------------------
             2 syscall usage | 100.00% | 100.00% | 2 syscall usage | 100.00% | "Keccak" 
             0 syscall usage |   0.00% | 100.00% | 2 syscall usage | 100.00% | "SCARB_EXECUTE/nTarget: standalone/nFunction: multiple_targets::with_syscalls" 
             0 syscall usage |   0.00% | 100.00% | 0 syscall usage |   0.00% | "core::keccak::finalize_padding" 
             0 syscall usage |   0.00% | 100.00% | 0 syscall usage |   0.00% | "core::keccak::keccak_u256s_le_inputs[637-804]" 
             0 syscall usage |   0.00% | 100.00% | 2 syscall usage | 100.00% | "multiple_targets::__executable_wrapper__with_syscalls" 
             0 syscall usage |   0.00% | 100.00% | 2 syscall usage | 100.00% | "multiple_targets::with_syscalls" 
            "#
        ));
}

#[test]
fn view_execute_with_arguments_bootloader_sierra_gas() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join(
                "crates/cairo-profiler/tests/executable_programs/multiple_targets/precompiled/",
            ),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("bootloader_with_arguments_trace.json")
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
            
            Showing nodes accounting for 30300 sierra gas, 100.00% of 30300 sierra gas total
            Showing top 33 nodes out of 33
            
                         flat |  flat% |    sum% |              cum |    cum% |  
            ------------------+--------+---------+------------------+---------+---------------------------------------------------------------------------------
             12700 sierra gas | 41.91% |  41.91% | 12700 sierra gas |  41.91% | "store_temp" 
              2500 sierra gas |  8.25% |  50.17% |  2500 sierra gas |   8.25% | "u256_safe_divmod" 
              2300 sierra gas |  7.59% |  57.76% |  2300 sierra gas |   7.59% | "u128_mul_guarantee_verify" 
              1100 sierra gas |  3.63% |  61.39% |  1100 sierra gas |   3.63% | "u8_overflowing_sub" 
              1000 sierra gas |  3.30% |  64.69% | 30300 sierra gas | 100.00% | "SCARB_EXECUTE/nTarget: bootloader/nFunction: multiple_targets::with_arguments" 
              1000 sierra gas |  3.30% |  67.99% |  1000 sierra gas |   3.30% | "array_snapshot_pop_front" 
              1000 sierra gas |  3.30% |  71.29% |  1000 sierra gas |   3.30% | "u32_overflowing_sub" 
               900 sierra gas |  2.97% |  74.26% |   900 sierra gas |   2.97% | "downcast" 
               900 sierra gas |  2.97% |  77.23% |   900 sierra gas |   2.97% | "enum_match" 
               600 sierra gas |  1.98% |  79.21% |   600 sierra gas |   1.98% | "array_append" 
               500 sierra gas |  1.65% |  80.86% |   500 sierra gas |   1.65% | "array_snapshot_pop_back" 
               400 sierra gas |  1.32% |  82.18% |   400 sierra gas |   1.32% | "branch_align" 
               400 sierra gas |  1.32% |  83.50% |   400 sierra gas |   1.32% | "jump" 
               400 sierra gas |  1.32% |  84.82% | 29300 sierra gas |  96.70% | "multiple_targets::__executable_wrapper__with_arguments" 
               400 sierra gas |  1.32% |  86.14% |   400 sierra gas |   1.32% | "store_local" 
               400 sierra gas |  1.32% |  87.46% |   400 sierra gas |   1.32% | "u128_is_zero" 
               400 sierra gas |  1.32% |  88.78% |   400 sierra gas |   1.32% | "u128s_from_felt252" 
               400 sierra gas |  1.32% |  90.10% |   400 sierra gas |   1.32% | "u32_overflowing_add" 
               400 sierra gas |  1.32% |  91.42% |   400 sierra gas |   1.32% | "u8_overflowing_add" 
               400 sierra gas |  1.32% |  92.74% |   400 sierra gas |   1.32% | "u8_try_from_felt252" 
               300 sierra gas |  0.99% |  93.73% |   300 sierra gas |   0.99% | "array_new" 
               300 sierra gas |  0.99% |  94.72% | 14400 sierra gas |  47.52% | "core::to_byte_array::append_formatted_to_byte_array" 
               300 sierra gas |  0.99% |  95.71% |  3200 sierra gas |  10.56% | "core::to_byte_array::append_formatted_to_byte_array[1509-1662]" 
               200 sierra gas |  0.66% |  96.37% |  4500 sierra gas |  14.85% | "core::byte_array::ByteArrayImpl::append_word" 
               200 sierra gas |  0.66% |  97.03% | 18000 sierra gas |  59.41% | "core::fmt::DisplayInteger::fmt" 
               200 sierra gas |  0.66% |  97.69% |   200 sierra gas |   0.66% | "u32_is_zero" 
               100 sierra gas |  0.33% |  98.02% |   600 sierra gas |   1.98% | "core::array::serialize_array_helper" 
               100 sierra gas |  0.33% |  98.35% |  1300 sierra gas |   4.29% | "core::bytes_31::one_shift_left_bytes_u128_nz" 
               100 sierra gas |  0.33% |  98.68% |  6700 sierra gas |  22.11% | "core::to_byte_array::append_formatted_to_byte_array[782-1099]" 
               100 sierra gas |  0.33% |  99.01% |   100 sierra gas |   0.33% | "enum_from_bounded_int" 
               100 sierra gas |  0.33% |  99.34% |   100 sierra gas |   0.33% | "finalize_locals" 
               100 sierra gas |  0.33% |  99.67% |   100 sierra gas |   0.33% | "print" 
               100 sierra gas |  0.33% | 100.00% |   100 sierra gas |   0.33% | "u256_is_zero" 
            "#
        ));
}

#[test]
fn view_execute_with_arguments_standalone_sierra_gas() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join(
                "crates/cairo-profiler/tests/executable_programs/multiple_targets/precompiled/",
            ),
            &["*.json"],
        )
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("build-profile")
        .arg("standalone_with_arguments_trace.json")
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
            
            Showing nodes accounting for 30600 sierra gas, 100.00% of 30600 sierra gas total
            Showing top 33 nodes out of 33
            
                         flat |  flat% |    sum% |              cum |    cum% |  
            ------------------+--------+---------+------------------+---------+---------------------------------------------------------------------------------
             12700 sierra gas | 41.50% |  41.50% | 12700 sierra gas |  41.50% | "store_temp" 
              2500 sierra gas |  8.17% |  49.67% |  2500 sierra gas |   8.17% | "u256_safe_divmod" 
              2300 sierra gas |  7.52% |  57.19% |  2300 sierra gas |   7.52% | "u128_mul_guarantee_verify" 
              1300 sierra gas |  4.25% |  61.44% | 30600 sierra gas | 100.00% | "SCARB_EXECUTE/nTarget: standalone/nFunction: multiple_targets::with_arguments" 
              1100 sierra gas |  3.59% |  65.03% |  1100 sierra gas |   3.59% | "u8_overflowing_sub" 
              1000 sierra gas |  3.27% |  68.30% |  1000 sierra gas |   3.27% | "array_snapshot_pop_front" 
              1000 sierra gas |  3.27% |  71.57% |  1000 sierra gas |   3.27% | "u32_overflowing_sub" 
               900 sierra gas |  2.94% |  74.51% |   900 sierra gas |   2.94% | "downcast" 
               900 sierra gas |  2.94% |  77.45% |   900 sierra gas |   2.94% | "enum_match" 
               600 sierra gas |  1.96% |  79.41% |   600 sierra gas |   1.96% | "array_append" 
               500 sierra gas |  1.63% |  81.05% |   500 sierra gas |   1.63% | "array_snapshot_pop_back" 
               400 sierra gas |  1.31% |  82.35% |   400 sierra gas |   1.31% | "branch_align" 
               400 sierra gas |  1.31% |  83.66% |   400 sierra gas |   1.31% | "jump" 
               400 sierra gas |  1.31% |  84.97% | 29300 sierra gas |  95.75% | "multiple_targets::__executable_wrapper__with_arguments" 
               400 sierra gas |  1.31% |  86.27% |   400 sierra gas |   1.31% | "store_local" 
               400 sierra gas |  1.31% |  87.58% |   400 sierra gas |   1.31% | "u128_is_zero" 
               400 sierra gas |  1.31% |  88.89% |   400 sierra gas |   1.31% | "u128s_from_felt252" 
               400 sierra gas |  1.31% |  90.20% |   400 sierra gas |   1.31% | "u32_overflowing_add" 
               400 sierra gas |  1.31% |  91.50% |   400 sierra gas |   1.31% | "u8_overflowing_add" 
               400 sierra gas |  1.31% |  92.81% |   400 sierra gas |   1.31% | "u8_try_from_felt252" 
               300 sierra gas |  0.98% |  93.79% |   300 sierra gas |   0.98% | "array_new" 
               300 sierra gas |  0.98% |  94.77% | 14400 sierra gas |  47.06% | "core::to_byte_array::append_formatted_to_byte_array" 
               300 sierra gas |  0.98% |  95.75% |  3200 sierra gas |  10.46% | "core::to_byte_array::append_formatted_to_byte_array[1509-1662]" 
               200 sierra gas |  0.65% |  96.41% |  4500 sierra gas |  14.71% | "core::byte_array::ByteArrayImpl::append_word" 
               200 sierra gas |  0.65% |  97.06% | 18000 sierra gas |  58.82% | "core::fmt::DisplayInteger::fmt" 
               200 sierra gas |  0.65% |  97.71% |   200 sierra gas |   0.65% | "u32_is_zero" 
               100 sierra gas |  0.33% |  98.04% |   600 sierra gas |   1.96% | "core::array::serialize_array_helper" 
               100 sierra gas |  0.33% |  98.37% |  1300 sierra gas |   4.25% | "core::bytes_31::one_shift_left_bytes_u128_nz" 
               100 sierra gas |  0.33% |  98.69% |  6700 sierra gas |  21.90% | "core::to_byte_array::append_formatted_to_byte_array[782-1099]" 
               100 sierra gas |  0.33% |  99.02% |   100 sierra gas |   0.33% | "enum_from_bounded_int" 
               100 sierra gas |  0.33% |  99.35% |   100 sierra gas |   0.33% | "finalize_locals" 
               100 sierra gas |  0.33% |  99.67% |   100 sierra gas |   0.33% | "print" 
               100 sierra gas |  0.33% | 100.00% |   100 sierra gas |   0.33% | "u256_is_zero" 
            "#
        ));
}
