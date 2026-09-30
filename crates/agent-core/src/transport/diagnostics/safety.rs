use super::*;

pub(super) const START_MEMORY_RESERVE: u64 = 256 * 1024 * 1024;
pub(super) const RUN_MEMORY_RESERVE: u64 = 128 * 1024 * 1024;
const MIN_DISK_AVAILABLE: u64 = 2 * 1024 * 1024 * 1024;
const MAX_LOAD_PER_CPU: f64 = 1.5;
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

impl DiagnosticWorker {
    pub(super) async fn preflight(&self, service: &ServiceJob) -> Result<()> {
        let resources = tokio::time::timeout(
            PROBE_TIMEOUT,
            self.privileged
                .diagnostic_resources(&service.working_directory),
        )
        .await
        .context("资源预检超时，请检查系统资源读取权限后重试")?
        .context("无法读取资源状态，请检查 /proc、cgroup 和工作目录权限后重试")?;
        let available = resources.memory.available_bytes();
        let required = service
            .memory_max
            .get()
            .checked_add(START_MEMORY_RESERVE)
            .context("诊断内存预算过大，请降低内存上限后重试")?;
        ensure!(
            available >= required,
            "可用内存不足：有效可用 {} MiB，诊断预算 {} MiB，另需预留 256 MiB；请停止其他任务或增加内存后重试",
            available / 1024 / 1024,
            service.memory_max.get() / 1024 / 1024
        );
        ensure!(
            resources.disk_available_bytes >= MIN_DISK_AVAILABLE,
            "工作目录磁盘空间不足：可用 {} MiB，至少需要 2048 MiB；请清理磁盘或更换工作目录后重试",
            resources.disk_available_bytes / 1024 / 1024
        );
        ensure!(
            resources.cpu_count > 0 && resources.load_one.is_finite() && resources.load_one >= 0.0,
            "无法读取有效的 CPU 负载，请检查系统指标后重试"
        );
        let max_load = f64::from(resources.cpu_count) * MAX_LOAD_PER_CPU;
        ensure!(
            resources.load_one <= max_load,
            "当前负载过高：一分钟负载 {:.2}，{} 个可用 CPU 的上限为 {:.2}；请等待其他任务结束后重试",
            resources.load_one,
            resources.cpu_count,
            max_load
        );
        let units = tokio::time::timeout(PROBE_TIMEOUT, self.services.running_diagnostic_units())
            .await
            .context("同机诊断检查超时，请检查 systemd 后重试")?
            .context("无法检查同机诊断任务，请检查 systemd 服务查询权限后重试")?;
        let conflicts: Vec<_> = units.iter().filter(|unit| *unit != &service.unit).collect();
        ensure!(
            conflicts.is_empty(),
            "同机已有诊断任务正在运行：{}；请等待或停止原任务后重试",
            conflicts
                .iter()
                .map(|unit| unit.as_str())
                .collect::<Vec<_>>()
                .join("、")
        );
        Ok(())
    }

    pub(super) async fn memory_stop_reason(&self) -> Option<String> {
        match tokio::time::timeout(PROBE_TIMEOUT, self.privileged.diagnostic_memory()).await {
            Ok(Ok(memory)) if memory.available_bytes() >= RUN_MEMORY_RESERVE => None,
            Ok(Ok(memory)) => Some(format!(
                "诊断因低内存保护停止：有效可用 {} MiB，低于 128 MiB 保留阈值；请释放内存后重试",
                memory.available_bytes() / 1024 / 1024
            )),
            Ok(Err(error)) => Some(format!(
                "诊断因无法确认可用内存而保护停止，请检查 /proc 和 cgroup 权限后重试：{error}"
            )),
            Err(_) => Some("诊断因可用内存读取超时而保护停止，请检查系统负载后重试".into()),
        }
    }
}
