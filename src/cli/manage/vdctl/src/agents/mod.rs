//! Discover AI client applications via declarative adapters.

mod cli_mcp;
mod mcp_config;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::Error;
use crate::paths;

pub use cli_mcp::{
    install as install_cli_mcp, is_registered as cli_mcp_registered, list_output as cli_mcp_list,
    uninstall as uninstall_cli_mcp,
};
pub use mcp_config::{install_mcp, uninstall_mcp, McpServerSpec};

const BUILTIN_ADAPTERS: &str = include_str!("adapters.toml");

const DEFAULT_MARKERS: &[&str] = &["vd-mcp", "voxdecoder", "\"vd_mcp\"", "vdmcp"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AppKind {
    #[default]
    Desktop,
    Cli,
    Custom,
}

impl AppKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::Cli => "cli",
            Self::Custom => "custom",
        }
    }

    pub fn uses_bundle(self) -> bool {
        matches!(self, Self::Desktop)
    }

    pub fn uses_cli_mcp(self) -> bool {
        matches!(self, Self::Cli)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum McpFormat {
    #[default]
    McpServers,
    Servers,
    None,
}

impl McpFormat {
    pub fn supports_json_mcp(self) -> bool {
        matches!(self, Self::McpServers | Self::Servers)
    }

    pub fn key(self) -> Option<&'static str> {
        match self {
            Self::McpServers => Some("mcpServers"),
            Self::Servers => Some("servers"),
            Self::None => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub kind: AppKind,
    pub installed: bool,
    /// MCP / Bundle integration present for VoxDecoder.
    pub configured: bool,
    pub mcp_format: McpFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skill_dirs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AdaptersFile {
    #[serde(default)]
    pub agent: Vec<AgentAdapter>,
    #[serde(default)]
    pub configured_markers: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct OsPaths {
    #[serde(default)]
    pub app_paths: Vec<String>,
    #[serde(default)]
    pub marker_dirs: Vec<String>,
    #[serde(default)]
    pub bins: Vec<String>,
    #[serde(default)]
    pub config_paths: Vec<String>,
    #[serde(default)]
    pub skill_dirs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentAdapter {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: AppKind,
    #[serde(default)]
    pub mcp_format: McpFormat,
    #[serde(default)]
    pub app_paths: Vec<String>,
    #[serde(default)]
    pub marker_dirs: Vec<String>,
    #[serde(default)]
    pub bins: Vec<String>,
    #[serde(default)]
    pub config_paths: Vec<String>,
    #[serde(default)]
    pub skill_dirs: Vec<String>,
    /// CLI MCP server name (default `voxdecoder`).
    #[serde(default)]
    pub mcp_server_name: Option<String>,
    /// Claude Code scope: local | user | project.
    #[serde(default)]
    pub mcp_scope: Option<String>,
    #[serde(default)]
    pub configured_markers: Vec<String>,
    #[serde(default)]
    pub macos: Option<OsPaths>,
    #[serde(default)]
    pub linux: Option<OsPaths>,
    #[serde(default)]
    pub windows: Option<OsPaths>,
}

#[derive(Debug, Default, Clone)]
pub struct ResolvedPaths {
    pub app_paths: Vec<String>,
    pub marker_dirs: Vec<String>,
    pub bins: Vec<String>,
    pub config_paths: Vec<String>,
    pub skill_dirs: Vec<String>,
}

impl AgentAdapter {
    pub fn resolve_for_host(&self) -> ResolvedPaths {
        let mut out = ResolvedPaths {
            app_paths: self.app_paths.clone(),
            marker_dirs: self.marker_dirs.clone(),
            bins: self.bins.clone(),
            config_paths: self.config_paths.clone(),
            skill_dirs: self.skill_dirs.clone(),
        };
        if let Some(os) = self.os_block() {
            out.app_paths.extend(os.app_paths.iter().cloned());
            out.marker_dirs.extend(os.marker_dirs.iter().cloned());
            out.bins.extend(os.bins.iter().cloned());
            out.config_paths.extend(os.config_paths.iter().cloned());
            out.skill_dirs.extend(os.skill_dirs.iter().cloned());
        }
        out
    }

    fn os_block(&self) -> Option<&OsPaths> {
        match std::env::consts::OS {
            "macos" => self.macos.as_ref(),
            "linux" => self.linux.as_ref(),
            "windows" => self.windows.as_ref(),
            _ => None,
        }
    }

    pub fn has_cli_mcp(&self) -> bool {
        self.kind.uses_cli_mcp()
            && self.mcp_server_name.as_ref().is_some_and(|s| !s.is_empty())
            && !self.resolve_for_host().bins.is_empty()
    }

    /// Preferred MCP config file (first path; created on install if missing).
    pub fn preferred_config_path(&self) -> Option<PathBuf> {
        let resolved = self.resolve_for_host();
        let existing = first_existing(
            &resolved
                .config_paths
                .iter()
                .map(|p| expand_path(p))
                .collect::<Vec<_>>(),
        );
        existing.or_else(|| resolved.config_paths.first().map(|p| expand_path(p)))
    }

    pub fn skill_dir_paths(&self) -> Vec<PathBuf> {
        self.resolve_for_host()
            .skill_dirs
            .iter()
            .map(|p| expand_path(p))
            .collect()
    }
}

pub fn discover_agents() -> Vec<Agent> {
    let file = load_adapters();
    let markers = effective_markers(&file);
    file.agent
        .iter()
        .map(|a| probe_adapter(a, &markers))
        .collect()
}

pub fn adapters() -> Vec<AgentAdapter> {
    load_adapters().agent
}

pub fn adapter_by_id(id: &str) -> Option<AgentAdapter> {
    adapters().into_iter().find(|a| a.id == id)
}

pub fn agents_json() -> Value {
    let agents = discover_agents();
    json!({
        "agents": agents,
        "applications": agents,
    })
}

fn probe_adapter(adapter: &AgentAdapter, global_markers: &[String]) -> Agent {
    let resolved = adapter.resolve_for_host();

    let app_paths: Vec<PathBuf> = resolved.app_paths.iter().map(|p| expand_path(p)).collect();
    let app = first_existing(&app_paths);

    let marker_hit = resolved
        .marker_dirs
        .iter()
        .map(|p| expand_path(p))
        .any(|p| p.is_dir());

    let bin_path = resolved.bins.iter().find_map(|b| which(b));
    let installed = app.is_some() || marker_hit || bin_path.is_some();

    let config_candidates: Vec<PathBuf> = resolved
        .config_paths
        .iter()
        .map(|p| expand_path(p))
        .collect();
    let config_path = first_existing(&config_candidates).filter(|p| p.is_file());

    let markers = if adapter.configured_markers.is_empty() {
        global_markers
    } else {
        &adapter.configured_markers
    };

    let configured = if adapter.has_cli_mcp() {
        cli_mcp::is_registered(adapter)
    } else if adapter.kind.uses_cli_mcp() {
        // CLI app without an MCP installer adapter (e.g. Codex for now).
        false
    } else {
        config_path
            .as_ref()
            .is_some_and(|p| config_mentions(p, markers))
    };

    let skill_dirs = resolved
        .skill_dirs
        .iter()
        .map(|p| expand_path(p).display().to_string())
        .collect();

    Agent {
        id: adapter.id.clone(),
        name: adapter.name.clone(),
        kind: adapter.kind,
        installed,
        configured,
        mcp_format: adapter.mcp_format,
        app_path: app.or(bin_path).map(|p| p.display().to_string()),
        config_path: config_path.map(|p| p.display().to_string()),
        skill_dirs,
    }
}

pub fn load_adapters() -> AdaptersFile {
    let path = paths::agents_config_path();
    if path.is_file() {
        match fs::read_to_string(&path)
            .ok()
            .and_then(|raw| toml::from_str(&raw).ok())
        {
            Some(file) => return file,
            None => {
                eprintln!(
                    "warning: failed to parse {}; using built-in adapters",
                    path.display()
                );
            }
        }
    }
    builtin_adapters()
}

pub fn builtin_adapters() -> AdaptersFile {
    toml::from_str(BUILTIN_ADAPTERS).expect("built-in agents/adapters.toml must parse")
}

fn effective_markers(file: &AdaptersFile) -> Vec<String> {
    if file.configured_markers.is_empty() {
        DEFAULT_MARKERS.iter().map(|s| (*s).to_string()).collect()
    } else {
        file.configured_markers.clone()
    }
}

fn config_mentions(path: &Path, markers: &[String]) -> bool {
    let Ok(body) = fs::read_to_string(path) else {
        return false;
    };
    let lower = body.to_ascii_lowercase();
    markers
        .iter()
        .any(|m| lower.contains(&m.to_ascii_lowercase()))
}

pub fn expand_path(raw: &str) -> PathBuf {
    let expanded = expand_tokens(raw.trim());
    if let Some(rest) = expanded.strip_prefix("~/") {
        return dirs_home().join(rest);
    }
    if expanded == "~" {
        return dirs_home();
    }
    PathBuf::from(expanded)
}

fn expand_tokens(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' {
            if let Some(end) = chars[i + 1..].iter().position(|&c| c == '%') {
                let name: String = chars[i + 1..i + 1 + end].iter().collect();
                if !name.is_empty() {
                    if let Some(val) = env_lookup(&name) {
                        out.push_str(&val);
                        i += end + 2;
                        continue;
                    }
                }
            }
            out.push('%');
            i += 1;
            continue;
        }
        if chars[i] == '$' {
            if i + 1 < chars.len() && chars[i + 1] == '{' {
                if let Some(end) = chars[i + 2..].iter().position(|&c| c == '}') {
                    let name: String = chars[i + 2..i + 2 + end].iter().collect();
                    if let Some(val) = env_lookup(&name) {
                        out.push_str(&val);
                        i += end + 3;
                        continue;
                    }
                }
            } else {
                let mut j = i + 1;
                while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                    j += 1;
                }
                if j > i + 1 {
                    let name: String = chars[i + 1..j].iter().collect();
                    if let Some(val) = env_lookup(&name) {
                        out.push_str(&val);
                        i = j;
                        continue;
                    }
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn env_lookup(name: &str) -> Option<String> {
    match name {
        "HOME" | "USERPROFILE" => Some(dirs_home().display().to_string()),
        "XDG_CONFIG_HOME" => std::env::var("XDG_CONFIG_HOME")
            .ok()
            .filter(|s| !s.is_empty())
            .or_else(|| Some(dirs_home().join(".config").display().to_string())),
        other => std::env::var(other).ok().filter(|s| !s.is_empty()),
    }
}

fn dirs_home() -> PathBuf {
    directories::UserDirs::new()
        .map(|u| u.home_dir().to_path_buf())
        .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn first_existing(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|p| p.exists()).cloned()
}

fn which(name: &str) -> Option<PathBuf> {
    let (cmd, arg) = if cfg!(windows) {
        ("where", name)
    } else {
        ("which", name)
    };
    let output = Command::new(cmd).arg(arg).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

pub fn print_agents_human(agents: &[Agent]) {
    let desktop: Vec<_> = agents.iter().filter(|a| a.kind == AppKind::Desktop).collect();
    let cli: Vec<_> = agents.iter().filter(|a| a.kind == AppKind::Cli).collect();
    let other: Vec<_> = agents
        .iter()
        .filter(|a| a.kind == AppKind::Custom)
        .collect();

    if !desktop.is_empty() {
        println!("Desktop");
        for a in desktop {
            print_one(a);
        }
        println!();
    }
    if !cli.is_empty() {
        println!("CLI");
        for a in cli {
            print_one(a);
        }
        println!();
    }
    if !other.is_empty() {
        println!("Other");
        for a in other {
            print_one(a);
        }
        println!();
    }
}

fn print_one(a: &Agent) {
    let mark = if a.installed { "✔" } else { "✘" };
    let extra = if a.installed && a.configured {
        match a.kind {
            AppKind::Cli => " · MCP registered",
            _ => " · Bundle",
        }
    } else {
        ""
    };
    println!("    {mark} {}{extra}", a.name);
}

/// Map legacy / short aliases to adapter ids.
fn resolve_app_alias(id: &str) -> String {
    match id.to_ascii_lowercase().as_str() {
        "claude" => "claude-desktop".into(),
        other => other.to_string(),
    }
}

pub fn filter_installed_adapters(apps: Option<&[String]>) -> Result<Vec<AgentAdapter>, Error> {
    let installed_ids: std::collections::HashSet<_> = discover_agents()
        .into_iter()
        .filter(|a| a.installed)
        .map(|a| a.id)
        .collect();
    let mut selected: Vec<_> = adapters()
        .into_iter()
        .filter(|a| installed_ids.contains(&a.id))
        .collect();

    if let Some(apps) = apps {
        let wanted: std::collections::HashSet<_> = apps
            .iter()
            .map(|s| resolve_app_alias(s))
            .collect();
        selected.retain(|a| wanted.contains(&a.id.to_ascii_lowercase()));
        for id in &wanted {
            if !selected.iter().any(|a| a.id.eq_ignore_ascii_case(id)) {
                return Err(Error::Usage(format!(
                    "unknown or not installed app: {id}"
                )));
            }
        }
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_adapters_parse_and_cover_known_ids() {
        let file = builtin_adapters();
        let ids: Vec<_> = file.agent.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"claude-desktop"));
        assert!(ids.contains(&"claude-code"));
        assert!(ids.contains(&"cursor"));
        assert!(ids.contains(&"chatgpt"));
        assert!(ids.contains(&"vscode"));
        assert!(ids.contains(&"codex"));
        let code = file.agent.iter().find(|a| a.id == "claude-code").unwrap();
        assert_eq!(code.kind, AppKind::Cli);
    }

    #[test]
    fn expand_home_prefix() {
        let p = expand_path("~/.cursor/mcp.json");
        assert!(p.ends_with(".cursor/mcp.json"));
    }

    #[test]
    fn expand_xdg_config_home_fallback() {
        let p = expand_path("$XDG_CONFIG_HOME/Code/User/mcp.json");
        assert!(p.ends_with("Code/User/mcp.json"));
    }

    #[test]
    fn resolve_merges_os_block() {
        let adapter = AgentAdapter {
            id: "t".into(),
            name: "T".into(),
            kind: AppKind::Desktop,
            mcp_format: McpFormat::McpServers,
            app_paths: vec!["~/shared".into()],
            marker_dirs: vec![],
            bins: vec!["code".into()],
            config_paths: vec!["~/.shared.json".into()],
            skill_dirs: vec!["~/.skills".into()],
            mcp_server_name: None,
            mcp_scope: None,
            configured_markers: vec![],
            macos: Some(OsPaths {
                app_paths: vec!["/Applications/T.app".into()],
                ..OsPaths::default()
            }),
            linux: Some(OsPaths {
                app_paths: vec!["/opt/t".into()],
                ..OsPaths::default()
            }),
            windows: Some(OsPaths {
                app_paths: vec!["%LOCALAPPDATA%/T/t.exe".into()],
                ..OsPaths::default()
            }),
        };
        let resolved = adapter.resolve_for_host();
        assert!(resolved.app_paths.iter().any(|p| p == "~/shared"));
        assert_eq!(resolved.bins, vec!["code".to_string()]);
    }
}
