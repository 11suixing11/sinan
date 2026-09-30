use crate::IpVersion;
use anyhow::{Result, bail, ensure};
use std::{collections::BTreeSet, path::PathBuf};

pub enum Command {
    Help,
    Version,
    BuildInfo,
    Run(Options),
}

#[derive(Clone, Debug)]
pub struct Options {
    pub workspace: PathBuf,
    pub targets_file: String,
    pub target_digest: String,
    pub ip_version: IpVersion,
    pub count: u8,
    pub concurrency: u8,
}

impl Options {
    pub(crate) fn valid(&self) -> bool {
        self.workspace.is_absolute()
            && self.workspace.parent().is_some()
            && !self.workspace.components().any(|part| {
                matches!(
                    part,
                    std::path::Component::ParentDir | std::path::Component::CurDir
                )
            })
            && self
                .workspace
                .to_str()
                .is_some_and(|value| !value.chars().any(char::is_control))
            && !self.targets_file.is_empty()
            && self.targets_file.len() <= 128
            && !self.targets_file.starts_with('.')
            && self
                .targets_file
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
            && self.target_digest.len() == 64
            && self
                .target_digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            && matches!(self.count, 4 | 8)
            && matches!(self.concurrency, 1 | 2)
    }
}

pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Command> {
    let args: Vec<_> = args.into_iter().collect();
    match args.as_slice() {
        [flag] if flag == "--help" || flag == "-h" => return Ok(Command::Help),
        [flag] if flag == "--version" => return Ok(Command::Version),
        [flag] if flag == "--build-info" => return Ok(Command::BuildInfo),
        _ => (),
    }
    let mut workspace = None;
    let mut targets_file = None;
    let mut target_digest = None;
    let mut ip_version = None;
    let mut count = 4;
    let mut concurrency = 1;
    let mut no_upload = false;
    let mut seen = BTreeSet::new();
    let mut args = args.into_iter();
    while let Some(flag) = args.next() {
        ensure!(seen.insert(flag.clone()), "duplicate option");
        if flag == "--no-rank-upload" {
            no_upload = true;
            continue;
        }
        ensure!(
            matches!(
                flag.as_str(),
                "--workspace"
                    | "--targets"
                    | "--target-digest"
                    | "--ip-version"
                    | "--count"
                    | "--concurrency"
            ),
            "unsupported option"
        );
        let value = args
            .next()
            .ok_or_else(|| anyhow::anyhow!("missing option value"))?;
        match flag.as_str() {
            "--workspace" => workspace = Some(PathBuf::from(value)),
            "--targets" => {
                ensure!(
                    !value.is_empty()
                        && value.len() <= 128
                        && !value.starts_with('.')
                        && value
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte)),
                    "targets must be an ordinary file name in the workspace"
                );
                targets_file = Some(value);
            }
            "--target-digest" => {
                ensure!(
                    value.len() == 64
                        && value
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
                    "target digest must be lowercase SHA256"
                );
                target_digest = Some(value);
            }
            "--ip-version" => {
                ip_version = Some(match value.as_str() {
                    "4" => IpVersion::V4,
                    "6" => IpVersion::V6,
                    _ => bail!("IP version must be 4 or 6"),
                })
            }
            "--count" => {
                count = match value.as_str() {
                    "4" => 4,
                    "8" => 8,
                    _ => bail!("count must be 4 or 8"),
                }
            }
            "--concurrency" => {
                concurrency = match value.as_str() {
                    "1" => 1,
                    "2" => 2,
                    _ => bail!("concurrency must be 1 or 2"),
                }
            }
            _ => unreachable!(),
        }
    }
    ensure!(no_upload, "--no-rank-upload is required");
    let workspace = workspace.ok_or_else(|| anyhow::anyhow!("workspace is required"))?;
    ensure!(
        workspace.is_absolute()
            && workspace.parent().is_some()
            && !workspace.components().any(|part| matches!(
                part,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )),
        "workspace must be an absolute private directory without traversal"
    );
    let options = Options {
        workspace,
        targets_file: targets_file
            .ok_or_else(|| anyhow::anyhow!("targets snapshot is required"))?,
        target_digest: target_digest.ok_or_else(|| anyhow::anyhow!("target digest is required"))?,
        ip_version: ip_version.ok_or_else(|| anyhow::anyhow!("IP version is required"))?,
        count,
        concurrency,
    };
    ensure!(options.valid(), "invalid probe options");
    Ok(Command::Run(options))
}
