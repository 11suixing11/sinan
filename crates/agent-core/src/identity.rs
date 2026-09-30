use crate::{config::validate_panel_url, telemetry::Collector, Config};
use anyhow::Context;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::SigningKey;
use sinan_protocol::{EnrollRequest, EnrollResponse};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    time::Duration,
};
use uuid::Uuid;

pub struct Identity {
    pub server_id: i64,
    pub signing_key: SigningKey,
}

pub fn load(config: &Config) -> anyhow::Result<Identity> {
    config.validate()?;
    check_origin(config)?;
    let signing_key = read_key(&config.identity_dir.join("device.key"))?;
    let server_id = fs::read_to_string(config.identity_dir.join("server_id"))?
        .trim()
        .parse()?;
    anyhow::ensure!(server_id > 0, "invalid enrolled server identity");
    Ok(Identity {
        server_id,
        signing_key,
    })
}

pub async fn enroll(config: &Config, token: &str) -> anyhow::Result<i64> {
    config.validate()?;
    anyhow::ensure!(
        !token.is_empty() && token.len() <= 512,
        "invalid enrollment token"
    );
    prepare_directory(&config.identity_dir)?;
    let origin_path = config.identity_dir.join("panel_origin");
    let origin = validate_panel_url(&config.panel_url)?
        .origin()
        .ascii_serialization();
    if origin_path.exists() {
        check_origin(config)?;
    } else {
        write_once(&origin_path, origin.as_bytes())?;
        check_origin(config)?;
    }
    let key_path = config.identity_dir.join("device.key");
    if !key_path.exists() {
        let key = SigningKey::generate(&mut rand::rngs::OsRng);
        write_once(&key_path, &key.to_bytes())?;
    }
    let signing_key = read_key(&key_path)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(config.operation_timeout_secs))
        .build()?;
    let request = EnrollRequest {
        token: token.to_owned(),
        device_public_key: URL_SAFE_NO_PAD.encode(signing_key.verifying_key().as_bytes()),
        static_info: Collector::new().static_info(),
    };
    let response = client
        .post(validate_panel_url(&config.panel_url)?.join("api/agent/v1/enroll")?)
        .json(&request)
        .send()
        .await
        .context("enrollment request failed")?;
    anyhow::ensure!(
        response.status().is_success(),
        "panel rejected enrollment with status {}",
        response.status()
    );
    let response: EnrollResponse = response.json().await?;
    anyhow::ensure!(response.server_id > 0, "invalid enrolled server identity");
    let id_path = config.identity_dir.join("server_id");
    if id_path.exists() {
        let previous: i64 = fs::read_to_string(&id_path)?.trim().parse()?;
        anyhow::ensure!(
            previous == response.server_id,
            "enrollment changed an existing server identity"
        );
    }
    atomic_write(&id_path, response.server_id.to_string().as_bytes())?;
    Ok(response.server_id)
}

fn check_origin(config: &Config) -> anyhow::Result<()> {
    let expected = validate_panel_url(&config.panel_url)?
        .origin()
        .ascii_serialization();
    let recorded = fs::read_to_string(config.identity_dir.join("panel_origin"))
        .context("identity has no recorded panel origin; enroll before running")?;
    anyhow::ensure!(
        recorded.trim() == expected,
        "identity belongs to a different panel origin"
    );
    Ok(())
}

fn read_key(path: &Path) -> anyhow::Result<SigningKey> {
    let metadata =
        fs::symlink_metadata(path).context("device key is missing; enroll before running")?;
    anyhow::ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "device key must be a regular file"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        anyhow::ensure!(
            metadata.permissions().mode() & 0o077 == 0,
            "device key must only be accessible by its owner"
        );
    }
    let bytes: [u8; 32] = fs::read(path)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid device key length"))?;
    Ok(SigningKey::from_bytes(&bytes))
}

fn prepare_directory(path: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(path)?;
        let metadata = fs::symlink_metadata(path)?;
        anyhow::ensure!(
            metadata.is_dir() && !metadata.file_type().is_symlink(),
            "identity directory must be an ordinary directory"
        );
        anyhow::ensure!(
            metadata.permissions().mode() & 0o022 == 0,
            "identity directory must not be writable by group or others"
        );
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        anyhow::bail!("private identity permissions require a Unix platform");
    }
    Ok(())
}

fn staged_file(path: &Path, bytes: &[u8]) -> anyhow::Result<std::path::PathBuf> {
    let parent = path.parent().context("identity file has no parent")?;
    let temporary = parent.join(format!(".pending-{}", Uuid::new_v4()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(temporary)
}

fn write_once(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let temporary = staged_file(path, bytes)?;
    let result = fs::hard_link(&temporary, path);
    let _ = fs::remove_file(&temporary);
    match result {
        Ok(()) => {
            fs::File::open(path.parent().context("identity file has no parent")?)?.sync_all()?;
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let temporary = staged_file(path, bytes)?;
    let result = fs::rename(&temporary, path);
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result?;
    fs::File::open(path.parent().context("identity file has no parent")?)?.sync_all()?;
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};

    #[test]
    fn identity_directory_preserves_existing_modes_and_rejects_unsafe_locations(
    ) -> anyhow::Result<()> {
        let directory = std::env::temp_dir().join(format!("sn-identity-mode-{}", Uuid::new_v4()));
        fs::create_dir(&directory)?;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o755))?;
        prepare_directory(&directory)?;
        assert_eq!(
            fs::metadata(&directory)?.permissions().mode() & 0o777,
            0o755
        );
        let private = directory.join("new/identity");
        prepare_directory(&private)?;
        assert_eq!(fs::metadata(&private)?.permissions().mode() & 0o777, 0o700);
        assert_eq!(
            fs::metadata(private.parent().unwrap())?
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        let alias = directory.join("alias");
        symlink(&private, &alias)?;
        assert!(prepare_directory(&alias).is_err());
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o777))?;
        assert!(prepare_directory(&directory).is_err());
        assert_eq!(
            fs::metadata(&directory)?.permissions().mode() & 0o777,
            0o777
        );
        fs::remove_dir_all(directory)?;
        Ok(())
    }
}
