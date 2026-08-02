use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_and_version() {
    Command::cargo_bin("vd-url")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("import-url"));

    Command::cargo_bin("vd-url")
        .unwrap()
        .arg("--version")
        .assert()
        .success();
}

#[test]
#[ignore = "requires network + yt-dlp; set VD_URL_E2E=1 and run with --ignored"]
fn youtube_inspect_live() {
    if std::env::var_os("VD_URL_E2E").is_none() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    Command::cargo_bin("vd-url")
        .unwrap()
        .args([
            "inspect",
            "-i",
            "https://www.youtube.com/watch?v=jNQXAC9IVRw",
            "--output-dir",
            dir.path().to_str().unwrap(),
            "--overwrite",
            "-o",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("metadata"));
}
