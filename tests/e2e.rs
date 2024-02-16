use assert_fs::fixture::PathCopy;
use snapbox::cmd::{cargo_bin, Command as SnapboxCommand};

#[test]
fn simple_package() {
    let project_root = project_root::get_project_root().unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    temp_dir
        .copy_from(project_root.join("tests/data/"), &["trace.json"])
        .unwrap();

    SnapboxCommand::new(cargo_bin!("cairo-profiler"))
        .current_dir(&temp_dir)
        .arg("./trace.json")
        .assert()
        .success();

    assert!(temp_dir.join("profile.pb.gz").exists());

    // TODO run pprof here
}
