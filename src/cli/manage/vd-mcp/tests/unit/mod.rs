//! Unit tests for the MCP gateway.

use vd_mcp::config::GatewayConfig;
use vd_mcp::mcp::tools;

#[test]
fn exposes_expected_tools() {
    let names: Vec<_> = tools::list()
        .into_iter()
        .filter_map(|tool| {
            tool.get("name")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .collect();
    assert_eq!(
        names,
        [
            "process_audio".to_string(),
            "process_meeting".to_string(),
            "submit_job".to_string(),
            "get_job".to_string(),
            "cancel_job".to_string(),
            "list_jobs".to_string(),
            "list_artifacts".to_string(),
            "health".to_string(),
            "doctor".to_string(),
            "server_info".to_string()
        ]
    );
}

#[test]
fn parses_gateway_config() {
    let config: GatewayConfig = toml::from_str(
        r#"
        transport = "tcp"
        tcp = "127.0.0.1:7701"
        "#,
    )
    .unwrap();
    assert_eq!(config.transport.as_deref(), Some("tcp"));
    assert_eq!(config.tcp.as_deref(), Some("127.0.0.1:7701"));
}
