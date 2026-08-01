//! Missing input → exit 3.

use tempfile::TempDir;

use super::{bin, with_isolation};

#[test]
fn missing_input_exit_3() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    let mut cmd = bin();
    with_isolation(&mut cmd, &cfg);
    cmd.args(["run", "-q"]).assert().failure().code(3);
}
