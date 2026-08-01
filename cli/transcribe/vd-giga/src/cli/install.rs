//! `vd-giga install`.

use crate::cli::{CliError, InstallArgs};
use crate::gigaam::catalog::CATALOG;
use crate::gigaam::weights::{self, InstallOutcome};
use crate::paths;
use crate::progress::{Progress, ProgressEvent, ProgressMode};

pub fn execute(args: InstallArgs) -> Result<(), CliError> {
    let root = paths::resolve_models_dir(args.download_root.clone());
    let mode = if args.quiet {
        ProgressMode::None
    } else {
        args.progress
            .map(ProgressMode::from)
            .unwrap_or(ProgressMode::Text)
    };
    let progress = Progress::new(mode);

    let models: Vec<&str> = if args.all {
        CATALOG.to_vec()
    } else {
        vec![args.model.as_deref().unwrap()]
    };

    for model in models {
        progress.emit(&ProgressEvent::Start {
            input: None,
            output: None,
            model: Some(model),
            device: None,
            path: Some(root.to_str().unwrap_or("")),
        });

        let mut last_pct = 0u8;
        let mut on_progress = |done: u64, total: Option<u64>| {
            let pct = match total {
                Some(t) if t > 0 => ((done * 100) / t) as u8,
                _ => 0,
            };
            if pct != last_pct {
                last_pct = pct;
                progress.emit(&ProgressEvent::Phase {
                    phase: "downloading",
                    percent: pct,
                    segment: None,
                    segment_total: None,
                    bytes_done: Some(done),
                    bytes_total: total,
                });
            }
        };

        match weights::install(&root, model, args.force, Some(&mut on_progress)) {
            Ok(InstallOutcome::AlreadyPresent(path)) => {
                if !args.quiet {
                    let _ = writeln_already(&path);
                }
                progress.emit(&ProgressEvent::Done {
                    output: None,
                    model: Some(model),
                    path: Some(path.to_str().unwrap_or("")),
                    duration_sec: None,
                    char_count: None,
                });
            }
            Ok(InstallOutcome::Installed(path)) => {
                progress.emit(&ProgressEvent::Done {
                    output: None,
                    model: Some(model),
                    path: Some(path.to_str().unwrap_or("")),
                    duration_sec: None,
                    char_count: None,
                });
            }
            Err(e) => {
                progress.emit(&ProgressEvent::Error {
                    code: "download_failed",
                    message: &e.to_string(),
                });
                return Err(CliError::with_code(4, e.to_string()));
            }
        }
    }
    Ok(())
}

fn writeln_already(path: &std::path::Path) -> std::io::Result<()> {
    use std::io::{self, Write};
    let mut err = io::stderr();
    writeln!(err, "already installed: {}", path.display())
}
