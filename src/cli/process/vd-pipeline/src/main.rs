//! `vd-pipeline` CLI — execute a Job.

fn main() -> std::process::ExitCode {
    vd_pipeline::run(std::env::args_os())
}
