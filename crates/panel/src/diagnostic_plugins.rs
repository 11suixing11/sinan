use crate::diagnostics::service::DiagnosticPlugin;
#[path = "../../../plugins/ipquality/panel/mod.rs"]
pub mod ipquality;
#[path = "../../../plugins/nodequality/panel/mod.rs"]
pub mod nodequality;
#[path = "../../../plugins/tcpquality/panel/mod.rs"]
pub mod tcpquality;
static NODEQUALITY: nodequality::NodeQualityPlugin = nodequality::NodeQualityPlugin;
static TCPQUALITY: tcpquality::TcpQualityPlugin = tcpquality::TcpQualityPlugin;
static IPQUALITY: ipquality::IpQualityPlugin = ipquality::IpQualityPlugin;
static REGISTERED: [&dyn DiagnosticPlugin; 3] = [&NODEQUALITY, &TCPQUALITY, &IPQUALITY];
pub fn all() -> &'static [&'static dyn DiagnosticPlugin] {
    &REGISTERED
}
pub fn find(id: &str) -> Option<&'static dyn DiagnosticPlugin> {
    all().iter().copied().find(|plugin| plugin.id() == id)
}

/// Jobs created before plugin registration belong to the original diagnostic plugin.
pub fn for_job(job: &serde_json::Value) -> Option<&'static dyn DiagnosticPlugin> {
    find(job["plugin"].as_str().unwrap_or("nodequality"))
}
