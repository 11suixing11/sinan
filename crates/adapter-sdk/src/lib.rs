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

pub trait Privileged: Send + Sync {
    fn execute<'a>(&'a self, program: &'a Path, args: &'a [String])
        -> BoxFuture<'a, CommandOutput>;
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
    fn install_archive<'a>(
        &'a self,
        archive: &'a Path,
        directory: &'a Path,
        binary_name: &'a str,
    ) -> BoxFuture<'a, ()>;
}

pub trait ServiceManager: Send + Sync {
    fn reload<'a>(&'a self, unit: &'a str) -> BoxFuture<'a, ()>;
    fn restart<'a>(&'a self, unit: &'a str) -> BoxFuture<'a, ()>;
    fn stop<'a>(&'a self, unit: &'a str) -> BoxFuture<'a, ()>;
    fn is_active<'a>(&'a self, unit: &'a str) -> BoxFuture<'a, bool>;
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
