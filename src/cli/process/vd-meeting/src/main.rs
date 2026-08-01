//! `vd-meeting` CLI — plan a meeting Job and submit to the shared Executor.

fn main() -> std::process::ExitCode {
    vd_meeting::run_cli(std::env::args_os())
}
