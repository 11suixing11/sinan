use sinan_protocol::{Metrics, NetworkMetrics, StaticInfo};
use std::{
    collections::{BTreeMap, BTreeSet},
    net::IpAddr,
    time::Instant,
};
use sysinfo::{Disks, Networks, System};

type NetworkTotals = BTreeMap<String, (u64, u64)>;

pub struct Collector {
    system: System,
    disks: Disks,
    networks: Networks,
    last_cpu: Option<Instant>,
    last_network: Option<(Instant, NetworkTotals)>,
}

impl Default for Collector {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector {
    pub fn new() -> Self {
        let mut system = System::new();
        system.refresh_cpu_all();
        system.refresh_memory();
        Self {
            system,
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            last_cpu: None,
            last_network: None,
        }
    }

    pub fn static_info(&self) -> StaticInfo {
        StaticInfo {
            system: System::long_os_version().or_else(System::name),
            kernel: System::kernel_version(),
            arch: Some(std::env::consts::ARCH.into()),
            cpu_model: self
                .system
                .cpus()
                .first()
                .map(|cpu| cpu.brand().to_string())
                .filter(|value| !value.is_empty()),
            cpu_cores: u32::try_from(self.system.cpus().len())
                .ok()
                .filter(|count| *count > 0),
            memory_total: positive(self.system.total_memory()),
            disk_total: disk_totals(&self.disks).map(|(total, _)| total),
            virtualization: virtualization(),
            hostname: System::host_name(),
            agent_version: Some(env!("CARGO_PKG_VERSION").into()),
            ip_addresses: normalized_addresses(
                self.networks
                    .values()
                    .flat_map(|network| network.ip_networks().iter().map(|address| address.addr)),
            ),
            ..StaticInfo::default()
        }
    }

