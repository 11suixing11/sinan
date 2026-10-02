#![allow(dead_code)]

use serde_json::{Value, json};

pub fn authorization() -> Value {
    json!({"region":"fixture","source":"TEST_ONLY owned fixture", "scope":"owned",
        "evidence":"TEST_ONLY synthetic permission; fixtures never contact this target", "expires_at":null})
}

pub fn configured(mut spec: Value) -> Value {
    spec["authorization"] = authorization();
    spec
}

pub fn editing(mut spec: Value, revision: i64) -> Value {
    spec["authorization"] = authorization();
    spec["revision"] = json!(revision);
    spec
}
