//! `vd-srv serve`.

use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::api;
use crate::cli::{CliError, ServeArgs};
use crate::config;
use crate::engine::{Engine, TransportEndpoint, TransportStatus};
use crate::paths;

pub fn run(args: ServeArgs) -> Result<(), CliError> {
    let mut cfg = config::load(&paths::config_path()).map_err(CliError::usage)?;
    if let Some(w) = args.workers {
        cfg.raw.workers = w;
    }
    let data = config::effective_data_dir(&cfg.raw, args.data_dir.as_deref());
    fs::create_dir_all(&data).map_err(|e| CliError::with_code(1, e.to_string()))?;

    let primary = config::effective_endpoint(
        &cfg.raw,
        &data,
        args.transport,
        args.socket.as_deref(),
        args.tcp.as_deref(),
    )
    .map_err(CliError::usage)?;

    // Optional secondary TCP listener when primary is IPC and `tcp` is configured.
    let secondary = if matches!(primary, api::Endpoint::Tcp(_)) {
        None
    } else {
        let tcp = args.tcp.as_deref().or(cfg.raw.tcp.as_deref());
        tcp.and_then(|addr| {
            api::resolve_endpoint(api::TransportKind::Tcp, None, None, Some(addr), &data).ok()
        })
    };

    let http_bind = cfg.raw.http.listen_addr(args.http.as_deref());
    let grpc_bind = cfg.raw.grpc.listen_addr(args.grpc.as_deref());

    let engine = Engine::start(data.clone(), cfg.raw.clone())
        .map_err(|e| CliError::with_code(1, e.to_string()))?;

    let mut transports = TransportStatus::default();
    match &primary {
        api::Endpoint::Uds(p) => {
            transports.uds = Some(TransportEndpoint::on(format!("unix://{}", p.display())));
        }
        api::Endpoint::Tcp(a) => {
            transports.tcp = Some(TransportEndpoint::on(format!("tcp://{a}")));
        }
        api::Endpoint::Pipe(p) => {
            transports.uds = Some(TransportEndpoint::on(format!("pipe://{p}")));
        }
    }
    if let Some(ref sec) = secondary {
        if let api::Endpoint::Tcp(a) = sec {
            transports.tcp = Some(TransportEndpoint::on(format!("tcp://{a}")));
        }
    }
    transports.http = Some(match &http_bind {
        Some(b) => TransportEndpoint::on(format!("http://{b}")),
        None => TransportEndpoint::off(),
    });
    transports.grpc = Some(match &grpc_bind {
        Some(b) => TransportEndpoint::on(format!("grpc://{b}")),
        None => TransportEndpoint::off(),
    });
    engine.set_transports(transports);

    fs::write(paths::pid_path(&data), std::process::id().to_string())
        .map_err(|e| CliError::with_code(1, e.to_string()))?;

    eprintln!(
        "vd-srv listening on {} (data {}) workers={}",
        primary.display(),
        data.display(),
        cfg.raw.workers
    );
    if let Some(ref sec) = secondary {
        eprintln!("vd-srv also listening on {}", sec.display());
    }
    if let Some(ref http) = http_bind {
        eprintln!("vd-srv HTTP transport on http://{http}");
    }
    if let Some(ref grpc) = grpc_bind {
        eprintln!("vd-srv gRPC transport on grpc://{grpc}");
    }

    let stop_serve = Arc::new(AtomicBool::new(false));
    let mut handles = Vec::new();

    {
        let stop_flag = Arc::clone(&stop_serve);
        let eng = engine.clone();
        let ep = primary.clone();
        handles.push(thread::spawn(move || {
            let r = api::serve(&ep, eng.clone(), Arc::clone(&stop_flag));
            eng.stop();
            r
        }));
    }
    if let Some(sec) = secondary {
        let stop_flag = Arc::clone(&stop_serve);
        let eng = engine.clone();
        handles.push(thread::spawn(move || api::serve(&sec, eng, stop_flag)));
    }
    if let Some(bind) = http_bind {
        let stop_flag = Arc::clone(&stop_serve);
        let eng = engine.clone();
        handles.push(thread::spawn(move || api::http::serve(&bind, eng, stop_flag)));
    }
    if let Some(bind) = grpc_bind {
        let stop_flag = Arc::clone(&stop_serve);
        let eng = engine.clone();
        handles.push(thread::spawn(move || api::grpc::serve(&bind, eng, stop_flag)));
    }

    while !engine.is_stopped() && !stop_serve.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(200));
    }
    stop_serve.store(true, Ordering::SeqCst);
    engine.stop();
    for h in handles {
        let _ = h.join();
    }
    let _ = fs::remove_file(paths::pid_path(&data));
    let _ = args.foreground;
    Ok(())
}
