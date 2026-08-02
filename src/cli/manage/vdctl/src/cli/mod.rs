//! Command-line interface.

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::{Parser, Subcommand};
use serde_json::json;

use crate::assets;
use crate::client;
use crate::config::{self, PlatformConfig};
use crate::discover;
use crate::doctor;
use crate::error::Error;
use crate::lifecycle;
use crate::mcp;
use crate::paths;
use crate::resolve;
use crate::skills;
use crate::update;

#[derive(Debug, Parser)]
#[command(name = "vdctl", version, about = "VoxDecoder Platform CLI")]
struct Root {
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[arg(long, global = true)]
    json: bool,
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
    #[arg(long, global = true)]
    transport: Option<String>,
    #[arg(long, global = true)]
    tcp: Option<String>,
    #[arg(long, global = true)]
    socket: Option<PathBuf>,
    #[command(subcommand)]
    command: Command_,
}

#[derive(Debug, Subcommand)]
enum Command_ {
    /// Install the platform from GitHub Releases (Installed mode).
    Install,
    /// Update the platform from GitHub Releases (Installed mode).
    Update {
        #[arg(long)]
        channel: Option<String>,
    },
    /// Uninstall the platform.
    Uninstall {
        #[arg(long)]
        purge: bool,
    },
    /// Start Runtime (+ MCP if configured).
    Up,
    /// Stop Runtime (and MCP).
    Down,
    /// Restart Runtime.
    Restart,
    /// Runtime status.
    Status,
    /// Wait until Runtime API is ready.
    Wait {
        #[arg(long, default_value_t = 30)]
        timeout: u64,
    },
    /// Runtime / platform health (Operator API).
    Health,
    /// Platform doctor.
    Doctor,
    /// Platform info.
    Info,
    /// Resolved paths.
    Paths,
    /// Effective environment.
    Env {
        #[command(subcommand)]
        action: Option<EnvAction>,
    },
    /// Inventory snapshot.
    Discover,
    /// Full install snapshot.
    Inspect,
    /// Print versions.
    Version,
    /// MCP gateway lifecycle / registration.
    Mcp {
        #[command(subcommand)]
        action: McpAction,
    },
    /// AI skills (content) — independent from Runtime / MCP.
    Skills {
        #[command(subcommand)]
        action: SkillsAction,
    },
    /// Asset management.
    Assets {
        #[command(subcommand)]
        action: AssetsAction,
    },
    /// Platform config.
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Operator API passthrough.
    Api {
        method: String,
        #[arg(long)]
        params: Option<String>,
    },
    /// Developer helpers.
    Dev {
        #[command(subcommand)]
        action: DevAction,
    },
    /// Symlink this vdctl onto the user PATH (~/.cargo/bin).
    Link {
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Open a shell with platform env.
    Shell,
    /// Reset local state (stub).
    Reset {
        #[arg(value_name = "TARGET")]
        target: String,
    },
}

#[derive(Debug, Subcommand)]
enum EnvAction {
    Export,
}

#[derive(Debug, Subcommand)]
enum McpAction {
    Start,
    Stop,
    Restart,
    /// Gateway process + Bundle status.
    Status,
    /// Build `$VD_HOME/bundles/voxdecoder.mcpb`.
    Build {
        #[arg(long)]
        dry_run: bool,
    },
    /// Sync Skills, build Bundle, install into AI apps, verify.
    Install {
        #[arg(long)]
        apps: Option<String>,
        #[arg(long)]
        skills: Option<String>,
        #[arg(long)]
        exclude: Option<String>,
        #[arg(long)]
        no_skills: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Rebuild Bundle and reinstall (same as install).
    Update {
        #[arg(long)]
        apps: Option<String>,
        #[arg(long)]
        skills: Option<String>,
        #[arg(long)]
        exclude: Option<String>,
        #[arg(long)]
        no_skills: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Remove Bundle and/or Skills from AI apps.
    Uninstall {
        #[arg(long)]
        apps: Option<String>,
        #[arg(long)]
        skills: Option<String>,
        #[arg(long)]
        exclude: Option<String>,
        #[arg(long)]
        no_skills: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Verify Bundle, Gateway, Runtime, Skills.
    Verify,
    List,
}

#[derive(Debug, Subcommand)]
enum SkillsAction {
    /// List discovered Skills (`skills/*/skill.md`).
    List,
    /// Inspect one Skill.
    Inspect { id: String },
    /// Validate all Skills (CI).
    Validate,
    /// Installation state per AI application.
    Status,
}

#[derive(Debug, Subcommand)]
enum AssetsAction {
    List,
    Install { name: String },
    Update { name: Option<String> },
    Remove { name: String },
}

#[derive(Debug, Subcommand)]
enum ConfigAction {
    Get { key: String },
    Set { key: String, value: String },
    Path,
    List,
    Edit,
}

#[derive(Debug, Subcommand)]
enum DevAction {
    /// Register current/repo workspace path in vdctl.toml.
    Init {
        #[arg(long)]
        path: Option<PathBuf>,
        /// Do not symlink vdctl onto ~/.cargo/bin.
        #[arg(long)]
        no_link: bool,
    },
}

pub fn run<I, T>(args: I) -> Result<(), Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let root = match Root::try_parse_from(args) {
        Ok(root) => root,
        Err(e) => {
            let msg = e.to_string();
            if matches!(
                e.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) {
                print!("{msg}");
                return Ok(());
            }
            return Err(Error::Usage(msg));
        }
    };

    let config_path = root.config.clone().unwrap_or_else(paths::config_path);
    let mut cfg = config::load(&config_path).map_err(Error::Message)?;
    apply_overrides(&mut cfg, &root);

    let platform = resolve::detect(&cfg)?;

    match root.command {
        Command_::Install => update::install(&platform),
        Command_::Update { channel } => update::update(&platform, channel.as_deref()),
        Command_::Uninstall { purge } => update::uninstall(&platform, purge),
        Command_::Up => lifecycle::up(&platform, &cfg),
        Command_::Down => lifecycle::down(&platform),
        Command_::Restart => lifecycle::restart(&platform, &cfg),
        Command_::Status => lifecycle::status(&platform, root.json),
        Command_::Wait { timeout } => lifecycle::wait(&platform, timeout),
        Command_::Health => discover::health(&platform, root.json),
        Command_::Doctor => doctor::run(&platform, root.json),
        Command_::Info => discover::info(&platform, root.json),
        Command_::Paths => discover::paths_cmd(&platform, root.json),
        Command_::Env { action } => {
            discover::env_cmd(&platform, matches!(action, Some(EnvAction::Export)))
        }
        Command_::Discover => discover::discover(&platform, root.json),
        Command_::Inspect => discover::inspect(&platform, root.json),
        Command_::Version => version_cmd(root.json),
        Command_::Mcp { action } => match action {
            McpAction::Start => mcp::start(&platform),
            McpAction::Stop => mcp::stop(&platform),
            McpAction::Restart => mcp::restart(&platform),
            McpAction::Status => mcp::status(&platform, root.json),
            McpAction::Build { dry_run } => mcp::build(&platform, dry_run),
            McpAction::Install {
                apps,
                skills,
                exclude,
                no_skills,
                dry_run,
            } => mcp::install(
                &platform,
                &mcp::InstallOpts {
                    apps: mcp::parse_csv_list(apps.as_deref()),
                    skills: mcp::parse_csv_list(skills.as_deref()),
                    exclude: mcp::parse_csv_list(exclude.as_deref()).unwrap_or_default(),
                    no_skills,
                    dry_run,
                },
            ),
            McpAction::Update {
                apps,
                skills,
                exclude,
                no_skills,
                dry_run,
            } => mcp::update(
                &platform,
                &mcp::InstallOpts {
                    apps: mcp::parse_csv_list(apps.as_deref()),
                    skills: mcp::parse_csv_list(skills.as_deref()),
                    exclude: mcp::parse_csv_list(exclude.as_deref()).unwrap_or_default(),
                    no_skills,
                    dry_run,
                },
            ),
            McpAction::Uninstall {
                apps,
                skills,
                exclude,
                no_skills,
                dry_run,
            } => mcp::uninstall(
                &platform,
                &mcp::InstallOpts {
                    apps: mcp::parse_csv_list(apps.as_deref()),
                    skills: mcp::parse_csv_list(skills.as_deref()),
                    exclude: mcp::parse_csv_list(exclude.as_deref()).unwrap_or_default(),
                    no_skills,
                    dry_run,
                },
            ),
            McpAction::Verify => mcp::verify(&platform, root.json),
            McpAction::List => mcp::list(&platform, root.json),
        },
        Command_::Skills { action } => match action {
            SkillsAction::List => skills::list(&platform, root.json),
            SkillsAction::Inspect { id } => skills::inspect(&platform, &id, root.json),
            SkillsAction::Validate => skills::validate(&platform, root.json),
            SkillsAction::Status => skills::status(&platform, root.json),
        },
        Command_::Assets { action } => match action {
            AssetsAction::List => assets::list(&platform, root.json),
            AssetsAction::Install { name } => assets::install(&name),
            AssetsAction::Update { name } => assets::update(name.as_deref()),
            AssetsAction::Remove { name } => assets::remove(&name),
        },
        Command_::Config { action } => config_cmd(&config_path, &mut cfg, action, root.json),
        Command_::Api { method, params } => {
            let params = match params {
                Some(raw) => Some(serde_json::from_str(&raw).map_err(|e| Error::Usage(e.to_string()))?),
                None => None,
            };
            let value = client::call(&platform, &method, params)?;
            crate::output::emit_json(&value)
        }
        Command_::Dev { action } => match action {
            DevAction::Init { path, no_link } => dev_init(&config_path, &mut cfg, path, !no_link),
        },
        Command_::Link { path } => link_cmd(path.as_deref(), &cfg),
        Command_::Shell => shell_cmd(&platform),
        Command_::Reset { target } => Err(Error::NotImplemented(format!(
            "vdctl reset {target} is not implemented yet"
        ))),
    }
}

fn apply_overrides(cfg: &mut PlatformConfig, root: &Root) {
    if let Some(v) = &root.data_dir {
        cfg.data_dir = Some(v.clone());
    }
    if let Some(v) = &root.transport {
        cfg.transport = Some(v.clone());
    }
    if let Some(v) = &root.tcp {
        cfg.tcp = Some(v.clone());
    }
    if let Some(v) = &root.socket {
        cfg.socket = Some(v.clone());
    }
}

fn version_cmd(json: bool) -> Result<(), Error> {
    let value = json!({
        "vdctl": env!("CARGO_PKG_VERSION"),
        "platform": env!("CARGO_PKG_VERSION"),
    });
    crate::output::emit_value(json, value, |v| {
        println!("vdctl {}", v["vdctl"].as_str().unwrap_or(""));
    })
}

fn config_cmd(
    path: &Path,
    cfg: &mut PlatformConfig,
    action: ConfigAction,
    json: bool,
) -> Result<(), Error> {
    match action {
        ConfigAction::Path => {
            println!("{}", path.display());
            Ok(())
        }
        ConfigAction::List => {
            let value = json!({
                "workspace": cfg.workspace.as_ref().map(|p| p.display().to_string()),
                "auto_build": cfg.auto_build.as_str(),
                "auto_start_mcp": cfg.auto_start_mcp,
                "transport": cfg.transport,
                "tcp": cfg.tcp,
                "socket": cfg.socket.as_ref().map(|p| p.display().to_string()),
                "data_dir": cfg.data_dir.as_ref().map(|p| p.display().to_string()),
            });
            crate::output::emit_value(json, value, |v| {
                println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
            })
        }
        ConfigAction::Get { key } => {
            let Some(v) = config::get(cfg, &key) else {
                return Err(Error::Usage(format!("unknown or unset key: {key}")));
            };
            println!("{v}");
            Ok(())
        }
        ConfigAction::Set { key, value } => {
            config::set(cfg, &key, &value).map_err(Error::Usage)?;
            config::save(path, cfg).map_err(Error::Message)?;
            Ok(())
        }
        ConfigAction::Edit => {
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
            if !path.exists() {
                config::save(path, cfg).map_err(Error::Message)?;
            }
            let status = Command::new(editor)
                .arg(path)
                .status()
                .map_err(|e| Error::Message(e.to_string()))?;
            if status.success() {
                Ok(())
            } else {
                Err(Error::Message(format!("editor exited with {status}")))
            }
        }
    }
}

fn dev_init(
    path: &Path,
    cfg: &mut PlatformConfig,
    override_path: Option<PathBuf>,
    do_link: bool,
) -> Result<(), Error> {
    let ws = if let Some(p) = override_path {
        p
    } else {
        let cwd = std::env::current_dir().map_err(|e| Error::Message(e.to_string()))?;
        resolve::detect(cfg)?
            .workspace
            .unwrap_or(cwd)
    };
    if !ws.join("Cargo.toml").is_file() {
        return Err(Error::Usage(format!(
            "not a workspace (no Cargo.toml): {}",
            ws.display()
        )));
    }
    cfg.workspace = Some(ws.clone());
    config::save(path, cfg).map_err(Error::Message)?;
    eprintln!("Registered workspace {}", ws.display());
    eprintln!("Config {}", path.display());
    if do_link {
        link_cmd(Some(ws.as_path()), cfg)?;
    } else {
        eprintln!("Skipped PATH link (--no-link). Run: vdctl link");
    }
    Ok(())
}

fn link_cmd(workspace: Option<&Path>, cfg: &PlatformConfig) -> Result<(), Error> {
    let ws = workspace
        .map(Path::to_path_buf)
        .or_else(|| cfg.workspace.clone())
        .or_else(|| resolve::detect(cfg).ok().and_then(|p| p.workspace));
    let source = crate::link::resolve_source(ws.as_deref());
    let result = crate::link::link_vdctl(&source)?;
    if result.changed {
        eprintln!("Linked {} → {}", source.display(), result.dest.display());
    } else {
        eprintln!(
            "Already linked {} → {}",
            source.display(),
            result.dest.display()
        );
    }
    let bin_dir = crate::link::user_bin_dir();
    // Never suggest exporting a directory that is already on PATH.
    if crate::link::is_on_path(&bin_dir) {
        eprintln!("PATH already includes {}", bin_dir.display());
    } else {
        eprintln!(
            "Note: {} is not on PATH. Add it once, e.g.:\n  export PATH=\"{}:$PATH\"",
            bin_dir.display(),
            bin_dir.display()
        );
    }
    Ok(())
}

fn shell_cmd(platform: &crate::resolve::Platform) -> Result<(), Error> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
    let status = Command::new(shell)
        .env("VD_HOME", paths::home_dir())
        .env("VD_TRANSPORT", &platform.transport)
        .env("VD_SOCKET", &platform.socket)
        .env("VD_MODELS_DIR", paths::models_dir())
        .env("VDCTL_CONFIG", paths::config_path())
        .status()
        .map_err(|e| Error::Message(e.to_string()))?;
    if status.success() {
        Ok(())
    } else {
        Err(Error::Message(format!("shell exited with {status}")))
    }
}
