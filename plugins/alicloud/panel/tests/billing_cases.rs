use super::*;
fn bill(page: i64, count: i64, rows: Vec<Value>) -> Reply {
    Reply::ok(
        "QueryInstanceBill",
        json!({"RequestId":"bill-request","Code":"Success","Success":true,"Data":{"BillingCycle":billing::month(sinan_protocol::now_timestamp()),"PageNum":page,"TotalCount":count,"Items":{"Item":rows}}}),
    )
}
fn row(usage: &str, instance: &str) -> Value {
    json!({"InstanceID":instance,"ProductCode":"cdt","ProductType":"cdt_DataTransfer_public_cn","BillingItem":"internet_traffic","Region":"测试地域","Usage":usage,"UsageUnit":"GB","PretaxAmount":"0.00","Currency":"CNY"})
}
fn traffic() -> Reply {
    Reply::ok(
        "ListCdtInternetTraffic",
        json!({"RequestId":"traffic","TrafficDetails":[{"BusinessRegionId":"cn-hangzhou","Traffic":"1073741824"},{"BusinessRegionId":"cn-hongkong","Traffic":2147483648_u64}]}),
    )
}

#[test]
fn compatibility_traffic_validates_every_region_and_never_silently_invents_zero() {
    let value = traffic().value;
    let parsed = billing::traffic(&value, 100).unwrap();
    assert_eq!(parsed.mainland_bytes, "1073741824");
    assert_eq!(parsed.overseas_bytes, "2147483648");
    for value in [
        json!({"TrafficDetails":[]}),
        json!({"TrafficDetails":[{"Traffic":10}]}),
        json!({"TrafficDetails":[{"BusinessRegionId":"cn-hangzhou","Traffic":-1}]}),
        json!({"TrafficDetails":[{"BusinessRegionId":"cn-hangzhou","Traffic":"NaN"}]}),
        json!({"NextToken":"more","TrafficDetails":value["TrafficDetails"]}),
    ] {
        assert!(billing::traffic(&value, 100).is_err());
    }
}
#[sqlx::test]
async fn bill_collects_all_pages_exactly_and_rejects_unknown_units_and_pagination_drift(
    pool: PgPool,
) {
    let (id, _) = seed(&pool, "ecs").await;
    let account = account(&pool, id).await;
    let now = sinan_protocol::now_timestamp();
    let mock = Mock::start(vec![
        bill(1, 2, vec![row("99.9999999", "first")]),
        bill(2, 2, vec![row("0.000001", "second")]),
    ])
    .await;
    let value = Cloud::local(&mock.endpoint)
        .bill(&account, now)
        .await
        .unwrap();
    assert_eq!(value.usage_micro_gb, Some(100_000_000));
    assert_eq!(value.rows.len(), 2);
    assert_eq!(mock.requests()[1].params["PageNum"], "2");
    mock.exhausted();
    let mock = Mock::start(vec![bill(
        1,
        1,
        vec![{
            let mut v = row("900", "x");
            v["UsageUnit"] = "unknown".into();
            v
        }],
    )])
    .await;
    let value = Cloud::local(&mock.endpoint)
        .bill(&account, now)
        .await
        .unwrap();
    assert!(value.usage_micro_gb.is_none());
    mock.exhausted();
    let mock = Mock::start(vec![
        bill(1, 2, vec![row("1", "x")]),
        bill(2, 3, vec![row("2", "y")]),
    ])
    .await;
    assert_eq!(
        Cloud::local(&mock.endpoint)
            .bill(&account, now)
            .await
            .unwrap_err()
            .code,
        "billing_incomplete"
    );
    mock.exhausted();
}
#[sqlx::test]
async fn auto_control_requires_opt_in_fresh_current_bill_and_only_lowers_once_per_configuration(
    pool: PgPool,
) {
    let (account_id, id) = seed(&pool, "ecs").await;
    sqlx::query("UPDATE alicloud_accounts SET auto_enabled=true WHERE id=$1")
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE alicloud_resources SET auto_enabled=true WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let mock = Mock::start(vec![
        bill(1, 1, vec![row("100.000000", "x")]),
        traffic(),
        snapshot("ecs", 10),
    ])
    .await;
    worker::refresh(&pool, account_id, &Cloud::local(&mock.endpoint))
        .await
        .unwrap();
    mock.exhausted();
    let mut current = account(&pool, account_id).await;
    let now = sinan_protocol::now_timestamp();
    assert!(billing::exceeded(&current, now));
    let operation: Uuid =
        sqlx::query_scalar("SELECT id FROM alicloud_operations WHERE source='automatic'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        operations::load(&pool, operation)
            .await
            .unwrap()
            .target
            .bandwidth_mbps,
        1
    );
    current.bill.as_mut().unwrap().queried_at = now - 901;
    assert!(!billing::exceeded(&current, now));
    current.bill.as_mut().unwrap().queried_at = now;
    current.bill.as_mut().unwrap().month = "2000-01".into();
    assert!(!billing::exceeded(&current, now));
    current.bill.as_mut().unwrap().month = billing::month(now);
    current.error_code = Some("network_error".into());
    assert!(!billing::exceeded(&current, now));
    // A changed cloud state prevents automatic execution; no blind repeat in this cycle.
    let mock = Mock::start(vec![snapshot("ecs", 20)]).await;
    operations::process(&pool, operation, &Cloud::local(&mock.endpoint))
        .await
        .unwrap();
    mock.exhausted();
    sqlx::query("UPDATE alicloud_accounts SET next_run_at=0 WHERE id=$1")
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();
    let mock = Mock::start(vec![
        bill(1, 1, vec![row("101", "x")]),
        traffic(),
        snapshot("ecs", 20),
    ])
    .await;
    worker::refresh(&pool, account_id, &Cloud::local(&mock.endpoint))
        .await
        .unwrap();
    mock.exhausted();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM alicloud_operations")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
}
#[sqlx::test]
async fn missing_or_unreadable_bill_preserves_history_and_cannot_trigger_with_stale_counters(
    pool: PgPool,
) {
    let (account_id, id) = seed(&pool, "ecs").await;
    sqlx::query("UPDATE alicloud_accounts SET auto_enabled=true WHERE id=$1")
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE alicloud_resources SET auto_enabled=true WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let mock = Mock::start(vec![bill(1, 0, vec![]), traffic(), snapshot("ecs", 10)]).await;
    worker::refresh(&pool, account_id, &Cloud::local(&mock.endpoint))
        .await
        .unwrap();
    mock.exhausted();
    let current = account(&pool, account_id).await;
    assert!(current.traffic.is_some());
    assert!(!billing::exceeded(
        &current,
        sinan_protocol::now_timestamp()
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM alicloud_operations")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[sqlx::test]
async fn automatic_lowering_verifies_the_result_without_switching_charge_mode(pool: PgPool) {
    let (account_id, id) = seed(&pool, "ecs").await;
    sqlx::query("UPDATE alicloud_accounts SET auto_enabled=true WHERE id=$1")
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE alicloud_resources SET auto_enabled=true WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let mock = Mock::start(vec![
        bill(1, 1, vec![row("101", "x")]),
        traffic(),
        snapshot("ecs", 10),
        snapshot("ecs", 10),
        Reply::ok(
            "ModifyInstanceNetworkSpec",
            json!({"RequestId":"automatic-write"}),
        ),
        snapshot("ecs", 1),
    ])
    .await;
    let cloud = Cloud::local(&mock.endpoint);
    worker::refresh(&pool, account_id, &cloud).await.unwrap();
    let operation: Uuid =
        sqlx::query_scalar("SELECT id FROM alicloud_operations WHERE source='automatic'")
            .fetch_one(&pool)
            .await
            .unwrap();
    operations::process(&pool, operation, &cloud).await.unwrap();
    assert_eq!(
        operations::load(&pool, operation).await.unwrap().status,
        "succeeded"
    );
    let requests = mock.requests();
    let write = requests
        .iter()
        .find(|r| r.action == "ModifyInstanceNetworkSpec")
        .unwrap();
    assert_eq!(write.params["InternetMaxBandwidthOut"], "1");
    assert!(!write.params.contains_key("NetworkChargeType"));
    mock.exhausted();
}
#[sqlx::test]
async fn bill_errors_preserve_old_display_but_revoke_control_authority(pool: PgPool) {
    let (account_id, id) = seed(&pool, "ecs").await;
    sqlx::query("UPDATE alicloud_accounts SET auto_enabled=true WHERE id=$1")
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();
    let mock = Mock::start(vec![
        bill(1, 1, vec![row("101", "x")]),
        traffic(),
        snapshot("ecs", 10),
    ])
    .await;
    worker::refresh(&pool, account_id, &Cloud::local(&mock.endpoint))
        .await
        .unwrap();
    mock.exhausted();
    sqlx::query("UPDATE alicloud_accounts SET next_run_at=0 WHERE id=$1")
        .bind(account_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE alicloud_resources SET auto_enabled=true WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let mock = Mock::start(vec![
        Reply {
            action: "QueryInstanceBill".into(),
            value: json!({"Code":"Throttling"}),
            status: 429,
        },
        traffic(),
        snapshot("ecs", 10),
    ])
    .await;
    worker::refresh(&pool, account_id, &Cloud::local(&mock.endpoint))
        .await
        .unwrap();
    mock.exhausted();
    let current = account(&pool, account_id).await;
    assert_eq!(
        current.bill.as_ref().unwrap().usage_micro_gb,
        Some(101_000_000)
    );
    assert!(!billing::exceeded(
        &current,
        sinan_protocol::now_timestamp()
    ));
    assert_eq!(current.error_code.as_deref(), Some("rate_limited"));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM alicloud_operations")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}
