use crate::{AppState, error::ApiResult};
use serde::Serialize;
use sha1::Sha1;
use sha2::{Digest, Sha256};
use sinan_protocol::release::RELEASE_SOURCE_REPO;

const BOOTSTRAP: &[u8] = include_bytes!("../../../deploy/bootstrap.sh");

#[derive(Serialize)]
pub struct Installation {
    pub version: String,
    pub tag: String,
    pub bootstrap_url: String,
    pub install_command: String,
}

pub async fn select(
    state: &AppState,
    version: Option<&str>,
    token: &str,
) -> ApiResult<Installation> {
    let (version, tag) = crate::releases::select_agent(state, version).await?;
    let bootstrap_url = bootstrap_url();
    let install_command = command(&tag, &state.config.public_url, token);
    Ok(Installation {
        version,
        tag,
        bootstrap_url,
        install_command,
    })
}

pub fn bootstrap_url() -> String {
    let mut hash = Sha1::new();
    hash.update(format!("blob {}\0", BOOTSTRAP.len()));
    hash.update(BOOTSTRAP);
    format!(
        "https://api.github.com/repos/{RELEASE_SOURCE_REPO}/git/blobs/{:x}",
        hash.finalize()
    )
}

pub fn command(tag: &str, panel: &str, token: &str) -> String {
    let program = concat!(
        "set -eu; umask 077; ",
        "if ! command -v curl >/dev/null; then ",
        "elevate=; if [ \"$(id -u)\" != 0 ]; then ",
        "command -v sudo >/dev/null || { echo '请以 root 执行安装命令' >&2; exit 1; }; ",
        "elevate=sudo; fi; ",
        "if command -v apt-get >/dev/null; then ",
        "$elevate apt-get update; ",
        "$elevate env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends curl ca-certificates; ",
        "elif command -v apk >/dev/null; then $elevate apk add --no-cache curl ca-certificates; ",
        "elif command -v dnf >/dev/null; then $elevate dnf install -y curl ca-certificates; ",
        "elif command -v yum >/dev/null; then $elevate yum install -y curl ca-certificates; ",
        "else echo '无法自动准备 curl，请使用提供系统软件源的 Linux 服务器' >&2; exit 1; fi; fi; ",
        "d=$(mktemp -d); trap 'rm -rf \"$d\"' EXIT; ",
        "curl --fail --silent --show-error --proto '=https' --tlsv1.2 ",
        "--noproxy '*' --connect-timeout 20 --max-time 120 --max-filesize 262144 ",
        "-H 'Accept: application/vnd.github.raw+json' \"$1\" -o \"$d/bootstrap.sh\"; ",
        "printf '%s  %s\\n' \"$2\" \"$d/bootstrap.sh\" | sha256sum -c - >/dev/null; ",
        "/bin/sh \"$d/bootstrap.sh\" --tag \"$3\" --panel \"$4\" --token \"$5\""
    );
    let checksum = format!("{:x}", Sha256::digest(BOOTSTRAP));
    format!(
        "sh -c {} sinan-bootstrap {} {} {} {} {}",
        shell_quote(program),
        shell_quote(&bootstrap_url()),
        shell_quote(&checksum),
        shell_quote(tag),
        shell_quote(panel),
        shell_quote(token),
    )
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::process::Command;

    #[test]
    fn command_pins_the_official_blob_and_installer_bytes() {
        let command = command("agent-v0.3.0", "https://panel.example.com", "fixture-token");
        assert!(command.contains(&bootstrap_url()));
        assert!(command.contains(&format!("{:x}", Sha256::digest(BOOTSTRAP))));
        assert!(command.contains("application/vnd.github.raw+json"));
        assert!(command.contains("sha256sum -c"));
        assert!(command.contains("--noproxy"));
        assert!(!command.contains("/install.sh"));
        assert!(BOOTSTRAP.len() <= 262144);
    }

    #[test]
    #[cfg(unix)]
    fn shell_data_stays_literal_even_with_quotes_and_substitutions() {
        let value = "https://panel.example.com/'$(exit 61)\n";
        let output = Command::new("/bin/sh")
            .arg("-c")
            .arg(format!("printf '%s' {}", shell_quote(value)))
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, value.as_bytes());
        let command = command("agent-v0.3.0", value, "'; exit 62; #");
        assert!(
            Command::new("/bin/sh")
                .args(["-n", "-c", &command])
                .status()
                .unwrap()
                .success()
        );
    }

    #[test]
    #[cfg(unix)]
    fn corrupted_bootstrap_download_is_rejected_before_execution() {
        use std::{fs, os::unix::fs::PermissionsExt};

        let directory =
            std::env::temp_dir().join(format!("sinan-bootstrap-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let marker = directory.join("executed");
        let payload = directory.join("payload");
        fs::write(
            &payload,
            format!(
                "#!/bin/sh\ntouch {}\n",
                shell_quote(marker.to_str().unwrap())
            ),
        )
        .unwrap();
        let curl = directory.join("curl");
        fs::write(
            &curl,
            format!(
                "#!/bin/sh\nfor target do :; done\ncat {} > \"$target\"\n",
                shell_quote(payload.to_str().unwrap())
            ),
        )
        .unwrap();
        fs::set_permissions(&curl, fs::Permissions::from_mode(0o700)).unwrap();
        let output = Command::new("/bin/sh")
            .args([
                "-c",
                &command("agent-v0.3.0", "https://panel.example.com", "fixture"),
            ])
            .env("PATH", format!("{}:/usr/bin:/bin", directory.display()))
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(!marker.exists());
        assert!(String::from_utf8_lossy(&output.stderr).contains("checksum did NOT match"));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn missing_curl_is_prepared_by_the_system_repository_before_hash_verification() {
        use std::{fs, os::unix::fs::PermissionsExt};

        for manager in ["apt-get", "apk", "dnf", "yum"] {
            for uid in ["0", "1000"] {
                let directory = std::env::temp_dir().join(format!(
                    "sinan-bootstrap-prerequisite-{}",
                    uuid::Uuid::new_v4()
                ));
                fs::create_dir_all(&directory).unwrap();
                let marker = directory.join("executed");
                let payload = directory.join("payload");
                fs::write(
                    &payload,
                    format!(
                        "#!/bin/sh\ntouch {}\n",
                        shell_quote(marker.to_str().unwrap())
                    ),
                )
                .unwrap();
                let curl_seed = directory.join("curl-seed");
                let package_log = directory.join("packages");
                let sudo_log = directory.join("elevation");
                let scripts = [
                    (
                        curl_seed.clone(),
                        format!(
                            "#!/bin/sh\nfor target do :; done\n/bin/cat {} > \"$target\"\n",
                            shell_quote(payload.to_str().unwrap())
                        ),
                    ),
                    (
                        directory.join("id"),
                        format!("#!/bin/sh\nprintf '%s\\n' {uid}\n"),
                    ),
                    (
                        directory.join(manager),
                        format!(
                            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> {}\nif [ \"$1\" != update ]; then /bin/ln -s {} {}; fi\n",
                            shell_quote(package_log.to_str().unwrap()),
                            shell_quote(curl_seed.to_str().unwrap()),
                            shell_quote(directory.join("curl").to_str().unwrap())
                        ),
                    ),
                    (
                        directory.join("sudo"),
                        format!(
                            "#!/bin/sh\nprintf '%s\\n' invoked >> {}\nexec \"$@\"\n",
                            shell_quote(sudo_log.to_str().unwrap())
                        ),
                    ),
                ];
                for (path, content) in scripts {
                    fs::write(&path, content).unwrap();
                    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
                }
                for tool in ["sh", "env", "mktemp", "rm", "sha256sum"] {
                    std::os::unix::fs::symlink(format!("/usr/bin/{tool}"), directory.join(tool))
                        .unwrap();
                }
                let output = Command::new("/bin/sh")
                    .args([
                        "-c",
                        &command("agent-v0.3.0", "https://panel.example.com", "fixture"),
                    ])
                    .env("PATH", &directory)
                    .output()
                    .unwrap();
                assert!(!output.status.success(), "{manager} as {uid}");
                assert!(!marker.exists(), "{manager} as {uid}");
                let packages = fs::read_to_string(package_log).unwrap();
                assert!(packages.contains("curl ca-certificates"), "{packages}");
                assert_eq!(sudo_log.exists(), uid != "0");
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains("checksum did NOT match"),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
                fs::remove_dir_all(directory).unwrap();
            }
        }
    }
}
