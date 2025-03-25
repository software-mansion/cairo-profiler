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
            "invalid_versioned_constants.json",
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
fn view_samples() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join("crates/cairo-profiler/tests/contracts/balance_simple/precompiled/"),
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
            project_root.join("crates/cairo-profiler/tests/contracts/balance_simple/precompiled/"),
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
            
            Showing nodes accounting for 1410 steps, 100.00% of 1410 steps total
            Showing top 14 nodes out of 14
            
                  flat |  flat% |    sum% |        cum |    cum% |  
            -----------+--------+---------+------------+---------+-----------------------------------------------------------------------------------------------
             866 steps | 61.42% |  61.42% |  866 steps |  61.42% | "CallContract" 
              91 steps |  6.45% |  67.87% |  168 steps |  11.91% | "core::result::ResultSerde::::deserialize" 
              87 steps |  6.17% |  74.04% |   87 steps |   6.17% | "StorageRead" 
              75 steps |  5.32% |  79.36% |   75 steps |   5.32% | "snforge_std::_cheatcode::handle_cheatcode" 
              53 steps |  3.76% |  83.12% | 1285 steps |  91.13% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value" 
              39 steps |  2.77% |  85.89% |   39 steps |   2.77% | "core::array::SpanFelt252Serde::deserialize" 
              38 steps |  2.70% |  88.58% |   38 steps |   2.70% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
              36 steps |  2.55% |  91.13% |  144 steps |  10.21% | "snforge_std::cheatcodes::contract_class::declare" 
              34 steps |  2.41% |  93.55% |  170 steps |  12.06% | "snforge_std::cheatcodes::contract_class::ContractClassImpl::deploy" 
              33 steps |  2.34% |  95.89% |  120 steps |   8.51% | "balance_simple::HelloStarknet::__wrapper__HelloStarknetImpl__get_balance" 
              27 steps |  1.91% |  97.80% |  341 steps |  24.18% | "balance_simple_integrationtest::test_contract::deploy_contract" 
              26 steps |  1.84% |  99.65% |   26 steps |   1.84% | "core::array::serialize_array_helper::" 
               5 steps |  0.35% | 100.00% | 1410 steps | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
               0 steps |  0.00% | 100.00% |  120 steps |   8.51% | "Contract: HelloStarknet\nFunction: get_balance\n" 
            "#
        ));
}

#[test]
fn view_range_check_builtin() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(
            project_root.join("crates/cairo-profiler/tests/contracts/balance_simple/precompiled/"),
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
            Showing top 3 nodes out of 14
            
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
            project_root.join("crates/cairo-profiler/tests/contracts/balance_simple/precompiled/"),
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
            project_root.join("crates/cairo-profiler/tests/contracts/balance_simple/precompiled/"),
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

            Showing nodes accounting for 1410 steps, 100.00% of 1410 steps total
            Showing top 11 nodes out of 11

                  flat |  flat% |    sum% |        cum |    cum% |  
            -----------+--------+---------+------------+---------+-----------------------------------------------------------------------------------------------
             866 steps | 61.42% |  61.42% |  866 steps |  61.42% | "CallContract" 
             145 steps | 10.28% |  71.70% |  170 steps |  12.06% | "snforge_std::cheatcodes::contract_class::ContractClassImpl::deploy" 
              87 steps |  6.17% |  77.87% |   87 steps |   6.17% | "StorageRead" 
              81 steps |  5.74% |  83.62% |  144 steps |  10.21% | "snforge_std::cheatcodes::contract_class::declare" 
              75 steps |  5.32% |  88.94% |   75 steps |   5.32% | "snforge_std::_cheatcode::handle_cheatcode" 
              53 steps |  3.76% |  92.70% | 1285 steps |  91.13% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value" 
              38 steps |  2.70% |  95.39% |   38 steps |   2.70% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
              33 steps |  2.34% |  97.73% |  120 steps |   8.51% | "balance_simple::HelloStarknet::__wrapper__HelloStarknetImpl__get_balance" 
              27 steps |  1.91% |  99.65% |  341 steps |  24.18% | "balance_simple_integrationtest::test_contract::deploy_contract" 
               5 steps |  0.35% | 100.00% | 1410 steps | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
               0 steps |  0.00% | 100.00% |  120 steps |   8.51% | "Contract: HelloStarknet\nFunction: get_balance\n" 
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
            project_root.join("crates/cairo-profiler/tests/contracts/balance_simple/precompiled/"),
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

            Showing nodes accounting for 1410 steps, 100.00% of 1410 steps total
            Showing top 11 nodes out of 11

                  flat |  flat% |    sum% |        cum |    cum% |  
            -----------+--------+---------+------------+---------+-----------------------------------------------------------------------------------------------
             866 steps | 61.42% |  61.42% |  866 steps |  61.42% | "CallContract" 
             145 steps | 10.28% |  71.70% |  170 steps |  12.06% | "snforge_std::cheatcodes::contract_class::ContractClassImpl::deploy" 
              87 steps |  6.17% |  77.87% |   87 steps |   6.17% | "StorageRead" 
              81 steps |  5.74% |  83.62% |  144 steps |  10.21% | "snforge_std::cheatcodes::contract_class::declare" 
              75 steps |  5.32% |  88.94% |   75 steps |   5.32% | "snforge_std::_cheatcode::handle_cheatcode" 
              53 steps |  3.76% |  92.70% | 1285 steps |  91.13% | "balance_simple_integrationtest::test_contract::test_cannot_increase_balance_with_zero_value" 
              38 steps |  2.70% |  95.39% |   38 steps |   2.70% | "snforge_std::cheatcodes::contract_class::DeclareResultSerde::deserialize" 
              33 steps |  2.34% |  97.73% |  120 steps |   8.51% | "balance_simple::HelloStarknet::__wrapper__HelloStarknetImpl__get_balance" 
              27 steps |  1.91% |  99.65% |  341 steps |  24.18% | "balance_simple_integrationtest::test_contract::deploy_contract" 
               5 steps |  0.35% | 100.00% | 1410 steps | 100.00% | "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n" 
               0 steps |  0.00% | 100.00% |  120 steps |   8.51% | "Contract: HelloStarknet\nFunction: get_balance\n" 
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
