//! Unit tests for vdctl.

use vdctl::config::{self, AutoBuild, PlatformConfig};
use vdctl::resolve;

#[test]
fn parses_platform_config() {
    let config: PlatformConfig = toml::from_str(
        r#"
        workspace = "/tmp/ws"
        auto_build = "always"
        auto_start_mcp = true
        "#,
    )
    .unwrap();
    assert_eq!(config.auto_build, AutoBuild::Always);
    assert!(config.auto_start_mcp);
    assert_eq!(
        config.workspace.as_deref().map(|p| p.to_string_lossy()),
        Some("/tmp/ws".into())
    );
}

#[test]
fn config_set_roundtrip() {
    let mut cfg = PlatformConfig::default();
    config::set(&mut cfg, "auto_build", "never").unwrap();
    assert_eq!(cfg.auto_build, AutoBuild::Never);
    assert_eq!(config::get(&cfg, "auto_build").as_deref(), Some("never"));
}

#[test]
fn detects_workspace_mode_from_crate() {
    let cfg = PlatformConfig::default();
    // Unit test runs with CWD somewhere under the workspace when cargo test is used.
    let platform = resolve::detect(&cfg).unwrap();
    // From manage/vdctl, walking up finds the repo workspace.
    assert!(
        platform.mode == resolve::Mode::Workspace || platform.vd_srv().parent().is_some(),
        "expected workspace or install resolution"
    );
}

#[test]
fn discover_agents_returns_known_ids() {
    let agents = vdctl::agents::discover_agents();
    let ids: Vec<_> = agents.iter().map(|a| a.id.as_str()).collect();
    assert!(ids.contains(&"claude"));
    assert!(ids.contains(&"cursor"));
    assert!(ids.contains(&"chatgpt"));
    assert!(ids.contains(&"vscode"));
    assert!(ids.contains(&"codex"));
}

#[test]
fn builtin_agent_adapters_parse() {
    let file = vdctl::agents::builtin_adapters();
    assert!(!file.agent.is_empty());
    assert!(file.agent.iter().any(|a| a.id == "claude"));
}

#[test]
fn discovers_repo_skills() {
    let cfg = PlatformConfig::default();
    let platform = resolve::detect(&cfg).unwrap();
    let report = vdctl::skills::discover(&platform);
    let ids: Vec<_> = report.skills.iter().map(|s| s.id.as_str()).collect();
    assert!(
        ids.contains(&"vd-audio") && ids.contains(&"vd-meeting"),
        "expected sample skills, got {ids:?} root={}",
        report.root
    );
}
