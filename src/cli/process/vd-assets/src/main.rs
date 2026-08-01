//! `vd-assets` CLI — Office/PDF → Markdown + shared dictionary.

fn main() -> std::process::ExitCode {
    vd_assets::run(std::env::args_os())
}
