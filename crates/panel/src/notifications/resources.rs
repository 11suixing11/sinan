use super::rules::{Aggregation, Metric, Spec};
use crate::servers::Server;
use serde_json::Value;

pub(super) struct Sample {
    bucket: i64,
    sampled_at: i64,
    values: [Option<f64>; 5],
}

const METRICS: [Metric; 5] = [
    Metric::Cpu,
    Metric::Memory,
    Metric::Disk,
    Metric::NetIn,
    Metric::NetOut,
];

impl Sample {
    pub(super) fn from_metrics(
        bucket: i64,
        sampled_at: i64,
        metrics: &Value,
        server: &Server,
    ) -> Self {
        Self {
            bucket,
            sampled_at,
            values: METRICS.map(|metric| value(metric, metrics, server)),
        }
    }
}

fn value(metric: Metric, sample: &Value, server: &Server) -> Option<f64> {
    let finite = |value: f64| value.is_finite() && value >= 0.0;
    match metric {
        Metric::Cpu => sample
            .get("cpu_percent")?
            .as_f64()
            .filter(|v| finite(*v) && *v <= 100.0),
        Metric::Memory | Metric::Disk => {
            let (used, total) = if metric == Metric::Memory {
                ("memory_used", "memory_total")
            } else {
                ("disk_used", "disk_total")
            };
            let used = sample.get(used)?.as_u64()?;
            let total = sample
                .get(total)
                .filter(|value| !value.is_null())
                .or_else(|| server.static_info.get(total))?
                .as_u64()?;
            (total > 0 && used <= total).then(|| used as f64 / total as f64 * 100.0)
        }
        Metric::NetIn | Metric::NetOut => {
            let field = if metric == Metric::NetIn {
                "receive_bytes_per_sec"
            } else {
                "transmit_bytes_per_sec"
            };
            let mut count = 0;
            let mut total = 0.0;
            for (name, metrics) in sample.get("network_interfaces")?.as_object()? {
                if !server.asset_settings.includes(name) {
                    continue;
                }
                let rate = metrics.get(field)?.as_f64().filter(|v| finite(*v))?;
                count += 1;
                total += rate;
            }
            (count > 0 && total.is_finite()).then_some(total / 1_048_576.0)
        }
    }
}

// Use one observation per completed minute, requiring a contiguous, complete window.
pub(super) fn evaluate(spec: &Spec, samples: &[Sample], now: i64) -> Option<(bool, f64)> {
    let end = now / 60 * 60;
    let start = end - i64::from(spec.duration_minutes) * 60;
    let mut next = start;
    let mut sum = 0.0;
    let mut minimum = f64::INFINITY;
    let index = METRICS.iter().position(|metric| *metric == spec.metric)?;
    for sample in samples
        .iter()
        .filter(|sample| sample.bucket >= start && sample.bucket < end)
    {
        if sample.bucket != next
            || sample.sampled_at < sample.bucket * 1000
            || sample.sampled_at >= (sample.bucket + 60) * 1000
        {
            return None;
        }
        let value = sample.values[index]?;
        sum += value;
        minimum = minimum.min(value);
        next += 60;
    }
    if next != end {
        return None;
    }
    let value = if spec.aggregation == Aggregation::Continuous {
        minimum
    } else {
        sum / f64::from(spec.duration_minutes)
    };
    Some((value >= spec.threshold, value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server_assets::AssetSettings;
    use serde_json::json;

    fn server() -> Server {
        Server {
            id: 1,
            name: "fixture".into(),
            device_public_key: None,
            static_info: json!({"memory_total":1000,"disk_total":2000}),
            last_seen: Some(600),
            last_heartbeat_at: None,
            metrics_sampled_at: None,
            agent_settings: json!({}),
            asset_settings: AssetSettings {
                network_interface: "eth*,!eth1".into(),
                ..Default::default()
            },
            traffic: None,
            latest_metrics: json!({}),
            manifest_rev: 0,
            capabilities: json!([]),
            online: true,
            metrics_stale: false,
        }
    }

    #[test]
    fn metric_values_use_valid_capacities_and_only_selected_interface_rates() {
        let server = server();
        let mut spec = Spec {
            name: "fixture".into(),
            metric: Metric::Memory,
            threshold: 90.0,
            duration_minutes: 1,
            aggregation: Aggregation::Average,
            all_servers: true,
            enabled: true,
            server_ids: vec![],
        };
        let metrics = json!({"memory_used":500,"disk_used":1500,"memory_total":null,"disk_total":null,"network_interfaces":{
            "eth0":{"receive_bytes_per_sec":1048576,"transmit_bytes_per_sec":2097152},
            "eth1":{"receive_bytes_per_sec":999999999},"lo":{}}});
        assert_eq!(value(spec.metric, &metrics, &server), Some(50.0));
        spec.metric = Metric::Disk;
        assert_eq!(value(spec.metric, &metrics, &server), Some(75.0));
        assert_eq!(
            value(spec.metric, &json!({"disk_used":2001}), &server),
            None
        );
        spec.metric = Metric::NetIn;
        assert_eq!(value(spec.metric, &metrics, &server), Some(1.0));
        spec.metric = Metric::NetOut;
        assert_eq!(value(spec.metric, &metrics, &server), Some(2.0));
        assert_eq!(
            value(
                spec.metric,
                &json!({"network_interfaces":{"eth0":{"transmitted_bytes":9999}}}),
                &server
            ),
            None
        );
        assert_eq!(
            value(
                spec.metric,
                &json!({"network_interfaces":{"eth0":{"transmit_bytes_per_sec":-1}}}),
                &server
            ),
            None
        );
        assert_eq!(
            value(spec.metric, &json!({"network_interfaces":{}}), &server),
            None
        );
    }

    #[test]
    fn continuous_windows_exclude_partial_minutes_and_future_or_invalid_samples() {
        let server = server();
        let spec = Spec {
            name: "fixture".into(),
            metric: Metric::Cpu,
            threshold: 90.0,
            duration_minutes: 1,
            aggregation: Aggregation::Continuous,
            all_servers: true,
            enabled: true,
            server_ids: vec![],
        };
        let samples = vec![
            (540, 541000, json!({"cpu_percent":95})),
            (600, 601000, json!({"cpu_percent":0})),
        ];
        let samples: Vec<_> = samples
            .into_iter()
            .map(|(bucket, at, metrics)| Sample::from_metrics(bucket, at, &metrics, &server))
            .collect();
        assert_eq!(evaluate(&spec, &samples, 615), Some((true, 95.0)));
        for point in [
            (540, 601000, json!({"cpu_percent":100})),
            (540, 541000, json!({"cpu_percent":101})),
            (480, 481000, json!({"cpu_percent":100})),
        ] {
            assert_eq!(
                evaluate(
                    &spec,
                    &[Sample::from_metrics(point.0, point.1, &point.2, &server)],
                    615
                ),
                None
            );
        }
    }
}
