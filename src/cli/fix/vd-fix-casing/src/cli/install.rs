//! `vd-fix-casing install`.

use crate::cli::{CliError, InstallArgs};
use crate::models::{self, InstallOutcome};
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

    let models: Vec<String> = if args.all {
        models::shipping_names()
            .into_iter()
            .map(str::to_string)
            .collect()
    } else {
        vec![args.model.as_deref().unwrap().to_string()]
    };

    for model in models {
        let path_s = root.display().to_string();
        progress.emit(&ProgressEvent::Start {
            input: None,
            output: None,
            artifact_type: None,
            language: None,
            model: Some(&model),
            device: None,
            path: Some(&path_s),
        });

        let mut last_pct = 0u8;
        let mut on_progress = |done: u64, total: Option<u64>| {
            let pct = match total {
                Some(t) if t > 0 => ((done * 100) / t) as u8,
                _ => 0,
            };
            if pct != last_pct {
                last_pct = pct;
                progress.emit(&ProgressEvent::phase_download(
                    "downloading",
                    pct,
                    done,
                    total,
                ));
            }
        };

        match models::install(&root, &model, args.force, Some(&mut on_progress)) {
            Ok(InstallOutcome::AlreadyPresent(path)) => {
                if !args.quiet {
                    let _ = writeln_already(&path);
                }
                let p = path.display().to_string();
                progress.emit(&ProgressEvent::Done {
                    output: None,
                    model: Some(&model),
                    path: Some(&p),
                    duration_sec: None,
                    char_count: None,
                });
            }
            Ok(InstallOutcome::Installed(path)) => {
                let p = path.display().to_string();
                progress.emit(&ProgressEvent::Done {
                    output: None,
                    model: Some(&model),
                    path: Some(&p),
                    duration_sec: None,
                    char_count: None,
                });
            }
            Err(e) => {
                progress.emit(&ProgressEvent::Error {
                    code: "download_failed",
                    message: &e.to_string(),
                });
                return Err(CliError::with_code(e.exit_code(), e.to_string()));
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
