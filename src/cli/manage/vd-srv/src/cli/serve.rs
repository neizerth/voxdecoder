//! `vd-srv serve`.

use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::api;
use crate::cli::{CliError, ServeArgs};
use crate::config;
use crate::engine::Engine;
use crate::paths;

pub fn run(args: ServeArgs) -> Result<(), CliError> {
    let mut cfg = config::load(&paths::config_path()).map_err(CliError::usage)?;
    if let Some(w) = args.workers {
        cfg.raw.workers = w;
    }
    let data = config::effective_data_dir(&cfg.raw, args.data_dir.as_deref());
    fs::create_dir_all(&data).map_err(|e| CliError::with_code(1, e.to_string()))?;
    let socket = args
        .socket
        .clone()
        .unwrap_or_else(|| config::effective_socket(&cfg.raw, &data));

    let engine = Engine::start(data.clone(), cfg.raw.clone())
        .map_err(|e| CliError::with_code(1, e.to_string()))?;

    fs::write(paths::pid_path(&data), std::process::id().to_string())
        .map_err(|e| CliError::with_code(1, e.to_string()))?;

    ctrlc_stub();

    eprintln!(
        "vd-srv listening on {} (data {}) workers={}",
        socket.display(),
        data.display(),
        cfg.raw.workers
    );

    let stop_serve = Arc::new(AtomicBool::new(false));
    let stop_flag = Arc::clone(&stop_serve);
    let sock = socket.clone();
    let eng2 = engine.clone();
    let handle = thread::spawn(move || {
        let r = api::serve_socket(&sock, eng2.clone(), Arc::clone(&stop_flag));
        eng2.stop();
        r
    });

    while !engine.is_stopped() && !stop_serve.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(200));
    }
    stop_serve.store(true, Ordering::SeqCst);
    engine.stop();
    let _ = handle.join();
    let _ = fs::remove_file(paths::pid_path(&data));
    let _ = args.foreground;
    Ok(())
}

fn ctrlc_stub() {
    // Rely on `vd-srv stop` / socket Stop; optional ctrlc crate later.
}
