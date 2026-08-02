fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Prefer system protoc; fall back to vendored binary for local/CI without brew.
    if std::env::var_os("PROTOC").is_none() {
        if let Ok(protoc) = protoc_bin_vendored::protoc_bin_path() {
            std::env::set_var("PROTOC", protoc);
        }
    }
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["proto/runtime/v1/runtime.proto"], &["proto"])?;
    Ok(())
}
