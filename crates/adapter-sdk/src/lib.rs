#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = anyhow::Result<T>> + Send + 'a>>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Descriptor {
    pub module: String,
    pub plugin_name: String,
    pub binary_name: String,
    #[serde(default)]
    pub auxiliary_files: Vec<String>,
    pub service_unit: String,
    pub service_group: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeSpec {
    pub revision: u64,
    pub kernel_version: String,
    pub config_hash: String,
    pub binary_path: PathBuf,
    pub revision_dir: PathBuf,
    pub stats_listen: String,
    pub files: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Prepared {
    pub spec: RuntimeSpec,
    pub listen_ports: Vec<u16>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    Noop,
    Reload,
    Restart,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Counter {
    pub stat_name: String,
    pub uplink: u64,
    pub downlink: u64,
}

#[derive(Clone, Debug, Default)]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Clone, Debug, Default)]
pub struct Execution {
    pub output: CommandOutput,
    pub timed_out: bool,
    pub truncated: bool,
}

pub trait ManagedProcess: Send {
    fn id(&self) -> u32;
    fn try_wait(&mut self) -> anyhow::Result<Option<bool>>;
    fn exit_code(&self) -> Option<i32> {
        None
    }
    fn terminate(&mut self) -> BoxFuture<'_, ()>;
}

pub trait Privileged: Send + Sync {
    fn spawn_managed<'a>(
        &'a self,
        _program: &'a Path,
        _args: &'a [String],
    ) -> BoxFuture<'a, Box<dyn ManagedProcess>> {
        Box::pin(async { anyhow::bail!("managed process spawning is not supported") })
    }
    fn execute<'a>(&'a self, program: &'a Path, args: &'a [String])
    -> BoxFuture<'a, CommandOutput>;
    fn execute_bounded<'a>(
        &'a self,
        program: &'a Path,
        args: &'a [String],
        _timeout_secs: u32,
        maximum: usize,
    ) -> BoxFuture<'a, Execution> {
        Box::pin(async move {
            let mut output = self.execute(program, args).await?;
            let truncated = output.stdout.len() > maximum || output.stderr.len() > maximum;
            for value in [&mut output.stdout, &mut output.stderr] {
                let mut limit = value.len().min(maximum);
                while !value.is_char_boundary(limit) {
                    limit -= 1;
                }
                value.truncate(limit);
            }
            Ok(Execution {
                output,
                timed_out: false,
                truncated,
            })
        })
    }
    fn create_dir<'a>(
        &'a self,
        path: &'a Path,
        mode: u32,
        group: Option<&'a str>,
    ) -> BoxFuture<'a, ()>;
    fn write_file<'a>(
        &'a self,
        path: &'a Path,
        bytes: &'a [u8],
        mode: u32,
        group: Option<&'a str>,
    ) -> BoxFuture<'a, ()>;
    fn atomic_symlink<'a>(&'a self, link: &'a Path, target: &'a Path) -> BoxFuture<'a, ()>;
    fn remove_symlink<'a>(&'a self, link: &'a Path) -> BoxFuture<'a, ()>;
    fn remove_file<'a>(&'a self, _path: &'a Path) -> BoxFuture<'a, ()> {
        Box::pin(async { anyhow::bail!("credential removal is not supported") })
    }
    fn remove_managed_directory<'a>(&'a self, _path: &'a Path) -> BoxFuture<'a, ()> {
        Box::pin(async { anyhow::bail!("managed directory removal is not supported") })
    }
    /// Remove an ordinary temporary artifact path after publication or failure.
    fn remove_path<'a>(&'a self, path: &'a Path) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let output = self
                .execute(
                    Path::new("rm"),
                    &[
                        "-rf".into(),
                        "--".into(),
                        path.to_string_lossy().into_owned(),
                    ],
                )
                .await?;
            anyhow::ensure!(
                output.success,
                "temporary artifact cleanup failed: {}",
                output.stderr
            );
            Ok(())
        })
    }
    /// Publish a verified sibling staging directory without replacing a version.
    fn publish_directory<'a>(
        &'a self,
        source: &'a Path,
        destination: &'a Path,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let output = self
                .execute(
                    Path::new("/bin/mv"),
                    &[
                        "--no-clobber".into(),
                        "--no-target-directory".into(),
                        "--".into(),
                        source.to_string_lossy().into_owned(),
                        destination.to_string_lossy().into_owned(),
                    ],
                )
                .await?;
            anyhow::ensure!(
                output.success,
                "artifact publication failed: {}",
                output.stderr
            );
            anyhow::ensure!(
                !source.try_exists()?,
                "artifact version appeared during publication"
            );
            Ok(())
        })
    }
    fn install_archive<'a>(
        &'a self,
        archive: &'a Path,
        directory: &'a Path,
        binary_name: &'a str,
    ) -> BoxFuture<'a, ()>;
    fn install_archive_files<'a>(
        &'a self,
        archive: &'a Path,
        directory: &'a Path,
        binary_name: &'a str,
        auxiliary_files: &'a [String],
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            anyhow::ensure!(
                auxiliary_files.is_empty(),
                "additional artifact files are not supported"
            );
            self.install_archive(archive, directory, binary_name).await
        })
    }
}

