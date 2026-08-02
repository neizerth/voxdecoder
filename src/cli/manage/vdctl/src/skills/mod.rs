//! Skill discovery, validation, and install into AI clients.
//!
//! Skills are platform assets under `skills/<id>/skill.md`.
//! `vdctl` owns the lifecycle; `vd-mcp` never discovers or installs Skills.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::json;

use crate::agents::{self, AgentAdapter};
use crate::error::Error;
use crate::resolve::Platform;

const SKILL_FILE: &str = "skill.md";

#[derive(Debug, Clone, Serialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub path: String,
    pub skill_md: String,
    pub has_examples: bool,
    pub has_readme: bool,
    pub valid: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiscoverReport {
    pub root: String,
    pub skills: Vec<Skill>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillAppStatus {
    pub id: String,
    pub name: String,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillStatusRow {
    pub id: String,
    pub name: String,
    pub apps: Vec<SkillAppStatus>,
}

/// Resolve skills catalog root: workspace `skills/` or installed `$VD_HOME/skills`.
pub fn skills_root(platform: &Platform) -> PathBuf {
    if let Some(ws) = &platform.workspace {
        let candidate = ws.join("skills");
        if candidate.is_dir() {
            return candidate;
        }
    }
    crate::paths::home_dir().join("skills")
}

pub fn discover(platform: &Platform) -> DiscoverReport {
    let root = skills_root(platform);
    let mut skills = Vec::new();
    let mut diagnostics = Vec::new();

    if !root.is_dir() {
        diagnostics.push(format!("skills root missing: {}", root.display()));
        return DiscoverReport {
            root: root.display().to_string(),
            skills,
            diagnostics,
        };
    }

    let Ok(entries) = fs::read_dir(&root) else {
        diagnostics.push(format!("cannot read {}", root.display()));
        return DiscoverReport {
            root: root.display().to_string(),
            skills,
            diagnostics,
        };
    };

    let mut seen = std::collections::HashSet::new();
    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();

    for dir in dirs {
        let id = dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if id.is_empty() || id.starts_with('.') {
            continue;
        }
        let skill_md = dir.join(SKILL_FILE);
        if !skill_md.is_file() {
            // Not a skill directory — ignored by design.
            continue;
        }
        if !seen.insert(id.clone()) {
            diagnostics.push(format!("duplicate skill id rejected: {id}"));
            continue;
        }
        match load_skill(&id, &dir, &skill_md) {
            Ok(skill) => {
                if !skill.valid {
                    diagnostics.extend(
                        skill
                            .diagnostics
                            .iter()
                            .map(|d| format!("{id}: {d}")),
                    );
                }
                skills.push(skill);
            }
            Err(e) => {
                diagnostics.push(format!("skip {id}: {e}"));
            }
        }
    }

    skills.sort_by(|a, b| a.id.cmp(&b.id));
    DiscoverReport {
        root: root.display().to_string(),
        skills,
        diagnostics,
    }
}

fn load_skill(id: &str, dir: &Path, skill_md: &Path) -> Result<Skill, String> {
    let body = fs::read_to_string(skill_md).map_err(|e| e.to_string())?;
    let mut diagnostics = Vec::new();
    if body.trim().is_empty() {
        diagnostics.push("skill.md is empty".into());
    }
    let (name, description) = parse_title_and_blurb(&body);
    let name = name.unwrap_or_else(|| id.to_string());
    let valid = diagnostics.is_empty();
    Ok(Skill {
        id: id.to_string(),
        name,
        description,
        path: dir.display().to_string(),
        skill_md: skill_md.display().to_string(),
        has_examples: dir.join("examples").is_dir(),
        has_readme: dir.join("README.md").is_file(),
        valid,
        diagnostics,
    })
}

fn parse_title_and_blurb(body: &str) -> (Option<String>, String) {
    let mut name = None;
    let mut blurb = String::new();
    for line in body.lines() {
        let t = line.trim();
        if t.is_empty() {
            if name.is_some() && !blurb.is_empty() {
                break;
            }
            continue;
        }
        if name.is_none() {
            if let Some(rest) = t.strip_prefix('#') {
                name = Some(rest.trim().trim_start_matches('#').trim().to_string());
                continue;
            }
            name = Some(t.to_string());
            continue;
        }
        if t.starts_with('#') {
            break;
        }
        if blurb.is_empty() {
            blurb = t.to_string();
        } else {
            break;
        }
    }
    (name, blurb)
}

pub fn list(platform: &Platform, json: bool) -> Result<(), Error> {
    let report = discover(platform);
    let value = json!(report);
    crate::output::emit_value(json, value, |v| {
        let Some(arr) = v["skills"].as_array() else {
            return;
        };
        if arr.is_empty() {
            println!("No skills under {}", v["root"].as_str().unwrap_or(""));
            println!("Add skills/<id>/skill.md");
            return;
        }
        for s in arr {
            let mark = if s["valid"].as_bool().unwrap_or(false) {
                "✔"
            } else {
                "✘"
            };
            println!("{mark} {}", s["id"].as_str().unwrap_or(""));
        }
        if let Some(diags) = v["diagnostics"].as_array() {
            for d in diags {
                if let Some(msg) = d.as_str() {
                    eprintln!("! {msg}");
                }
            }
        }
    })
}

pub fn inspect(platform: &Platform, id: &str, json: bool) -> Result<(), Error> {
    let report = discover(platform);
    let Some(skill) = report.skills.iter().find(|s| s.id == id) else {
        return Err(Error::Usage(format!("skill not found: {id}")));
    };
    let value = json!(skill);
    crate::output::emit_value(json, value, |v| {
        println!("id           {}", v["id"].as_str().unwrap_or(""));
        println!("name         {}", v["name"].as_str().unwrap_or(""));
        println!("description  {}", v["description"].as_str().unwrap_or(""));
        println!("location     {}", v["path"].as_str().unwrap_or(""));
        println!(
            "examples     {}",
            if v["has_examples"].as_bool().unwrap_or(false) {
                "yes"
            } else {
                "no"
            }
        );
        println!(
            "readme       {}",
            if v["has_readme"].as_bool().unwrap_or(false) {
                "yes"
            } else {
                "no"
            }
        );
        println!(
            "valid        {}",
            if v["valid"].as_bool().unwrap_or(false) {
                "yes"
            } else {
                "no"
            }
        );
    })
}

pub fn validate(platform: &Platform, json: bool) -> Result<(), Error> {
    let report = discover(platform);
    let ok = report.diagnostics.is_empty() && report.skills.iter().all(|s| s.valid);
    let value = json!({
        "ok": ok,
        "root": report.root,
        "skills": report.skills,
        "diagnostics": report.diagnostics,
    });
    crate::output::emit_value(json, value, |v| {
        let Some(arr) = v["skills"].as_array() else {
            return;
        };
        for s in arr {
            let mark = if s["valid"].as_bool().unwrap_or(false) {
                "✔"
            } else {
                "✘"
            };
            println!("{mark} {}", s["id"].as_str().unwrap_or(""));
        }
        if let Some(diags) = v["diagnostics"].as_array() {
            for d in diags {
                if let Some(msg) = d.as_str() {
                    println!("! {msg}");
                }
            }
        }
        if !ok {
            eprintln!("skills validate failed");
        }
    })?;
    if ok {
        Ok(())
    } else {
        Err(Error::Message("skills validate failed".into()))
    }
}

pub fn status(platform: &Platform, json: bool) -> Result<(), Error> {
    let report = discover(platform);
    let agents = agents::discover_agents()
        .into_iter()
        .filter(|a| a.installed)
        .collect::<Vec<_>>();
    let rows: Vec<SkillStatusRow> = report
        .skills
        .iter()
        .map(|skill| SkillStatusRow {
            id: skill.id.clone(),
            name: skill.name.clone(),
            apps: agents
                .iter()
                .map(|a| {
                    let adapter = agents::adapter_by_id(&a.id);
                    let installed = adapter
                        .as_ref()
                        .map(|ad| skill_installed(ad, &skill.id))
                        .unwrap_or(false);
                    SkillAppStatus {
                        id: a.id.clone(),
                        name: a.name.clone(),
                        installed,
                    }
                })
                .collect(),
        })
        .collect();
    let value = json!({ "skills": rows, "root": report.root });
    crate::output::emit_value(json, value, |v| {
        let Some(arr) = v["skills"].as_array() else {
            return;
        };
        for s in arr {
            println!("{}", s["id"].as_str().unwrap_or(""));
            println!();
            if let Some(apps) = s["apps"].as_array() {
                let width = apps
                    .iter()
                    .filter_map(|a| a["name"].as_str())
                    .map(str::len)
                    .max()
                    .unwrap_or(12)
                    .max(12);
                for a in apps {
                    let mark = if a["installed"].as_bool().unwrap_or(false) {
                        "✔"
                    } else {
                        "✘"
                    };
                    println!(
                        "  {:<width$} {mark}",
                        a["name"].as_str().unwrap_or(""),
                        width = width
                    );
                }
            }
            println!();
        }
    })
}

pub fn skill_installed(adapter: &AgentAdapter, skill_id: &str) -> bool {
    adapter.skill_dir_paths().iter().any(|root| {
        root.join(skill_id).join(SKILL_FILE).is_file()
    })
}

pub fn install_skill(
    adapter: &AgentAdapter,
    skill: &Skill,
    dry_run: bool,
) -> Result<(), Error> {
    let dirs = adapter.skill_dir_paths();
    if dirs.is_empty() {
        eprintln!("  · {} — no skill_dirs in adapter (skipped)", adapter.name);
        return Ok(());
    }
    let dest_root = &dirs[0];
    let dest = dest_root.join(&skill.id);
    let dest_md = dest.join(SKILL_FILE);

    if dry_run {
        eprintln!(
            "  [dry-run] would install {} → {}",
            skill.id,
            dest_md.display()
        );
        return Ok(());
    }

    fs::create_dir_all(&dest).map_err(|e| Error::Message(e.to_string()))?;
    fs::copy(&skill.skill_md, &dest_md).map_err(|e| Error::Message(e.to_string()))?;

    let src = PathBuf::from(&skill.path);
    let readme = src.join("README.md");
    if readme.is_file() {
        let _ = fs::copy(&readme, dest.join("README.md"));
    }
    let examples = src.join("examples");
    if examples.is_dir() {
        copy_dir_recursive(&examples, &dest.join("examples"))?;
    }
    Ok(())
}

pub fn remove_skill(adapter: &AgentAdapter, skill_id: &str, dry_run: bool) -> Result<(), Error> {
    for root in adapter.skill_dir_paths() {
        let dest = root.join(skill_id);
        if !dest.exists() {
            continue;
        }
        if dry_run {
            eprintln!("  [dry-run] would remove {}", dest.display());
            continue;
        }
        fs::remove_dir_all(&dest).map_err(|e| Error::Message(e.to_string()))?;
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), Error> {
    fs::create_dir_all(dst).map_err(|e| Error::Message(e.to_string()))?;
    for ent in fs::read_dir(src).map_err(|e| Error::Message(e.to_string()))? {
        let ent = ent.map_err(|e| Error::Message(e.to_string()))?;
        let from = ent.path();
        let to = dst.join(ent.file_name());
        if from.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|e| Error::Message(e.to_string()))?;
        }
    }
    Ok(())
}

pub fn select_skills(
    report: &DiscoverReport,
    only: Option<&[String]>,
    exclude: &[String],
) -> Result<Vec<Skill>, Error> {
    let exclude: std::collections::HashSet<_> =
        exclude.iter().map(|s| s.to_ascii_lowercase()).collect();
    let mut skills: Vec<_> = report
        .skills
        .iter()
        .filter(|s| s.valid)
        .filter(|s| !exclude.contains(&s.id.to_ascii_lowercase()))
        .cloned()
        .collect();

    if let Some(only) = only {
        let wanted: std::collections::HashSet<_> =
            only.iter().map(|s| s.to_ascii_lowercase()).collect();
        skills.retain(|s| wanted.contains(&s.id.to_ascii_lowercase()));
        for id in &wanted {
            if !skills.iter().any(|s| s.id.eq_ignore_ascii_case(id))
                && !report.skills.iter().any(|s| s.id.eq_ignore_ascii_case(id))
            {
                return Err(Error::Usage(format!("unknown skill: {id}")));
            }
            if report
                .skills
                .iter()
                .any(|s| s.id.eq_ignore_ascii_case(id) && !s.valid)
            {
                return Err(Error::Message(format!("skill invalid: {id}")));
            }
        }
    }
    Ok(skills)
}
