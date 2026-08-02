//! Resolve Workspace vs Installed Platform and binary locations.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::config::{AutoBuild, BuildProfile, PlatformConfig};
use crate::error::Error;
use crate::paths;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Workspace,
    Installed,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::Installed => "installed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Platform {
    pub mode: Mode,
    pub workspace: Option<PathBuf>,
    pub build: BuildProfile,
    pub bin_dir: PathBuf,
    pub data_dir: PathBuf,
    pub socket: PathBuf,
    pub tcp: Option<String>,
    pub http: Option<String>,
    pub transport: String,
}

impl Platform {
    pub fn vd_srv(&self) -> PathBuf {
        self.bin_dir.join(bin_name("vd-srv"))
    }

    pub fn vd_mcp(&self) -> PathBuf {
        self.bin_dir.join(bin_name("vd-mcp"))
    }
}

pub fn detect(config: &PlatformConfig) -> Result<Platform, Error> {
    let cwd = env::current_dir().map_err(|e| Error::Message(e.to_string()))?;
    let workspace = find_workspace(&cwd).or_else(|| {
        config
            .workspace
            .as_ref()
            .filter(|p| p.join("Cargo.toml").is_file())
            .cloned()
    });

    let mode = if workspace.is_some() {
        Mode::Workspace
    } else {
        Mode::Installed
    };

    let build = resolve_build_profile(config);
    let bin_dir = match mode {
        Mode::Workspace => {
            let root = workspace.as_ref().expect("workspace mode");
            workspace_bin_dir(root, build)
        }
        Mode::Installed => installed_bin_dir()?,
    };

    let data_dir = config
        .data_dir
        .clone()
        .unwrap_or_else(crate::paths::runtime_data_dir);

    let transport = config
        .transport
        .clone()
        .or_else(|| env::var(crate::paths::ENV_TRANSPORT).ok())
        .unwrap_or_else(|| "auto".into());

    let tcp = config
        .tcp
        .clone()
        .or_else(|| env::var(crate::paths::ENV_TCP).ok());

    let http = config
        .http
        .clone()
        .or_else(|| env::var(crate::paths::ENV_HTTP).ok());

    let socket = config
        .socket
        .clone()
        .or_else(|| env::var(crate::paths::ENV_SOCKET).ok().map(PathBuf::from))
        .unwrap_or_else(|| vd_srv::paths::default_socket_path(&data_dir));

    Ok(Platform {
        mode,
        workspace,
        build,
        bin_dir,
        data_dir,
        socket,
        tcp,
        http,
        transport,
    })
}

fn resolve_build_profile(config: &PlatformConfig) -> BuildProfile {
    if let Ok(raw) = env::var(paths::ENV_BUILD) {
        if let Some(p) = BuildProfile::parse(&raw) {
            return p;
        }
    }
    config.build
}

pub fn ensure_runtime_built(
    platform: &Platform,
    auto_build: AutoBuild,
    build: BuildProfile,
) -> Result<(), Error> {
    if platform.mode != Mode::Workspace {
        return Ok(());
    }
    let root = platform
        .workspace
        .as_ref()
        .ok_or_else(|| Error::Message("workspace root missing".into()))?;
    let bin = platform.vd_srv();
    let need = match auto_build {
        AutoBuild::Never => false,
        AutoBuild::Always => true,
        AutoBuild::Missing => !bin.is_file(),
    };
    if !need {
        return Ok(());
    }
    build_workspace(root, build)
}

/// Build workspace Runtime packages for the selected profile (`scripts/build.sh`).
pub fn build_workspace(root: &Path, build: BuildProfile) -> Result<(), Error> {
    let script = root.join("scripts/build.sh");
    if !script.is_file() {
        return Err(Error::Message(format!(
            "build script missing: {}",
            script.display()
        )));
    }
    let profile_flag = match build {
        BuildProfile::Debug => "--debug",
        BuildProfile::Release => "--release",
    };
    eprintln!(
        "vdctl: building workspace ({}) via scripts/build.sh {profile_flag}…",
        build.as_str()
    );
    let status = Command::new(&script)
        .arg(profile_flag)
        .current_dir(root)
        .status()
        .map_err(|e| Error::Message(format!("build.sh failed to start: {e}")))?;
    if !status.success() {
        return Err(Error::Message(format!(
            "build.sh failed with status {status}"
        )));
    }
    Ok(())
}

pub fn refuse_release_ops(platform: &Platform) -> Result<(), Error> {
    if platform.mode == Mode::Workspace {
        return Err(Error::Workspace(
            "Running from a development workspace.\n\n\
             Updates are managed through Git.\n\
             Use:\n\n\
                 git pull\n\
                 cargo build\n"
                .into(),
        ));
    }
    Ok(())
}

fn find_workspace(start: &Path) -> Option<PathBuf> {
    let mut cur = Some(start);
    while let Some(dir) = cur {
        let cargo = dir.join("Cargo.toml");
        if cargo.is_file() {
            if let Ok(body) = fs::read_to_string(&cargo) {
                if body.contains("[workspace]") {
                    return Some(dir.to_path_buf());
                }
            }
        }
        cur = dir.parent();
    }
    None
}

fn workspace_bin_dir(root: &Path, build: BuildProfile) -> PathBuf {
    root.join("target").join(build.target_dir_name())
}

fn installed_bin_dir() -> Result<PathBuf, Error> {
    // Same directory as this vdctl binary (Release archive layout).
    let exe = env::current_exe().map_err(|e| Error::Message(e.to_string()))?;
    exe.parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| Error::Message("cannot resolve install bin dir".into()))
}

fn bin_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_workspace_from_crate_dir() {
        let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let ws = find_workspace(&here).expect("workspace");
        assert!(ws.join("Cargo.toml").is_file());
        let body = fs::read_to_string(ws.join("Cargo.toml")).unwrap();
        assert!(body.contains("[workspace]"));
    }

    #[test]
    fn workspace_bin_dir_respects_profile() {
        let root = PathBuf::from("/tmp/ws");
        assert_eq!(
            workspace_bin_dir(&root, BuildProfile::Debug),
            PathBuf::from("/tmp/ws/target/debug")
        );
        assert_eq!(
            workspace_bin_dir(&root, BuildProfile::Release),
            PathBuf::from("/tmp/ws/target/release")
        );
    }
}
