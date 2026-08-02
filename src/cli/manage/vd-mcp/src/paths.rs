//! Gateway configuration paths.

use std::path::PathBuf;

pub const ENV_CONFIG: &str = "VD_MCP_CONFIG";
pub const ENV_TRANSPORT: &str = "VD_TRANSPORT";
pub const ENV_TCP: &str = "VD_TCP";
pub const ENV_SOCKET: &str = "VD_SOCKET";

pub fn config_path() -> PathBuf {
    vd_artifact::paths::config_path("vd-mcp", ENV_CONFIG)
}
