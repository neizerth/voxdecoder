//! `vd-pipeline prune` — delete old cache entries.

use std::fs;
use std::time::SystemTime;

use super::{CliError, PruneCli};

pub fn execute(args: PruneCli) -> Result<(), CliError> {
    let cache_root = vd_artifact::job_cache_root();
    if !cache_root.exists() {
        println!("cache empty or not found: {}", cache_root.display());
        return Ok(());
    }

    let older_than_secs = parse_duration(&args.older_than)
        .ok_or_else(|| CliError::usage(format!("invalid duration: {}", args.older_than)))?;

    let now = SystemTime::now();
    let cutoff = now - std::time::Duration::from_secs(older_than_secs);

    let mut candidates = Vec::new();
    if let Ok(entries) = fs::read_dir(&cache_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if let Ok(meta) = path.metadata() {
                if let Ok(modified) = meta.modified() {
                    if modified < cutoff {
                        if let Some(key) = path.file_name().and_then(|n| n.to_str()) {
                            let size = du_bytes(&path).unwrap_or(0);
                            candidates.push((key.to_string(), path, size));
                        }
                    }
                }
            }
        }
    }

    if candidates.is_empty() {
        println!("no entries older than {}", args.older_than);
        return Ok(());
    }

    let total_size: u64 = candidates.iter().map(|(_, _, s)| s).sum();
    println!(
        "found {} entries ({} bytes) to delete:",
        candidates.len(),
        total_size
    );
    for (key, path, size) in &candidates {
        println!("  {} ({} bytes) @ {}", key, size, path.display());
    }

    if !args.force {
        println!("dry-run only (use --force to delete)");
        return Ok(());
    }

    if !args.yes {
        eprintln!("confirm deletion? (y/N): ");
        let mut buf = String::new();
        std::io::stdin()
            .read_line(&mut buf)
            .map_err(|e| CliError::usage(format!("read stdin: {e}")))?;
        if !buf.trim().eq_ignore_ascii_case("y") {
            println!("aborted");
            return Ok(());
        }
    }

    for (key, path, _size) in &candidates {
        if let Err(e) = fs::remove_dir_all(path) {
            eprintln!("failed to delete {}: {}", key, e);
        } else {
            println!("deleted: {}", key);
        }
    }

    Ok(())
}

fn parse_duration(s: &str) -> Option<u64> {
    let (num_str, unit) = if let Some(pos) = s.find(|c: char| !c.is_ascii_digit()) {
        s.split_at(pos)
    } else {
        (s, "s")
    };

    let num: u64 = num_str.parse().ok()?;
    Some(match unit {
        "s" | "sec" | "second" | "seconds" => num,
        "m" | "min" | "minute" | "minutes" => num * 60,
        "h" | "hour" | "hours" => num * 3600,
        "d" | "day" | "days" => num * 86400,
        "w" | "week" | "weeks" => num * 604800,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_common_units() {
        assert_eq!(parse_duration("7d"), Some(7 * 86400));
        assert_eq!(parse_duration("24h"), Some(24 * 3600));
        assert_eq!(parse_duration("1w"), Some(604800));
        assert_eq!(parse_duration("30m"), Some(30 * 60));
        assert_eq!(parse_duration("45s"), Some(45));
    }

    #[test]
    fn bare_number_defaults_to_seconds() {
        assert_eq!(parse_duration("100"), Some(100));
    }

    #[test]
    fn rejects_unknown_unit() {
        assert_eq!(parse_duration("7x"), None);
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse_duration(""), None);
        assert_eq!(parse_duration("abc"), None);
    }

    #[test]
    fn default_cli_value_parses() {
        assert_eq!(parse_duration("7d"), Some(604800));
    }
}

fn du_bytes(path: &std::path::Path) -> std::io::Result<u64> {
    let mut size = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        if meta.is_dir() {
            size += du_bytes(&entry.path())?;
        } else {
            size += meta.len();
        }
    }
    Ok(size)
}
