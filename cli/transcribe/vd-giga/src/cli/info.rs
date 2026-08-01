//! `vd-giga info`.

use std::fs;

use serde::Serialize;

use crate::cli::{CliError, InfoArgs};
use crate::gigaam::catalog::{self, decoder_kind, line_label, resolve_model_name, DecoderKind};
use crate::gigaam::weights;
use crate::paths;

#[derive(Serialize)]
struct InfoJson {
    name: String,
    decoder: String,
    line: String,
    language: String,
    installed: bool,
    downloaded: bool,
    path: String,
    size: Option<String>,
    sha256: Option<String>,
}

pub fn execute(args: InfoArgs) -> Result<(), CliError> {
    let root = paths::resolve_models_dir(None);
    let name = if looks_like_path(&args.model) {
        args.model.clone()
    } else {
        resolve_model_name(&args.model).to_string()
    };

    let converted = weights::resolve_converted(&root, &name).ok();
    let path = if looks_like_path(&args.model) {
        std::path::PathBuf::from(&args.model)
    } else if let Some(ref c) = converted {
        c.safetensors.clone()
    } else {
        weights::checkpoint_path(&root, &name)
    };

    let installed = if looks_like_path(&args.model) {
        path.is_file()
    } else {
        weights::is_installed(&root, &name)
    };
    let decoder = decoder_kind(&name)
        .map(|k| match k {
            DecoderKind::Ctc => "ctc",
            DecoderKind::Rnnt => "rnnt",
        })
        .unwrap_or("unknown");
    let line = if catalog::is_catalog_name(&name) {
        line_label(&name)
    } else {
        "local".into()
    };

    let size = if installed {
        fs::metadata(&path).ok().map(|m| format_size(m.len()))
    } else {
        None
    };
    // Catalog checksums / content hash land with real downloads.
    let sha256: Option<String> = None;

    if args.json {
        let body = InfoJson {
            name: name.clone(),
            decoder: decoder.into(),
            line,
            language: "ru".into(),
            installed,
            downloaded: installed,
            path: path.display().to_string(),
            size,
            sha256,
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&body).unwrap_or_default()
        );
        return Ok(());
    }

    println!("name:       {name}");
    println!("decoder:    {decoder}");
    println!("line:       {line}");
    println!("language:   ru");
    println!("installed:  {}", yes_no(installed));
    println!("downloaded: {}", yes_no(installed));
    println!("path:       {}", path.display());
    if let Some(s) = size {
        println!("size:       {s}");
    }
    if let Some(h) = sha256 {
        println!("sha256:     {h}");
    }
    Ok(())
}

fn yes_no(v: bool) -> &'static str {
    if v {
        "yes"
    } else {
        "no"
    }
}

fn looks_like_path(s: &str) -> bool {
    s.contains('/') || s.contains('\\') || s.ends_with(".ckpt") || s.ends_with(".pt")
}

fn format_size(bytes: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    if bytes as f64 >= MIB {
        format!("{:.0} MiB", bytes as f64 / MIB)
    } else {
        format!("{bytes} B")
    }
}