    pub fn metrics(&mut self) -> Metrics {
        let now = Instant::now();
        self.system.refresh_cpu_all();
        self.system.refresh_memory();
        // Fresh inventories do not retain measurements after a failed refresh.
        self.disks = Disks::new_with_refreshed_list();
        self.networks = Networks::new_with_refreshed_list();
        let cpu = self
            .last_cpu
            .filter(|previous| {
                now.duration_since(*previous) >= sysinfo::MINIMUM_CPU_UPDATE_INTERVAL
            })
            .filter(|_| !self.system.cpus().is_empty())
            .map(|_| f64::from(self.system.global_cpu_usage()))
            .filter(|value| value.is_finite());
        self.last_cpu = Some(now);
        let totals: BTreeMap<_, _> = self
            .networks
            .iter()
            .map(|(name, data)| {
                (
                    name.clone(),
                    (data.total_received(), data.total_transmitted()),
                )
            })
            .collect();
        let interfaces = totals
            .iter()
            .map(|(name, &(received, transmitted))| {
                let previous = self
                    .last_network
                    .as_ref()
                    .and_then(|(time, previous)| previous.get(name).map(|values| (*time, *values)));
                let (receive_rate, transmit_rate) =
                    previous.map_or((None, None), |(time, (rx, tx))| {
                        let elapsed = now.duration_since(time).as_secs_f64();
                        if elapsed <= 0.0 {
                            return (None, None);
                        }
                        (
                            received.checked_sub(rx).map(|value| value as f64 / elapsed),
                            transmitted
                                .checked_sub(tx)
                                .map(|value| value as f64 / elapsed),
                        )
                    });
                (
                    name.clone(),
                    NetworkMetrics {
                        received_bytes: Some(received),
                        transmitted_bytes: Some(transmitted),
                        receive_bytes_per_sec: receive_rate,
                        transmit_bytes_per_sec: transmit_rate,
                    },
                )
            })
            .collect();
        self.last_network = Some((now, totals));
        let load = load_average();
        Metrics {
            cpu_percent: cpu,
            memory_used: positive(self.system.total_memory()).map(|_| self.system.used_memory()),
            load_1: load.map(|value| value.0),
            load_5: load.map(|value| value.1),
            load_15: load.map(|value| value.2),
            disk_used: disk_totals(&self.disks).map(|(_, used)| used),
            network_interfaces: interfaces,
            tcp_connections: connection_count("tcp"),
            udp_connections: connection_count("udp"),
            uptime_secs: positive(System::uptime()),
            ..Metrics::default()
        }
    }
}

pub(crate) fn normalized_addresses(addresses: impl IntoIterator<Item = IpAddr>) -> Vec<String> {
    addresses
        .into_iter()
        .map(|address| match address {
            IpAddr::V6(address) => address
                .to_ipv4_mapped()
                .map(IpAddr::V4)
                .unwrap_or(IpAddr::V6(address)),
            address => address,
        })
        .filter(|address| {
            !address.is_unspecified() && !address.is_multicast() && !address.is_loopback()
        })
        .filter(|address| match address {
            IpAddr::V4(address) => !address.is_link_local() && !address.is_broadcast(),
            IpAddr::V6(address) => !address.is_unicast_link_local(),
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|address| address.to_string())
        .collect()
}

fn positive(value: u64) -> Option<u64> {
    (value > 0).then_some(value)
}

fn disk_totals(disks: &Disks) -> Option<(u64, u64)> {
    let mut seen = std::collections::HashSet::new();
    let mut total = 0_u64;
    let mut used = 0_u64;
    for disk in disks.list() {
        if !seen.insert(disk.name()) {
            continue;
        }
        positive(disk.total_space())?;
        total = total.checked_add(disk.total_space())?;
        used = used.checked_add(disk.total_space().checked_sub(disk.available_space())?)?;
    }
    positive(total).map(|total| (total, used))
}

fn load_average() -> Option<(f64, f64, f64)> {
    #[cfg(target_os = "linux")]
    {
        let value = std::fs::read_to_string("/proc/loadavg").ok()?;
        let mut fields = value.split_whitespace();
        Some((
            fields.next()?.parse().ok()?,
            fields.next()?.parse().ok()?,
            fields.next()?.parse().ok()?,
        ))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let load = System::load_average();
        (load.one.is_finite() && load.five.is_finite() && load.fifteen.is_finite()).then_some((
            load.one,
            load.five,
            load.fifteen,
        ))
    }
}

fn connection_count(protocol: &str) -> Option<u64> {
    let ipv4 = std::fs::read_to_string(format!("/proc/net/{protocol}")).ok()?;
    let ipv6 = std::fs::read_to_string(format!("/proc/net/{protocol}6")).ok()?;
    u64::try_from(
        ipv4.lines()
            .skip(1)
            .filter(|line| !line.trim().is_empty())
            .count()
            + ipv6
                .lines()
                .skip(1)
                .filter(|line| !line.trim().is_empty())
                .count(),
    )
    .ok()
}

fn virtualization() -> Option<String> {
    let cgroups = std::fs::read_to_string("/proc/1/cgroup").unwrap_or_default();
    for name in ["docker", "kubepods", "lxc"] {
        if cgroups.contains(name) {
            return Some(name.into());
        }
    }
    let product = std::fs::read_to_string("/sys/class/dmi/id/product_name")
        .ok()?
        .to_lowercase();
    for (pattern, name) in [
        ("kvm", "KVM"),
        ("qemu", "QEMU"),
        ("vmware", "VMware"),
        ("virtualbox", "VirtualBox"),
        ("virtual machine", "Virtual machine"),
    ] {
        if product.contains(pattern) {
            return Some(name.into());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_sample_omits_unavailable_deltas() {
        let mut collector = Collector::new();
        let metrics = collector.metrics();
        assert!(metrics.cpu_percent.is_none());
        assert!(
            metrics
                .network_interfaces
                .values()
                .all(|value| value.receive_bytes_per_sec.is_none()
                    && value.transmit_bytes_per_sec.is_none())
        );
        let encoded = serde_json::to_value(metrics).unwrap();
        assert!(encoded.get("cpu_percent").is_none());
    }

    #[test]
    fn reported_addresses_are_canonical_deduplicated_and_exclude_local_noise() {
        let addresses = normalized_addresses(
            [
                "::1",
                "127.0.0.1",
                "0.0.0.0",
                "224.0.0.1",
                "fe80::1",
                "169.254.0.1",
                "192.0.2.1",
                "192.0.2.1",
                "2001:0db8:0:0:0:0:0:1",
                "10.0.0.2",
            ]
            .map(|value| value.parse().unwrap()),
        );
        assert_eq!(addresses, vec!["10.0.0.2", "192.0.2.1", "2001:db8::1"]);
    }
}
