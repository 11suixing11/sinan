#![forbid(unsafe_code)]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("PROTOC", protoc_bin_vendored::protoc_bin_path()?);
    tonic_build::configure()
        .build_client(false)
        .build_server(false)
        .compile_protos(&["proto/1.14/stats.proto"], &["proto/1.14"])?;
    println!("cargo:rerun-if-changed=proto/1.14/stats.proto");
    Ok(())
}
