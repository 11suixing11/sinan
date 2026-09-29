use anyhow::{bail, Context, Result};
use sinan_adapter_sdk::Counter;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};
use tonic::{client::Grpc, codec::ProstCodec, transport::Endpoint, Request};

#[allow(dead_code)]
mod proto {
    tonic::include_proto!("experimental.v2rayapi");
}

const QUERY_PATH: &str = "/v2ray.core.app.stats.command.StatsService/QueryStats";

pub(crate) async fn query(address: &str) -> Result<Vec<proto::Stat>> {
    let address = crate::native::stats_address(address)?;
    tokio::time::timeout(Duration::from_secs(3), async {
        let channel = Endpoint::from_shared(format!("http://{address}"))?
            .connect_timeout(Duration::from_secs(1))
            .timeout(Duration::from_secs(2))
            .connect()
            .await
            .context("connect to statistics API")?;
        let mut client = Grpc::new(channel).max_decoding_message_size(16 * 1024 * 1024);
        client
            .ready()
            .await
            .context("statistics API is not ready")?;
        let request = Request::new(proto::QueryStatsRequest {
            pattern: String::new(),
            patterns: vec!["user>>>".into()],
            reset: false,
            regexp: false,
        });
        let response: tonic::Response<proto::QueryStatsResponse> = client
            .unary(
                request,
                tonic::codegen::http::uri::PathAndQuery::from_static(QUERY_PATH),
                ProstCodec::default(),
            )
            .await
            .context("query statistics API")?;
        Ok(response.into_inner().stat)
    })
    .await
    .context("statistics request timed out")?
}

pub(crate) fn counters(stats: Vec<proto::Stat>) -> Result<Vec<Counter>> {
    let mut counters = BTreeMap::<String, Counter>::new();
    let mut seen = BTreeSet::new();
    for stat in stats {
        let Some(user) = stat.name.strip_prefix("user>>>") else {
            continue;
        };
        let (name, direction) = user
            .split_once(">>>traffic>>>")
            .context("malformed user counter")?;
        if name.is_empty() || name.contains(">>>") || !matches!(direction, "uplink" | "downlink") {
            bail!("malformed user counter");
        }
        if !seen.insert(stat.name.clone()) {
            bail!("duplicate user counter");
        }
        let value: u64 = stat.value.try_into().context("negative user counter")?;
        let counter = counters.entry(name.into()).or_insert_with(|| Counter {
            stat_name: name.into(),
            ..Counter::default()
        });
        match direction {
            "uplink" => counter.uplink = value,
            "downlink" => counter.downlink = value,
            _ => unreachable!(),
        }
    }
    Ok(counters.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stat(name: &str, value: i64) -> proto::Stat {
        proto::Stat {
            name: name.into(),
            value,
        }
    }

    #[test]
    fn parses_user_totals_without_counting_inbounds_twice() {
        let totals = counters(vec![
            stat("user>>>u2_n3>>>traffic>>>downlink", 15),
            stat("inbound>>>node-3>>>traffic>>>uplink", 100),
            stat("user>>>u1_n3>>>traffic>>>uplink", 5),
            stat("user>>>u1_n3>>>traffic>>>downlink", 10),
        ])
        .unwrap();
        assert_eq!(
            totals,
            vec![
                Counter {
                    stat_name: "u1_n3".into(),
                    uplink: 5,
                    downlink: 10
                },
                Counter {
                    stat_name: "u2_n3".into(),
                    uplink: 0,
                    downlink: 15
                },
            ]
        );
    }

    #[test]
    fn rejects_negative_duplicate_and_malformed_counters() {
        let name = "user>>>u1_n3>>>traffic>>>uplink";
        assert!(counters(vec![stat(name, -1)]).is_err());
        assert!(counters(vec![stat(name, 1), stat(name, 2)]).is_err());
        for name in [
            "user>>>",
            "user>>>>>>traffic>>>uplink",
            "user>>>u1_n3>>>traffic>>>other",
            "user>>>bad>>>name>>>traffic>>>uplink",
        ] {
            assert!(counters(vec![stat(name, 1)]).is_err());
        }
    }
}
