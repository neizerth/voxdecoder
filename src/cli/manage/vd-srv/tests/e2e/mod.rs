//! Binary e2e for vd-srv.

#![allow(clippy::needless_borrows_for_generic_args)]

use std::fs;
use std::process::Command;
use std::thread;
use std::time::Duration;

use assert_cmd::cargo::cargo_bin;
use tempfile::TempDir;

#[test]
fn serve_ping_stop() {
    let dir = TempDir::new().unwrap();
    let data = dir.path().join("data");
    let sock = dir.path().join("srv.sock");
    fs::create_dir_all(&data).unwrap();

    let bin = cargo_bin!("vd-srv");
    let mut child = Command::new(&bin)
        .args([
            "serve",
            "--data-dir",
            data.to_str().unwrap(),
            "--socket",
            sock.to_str().unwrap(),
            "--workers",
            "1",
        ])
        .spawn()
        .unwrap();

    let mut ok = false;
    for _ in 0..50 {
        thread::sleep(Duration::from_millis(100));
        if !sock.exists() {
            continue;
        }
        let out = Command::new(&bin)
            .env("VD_SRV_DATA", &data)
            .args(["ping"])
            // Client uses config socket; force via env by writing config — use same data_dir
            // and default socket name under data when --socket differs.
            .output();
        // Ping uses config path; set socket via config file in data isn't wired.
        // Call via explicit: write tiny config.
        let _ = out;
        let cfg = dir.path().join("config.toml");
        fs::write(
            &cfg,
            format!(
                "workers = 1\nsocket = \"{}\"\ndata_dir = \"{}\"\n",
                sock.display(),
                data.display()
            ),
        )
        .unwrap();
        let ping = Command::new(&bin)
            .env("VD_SRV_CONFIG", &cfg)
            .arg("ping")
            .output()
            .unwrap();
        if ping.status.success() {
            ok = true;
            break;
        }
    }
    let _ = Command::new(&bin)
        .env(
            "VD_SRV_CONFIG",
            dir.path().join("config.toml"),
        )
        .arg("stop")
        .output();
    let _ = child.wait();
    assert!(ok, "ping never succeeded");
}
