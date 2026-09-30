use sinan_adapter_sdk::{
    Adapter, BoxFuture, Counter, Descriptor, Plan, Prepared, Privileged, RuntimeSpec,
    ServiceManager, UsageSource,
};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;

#[derive(Default)]
pub struct FakeServiceManager {
    pub active: AtomicBool,
    pub fail_next: AtomicBool,
    pub actions: Mutex<Vec<String>>,
}

impl FakeServiceManager {
    fn action(&self, action: &str, unit: &str, active: bool) -> anyhow::Result<()> {
        self.actions
            .lock()
            .unwrap()
            .push(format!("{action}:{unit}"));
        anyhow::ensure!(
            !self.fail_next.swap(false, Ordering::SeqCst),
            "injected service failure"
        );
        self.active.store(active, Ordering::SeqCst);
        Ok(())
    }
}

impl ServiceManager for FakeServiceManager {
    fn reload<'a>(&'a self, unit: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async move { self.action("reload", unit, true) })
    }
    fn restart<'a>(&'a self, unit: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async move { self.action("restart", unit, true) })
    }
    fn stop<'a>(&'a self, unit: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async move { self.action("stop", unit, false) })
    }
    fn is_active<'a>(&'a self, _unit: &'a str) -> BoxFuture<'a, bool> {
        Box::pin(async move { Ok(self.active.load(Ordering::SeqCst)) })
    }
}

#[derive(Default)]
pub struct FakeAdapter {
    pub counters: Mutex<Vec<Counter>>,
    pub applied: Mutex<Vec<u64>>,
    pub fail_prepare: AtomicBool,
    pub fail_apply: AtomicBool,
    pub fail_health: AtomicBool,
    pub fail_counters: AtomicBool,
    pub apply_delay_ms: AtomicU64,
}

impl Adapter for FakeAdapter {
    fn describe(&self) -> Descriptor {
        Descriptor {
            module: "demo".into(),
            plugin_name: "demo".into(),
            binary_name: "demo".into(),
            auxiliary_files: Vec::new(),
            service_unit: "demo@main".into(),
            service_group: String::new(),
        }
    }
    fn prepare<'a>(
        &'a self,
        runtime: RuntimeSpec,
        _privileged: &'a dyn Privileged,
    ) -> BoxFuture<'a, Prepared> {
        Box::pin(async move {
            anyhow::ensure!(
                !self.fail_prepare.swap(false, Ordering::SeqCst),
                "injected preparation failure"
            );
            Ok(Prepared {
                spec: runtime,
                listen_ports: vec![],
            })
        })
    }
    fn plan<'a>(
        &'a self,
        previous: Option<&'a Prepared>,
        target: &'a Prepared,
    ) -> BoxFuture<'a, Plan> {
        Box::pin(async move {
            Ok(match previous {
                Some(previous) if previous.spec.kernel_version == target.spec.kernel_version => {
                    if previous.spec.config_hash == target.spec.config_hash {
                        Plan::Noop
                    } else {
                        Plan::Reload
                    }
                }
                _ => Plan::Restart,
            })
        })
    }
    fn apply<'a>(
        &'a self,
        plan: Plan,
        target: &'a Prepared,
        services: &'a dyn ServiceManager,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            self.applied.lock().unwrap().push(target.spec.revision);
            tokio::time::sleep(Duration::from_millis(
                self.apply_delay_ms.load(Ordering::SeqCst),
            ))
            .await;
            anyhow::ensure!(
                !self.fail_apply.swap(false, Ordering::SeqCst),
                "injected application failure"
            );
            match plan {
                Plan::Noop => {}
                Plan::Reload => services.reload("demo@main").await?,
                Plan::Restart => services.restart("demo@main").await?,
            }
            if plan != Plan::Noop {
                for counter in self.counters.lock().unwrap().iter_mut() {
                    counter.uplink = 0;
                    counter.downlink = 0;
                }
            }
            Ok(())
        })
    }
    fn health<'a>(
        &'a self,
        _target: &'a Prepared,
        services: &'a dyn ServiceManager,
    ) -> BoxFuture<'a, bool> {
        Box::pin(async move {
            if self.fail_health.swap(false, Ordering::SeqCst) {
                return Ok(false);
            }
            services.is_active("demo@main").await
        })
    }
    fn usage_source(&self) -> Option<&dyn UsageSource> {
        Some(self)
    }
}

impl UsageSource for FakeAdapter {
    fn read_counters<'a>(&'a self, _runtime: &'a Prepared) -> BoxFuture<'a, Vec<Counter>> {
        Box::pin(async move {
            anyhow::ensure!(
                !self.fail_counters.swap(false, Ordering::SeqCst),
                "injected counter failure"
            );
            Ok(self.counters.lock().unwrap().clone())
        })
    }
}