pub trait ServiceManager: Send + Sync {
    fn reload<'a>(&'a self, unit: &'a str) -> BoxFuture<'a, ()>;
    fn restart<'a>(&'a self, unit: &'a str) -> BoxFuture<'a, ()>;
    fn stop<'a>(&'a self, unit: &'a str) -> BoxFuture<'a, ()>;
    fn is_active<'a>(&'a self, unit: &'a str) -> BoxFuture<'a, bool>;
    fn start_job<'a>(&'a self, _job: &'a ServiceJob) -> BoxFuture<'a, ()> {
        Box::pin(async { anyhow::bail!("one-shot services are not supported") })
    }
    fn job_status<'a>(&'a self, _unit: &'a str) -> BoxFuture<'a, JobStatus> {
        Box::pin(async { anyhow::bail!("one-shot services are not supported") })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticDescriptor {
    pub plugin_name: String,
    pub binary_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticSpec {
    pub id: String,
    pub version: String,
    pub binary_path: PathBuf,
    pub job_dir: PathBuf,
    pub timeout_secs: u32,
    pub options: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceJob {
    pub unit: String,
    pub program: PathBuf,
    pub args: Vec<String>,
    pub working_directory: PathBuf,
    pub timeout_secs: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Missing,
    Running,
    Succeeded,
    Failed { error: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticOutput {
    pub text: String,
    pub report_url: Option<String>,
}

pub trait DiagnosticAdapter: Send + Sync {
    fn describe(&self) -> DiagnosticDescriptor;
    fn prepare<'a>(
        &'a self,
        spec: &'a DiagnosticSpec,
        privileged: &'a dyn Privileged,
    ) -> BoxFuture<'a, ServiceJob>;
    fn collect<'a>(&'a self, spec: &'a DiagnosticSpec) -> BoxFuture<'a, Option<DiagnosticOutput>>;
}

pub trait UsageSource: Send + Sync {
    fn read_counters<'a>(&'a self, runtime: &'a Prepared) -> BoxFuture<'a, Vec<Counter>>;
}

pub trait Adapter: Send + Sync {
    fn describe(&self) -> Descriptor;
    fn prepare<'a>(
        &'a self,
        runtime: RuntimeSpec,
        privileged: &'a dyn Privileged,
    ) -> BoxFuture<'a, Prepared>;
    fn plan<'a>(
        &'a self,
        previous: Option<&'a Prepared>,
        target: &'a Prepared,
    ) -> BoxFuture<'a, Plan>;
    fn apply<'a>(
        &'a self,
        plan: Plan,
        target: &'a Prepared,
        services: &'a dyn ServiceManager,
    ) -> BoxFuture<'a, ()>;
    fn health<'a>(
        &'a self,
        target: &'a Prepared,
        services: &'a dyn ServiceManager,
    ) -> BoxFuture<'a, bool>;
    fn usage_source(&self) -> Option<&dyn UsageSource> {
        None
    }
}
