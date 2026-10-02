use anyhow::{Result, ensure};
use rustls_pki_types::{
    CertificateDer,
    pem::{PemObject, SectionKind},
};
use std::{
    fs::{File, Metadata, OpenOptions},
    io::Read,
    path::{Component, Path},
    sync::Arc,
};

const MAX_BYTES: u64 = 256 * 1024;
const MAX_CERTIFICATES: usize = 32;

fn ordinary_path(path: &Path) -> Result<Metadata> {
    ensure!(
        path.is_absolute(),
        "panel CA file must use an absolute path"
    );
    let mut current = std::path::PathBuf::new();
    for component in path.components() {
        ensure!(
            !matches!(component, Component::ParentDir),
            "panel CA path cannot contain parent components"
        );
        current.push(component.as_os_str());
        let metadata = std::fs::symlink_metadata(&current)
            .map_err(|_| anyhow::anyhow!("panel CA path cannot be read"))?;
        ensure!(
            !metadata.file_type().is_symlink(),
            "panel CA path cannot contain symbolic links"
        );
    }
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|_| anyhow::anyhow!("panel CA file cannot be read"))?;
    ensure!(
        metadata.is_file() && metadata.len() > 0 && metadata.len() <= MAX_BYTES,
        "panel CA file must be a nonempty ordinary file of at most 256 KiB"
    );
    Ok(metadata)
}

fn same_file(first: &Metadata, second: &Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        first.dev() == second.dev()
            && first.ino() == second.ino()
            && first.len() == second.len()
            && first.mtime() == second.mtime()
            && first.mtime_nsec() == second.mtime_nsec()
    }
    #[cfg(not(unix))]
    {
        first.is_file() == second.is_file()
            && first.len() == second.len()
            && first.modified().ok() == second.modified().ok()
            && first.created().ok() == second.created().ok()
    }
}

pub(crate) fn certificates(path: Option<&Path>) -> Result<Vec<CertificateDer<'static>>> {
    let Some(path) = path else {
        return Ok(Vec::new());
    };
    let expected = ordinary_path(path)?;
    let file = open_checked(path, &expected)?;
    let opened = file
        .metadata()
        .map_err(|_| anyhow::anyhow!("panel CA file cannot be inspected"))?;
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| anyhow::anyhow!("panel CA file cannot be read"))?;
    ensure!(
        bytes.len() as u64 <= MAX_BYTES,
        "panel CA file exceeds 256 KiB"
    );
    ensure!(
        same_file(&opened, &ordinary_path(path)?),
        "panel CA file changed while reading"
    );
    // The standard PEM decoder may skip unknown sections. Reject all other
    // section labels before decoding; certificate DER stays with rustls.
    let mut starts = 0;
    let mut ends = 0;
    let mut in_certificate = false;
    for line in bytes.split(|byte| matches!(byte, b'\r' | b'\n')) {
        let line = line.trim_ascii();
        match line {
            b"-----BEGIN CERTIFICATE-----" => {
                ensure!(
                    !in_certificate,
                    "panel CA file has incomplete certificate blocks"
                );
                starts += 1;
                in_certificate = true;
            }
            b"-----END CERTIFICATE-----" => {
                ensure!(
                    in_certificate,
                    "panel CA file has incomplete certificate blocks"
                );
                ends += 1;
                in_certificate = false;
            }
            value if value.starts_with(b"-----") => {
                anyhow::bail!("panel CA file can contain only certificate PEM blocks")
            }
            value if !in_certificate => ensure!(
                value.is_empty() || value.starts_with(b"#"),
                "panel CA file has text outside its certificate PEM blocks"
            ),
            _ => {}
        }
    }
    ensure!(
        (1..=MAX_CERTIFICATES).contains(&starts) && starts == ends,
        "panel CA file must contain between 1 and 32 complete certificates"
    );
    let mut roots = rustls::RootCertStore::empty();
    let mut result = Vec::new();
    for section in <(SectionKind, Vec<u8>)>::pem_slice_iter(&bytes) {
        let (kind, der) = section.map_err(|_| anyhow::anyhow!("panel CA file has invalid PEM"))?;
        ensure!(
            kind == SectionKind::Certificate,
            "panel CA file can contain only certificates"
        );
        let certificate = CertificateDer::from(der);
        roots
            .add(certificate.clone())
            .map_err(|_| anyhow::anyhow!("panel CA file has invalid certificate DER"))?;
        result.push(certificate);
    }
    ensure!(
        result.len() == starts,
        "panel CA file has incomplete certificate blocks"
    );
    Ok(result)
}

fn open_checked(path: &Path, expected: &Metadata) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(path)
        .map_err(|_| anyhow::anyhow!("panel CA file cannot be opened"))?;
    let opened = file
        .metadata()
        .map_err(|_| anyhow::anyhow!("panel CA file cannot be inspected"))?;
    ensure!(
        opened.is_file() && same_file(expected, &opened),
        "panel CA file changed while opening"
    );
    Ok(file)
}

pub(crate) fn client_builder(path: Option<&Path>) -> Result<reqwest::ClientBuilder> {
    let mut builder = reqwest::Client::builder();
    for certificate in certificates(path)? {
        builder = builder.add_root_certificate(
            reqwest::Certificate::from_der(certificate.as_ref())
                .map_err(|_| anyhow::anyhow!("panel CA certificate cannot be loaded"))?,
        );
    }
    Ok(builder)
}

pub(crate) fn websocket_connector(
    path: Option<&Path>,
) -> Result<Option<tokio_tungstenite::Connector>> {
    let Some(path) = path else {
        return Ok(None);
    };
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    for certificate in certificates(Some(path))? {
        roots
            .add(certificate)
            .map_err(|_| anyhow::anyhow!("panel CA certificate cannot be loaded"))?;
    }
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()?
    .with_root_certificates(roots)
    .with_no_client_auth();
    Ok(Some(tokio_tungstenite::Connector::Rustls(Arc::new(config))))
}

#[cfg(all(test, unix))]
pub(crate) mod test_support;
#[cfg(test)]
mod tests;
