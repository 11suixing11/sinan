#![forbid(unsafe_code)]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = tonic_build::Config::new();
    if std::env::var_os("PROTOC").is_none() {
        config.protoc_executable(protoc_bin_vendored::protoc_bin_path()?);
    }
    tonic_build::configure()
        .build_client(false)
        .build_server(false)
        .compile_protos_with_config(config, &["proto/1.14/stats.proto"], &["proto/1.14"])?;
    println!("cargo:rerun-if-changed=proto/1.14/stats.proto");
    println!("cargo:rerun-if-env-changed=PROTOC");
    Ok(())
}
