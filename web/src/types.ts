export type StaticInfo = {
  system?: string; kernel?: string; arch?: string; cpu_model?: string; cpu_cores?: number;
  memory_total?: number; disk_total?: number; virtualization?: string; hostname?: string;
  agent_version?: string; runtime_version?: string;
  ip_addresses?: string[];
}
export type AgentSettings = { sample_interval_secs: number; upload_interval_secs: number; auto_update: boolean; discover_public_ips: boolean }
export type Metrics = {
  swap_used?: number; swap_total?: number; processes?: number;
  disks?: { name: string; mount_point: string; total_bytes?: number | null; used_bytes?: number | null; read_bytes_per_sec?: number | null; write_bytes_per_sec?: number | null; read_iops?: number | null; write_iops?: number | null; await_ms?: number | null; utilization_percent?: number | null }[];
  gpus?: { model: string; usage_percent?: number | null; memory_used?: number | null; memory_total?: number | null }[];
  cpu_percent?: number; memory_used?: number; load_1?: number; load_5?: number; load_15?: number;
  disk_used?: number; tcp_connections?: number; udp_connections?: number; uptime_secs?: number;
  network_interfaces?: Record<string, { received_bytes?: number; transmitted_bytes?: number; receive_bytes_per_sec?: number; transmit_bytes_per_sec?: number }>;
}
export type Server = { id: number; name: string; device_public_key: string | null; static_info: StaticInfo; last_seen: number | null; latest_metrics: Metrics; manifest_rev: number; online: boolean }
export type Node = { id: number; name: string; server_id: number; protocol: string; port: number; public_host: string; sni: string; public_key: string; short_id: string }
export type User = { id: number; name: string; subscription_token: string; subscription_url: string }
export type Access = { user_id: number; node_id: number; uuid: string; stat_name: string }
export type Enrollment = { token: string; expires_at: number; install_command: string; freebsd_install_command: string; windows_install_command: string }
export type Deployment = { status: { module: string; target_rev: number; applied_rev: number; last_result_rev: number; healthy: boolean; last_error: string | null; updated_at: number } | null; history: { module: string; rev: number; bundle_sha256: string; created_at: number }[] }
export type Usage = { uplink: string; downlink: string; total: string; by_user: { user_id: number; name: string; deleted: boolean; uplink: string; downlink: string }[]; by_node: { node_id: number; name: string; deleted: boolean; uplink: string; downlink: string }[] }
export type Artifact = { name: string; version: string; arch: string; sha256: string; bytes: number }
export type QualityDatabase = { database: string; label: string; status: 'succeeded' | 'failed'; fields: { label: string; value: string | number | boolean }[]; error: string | null }
export type IpQuality = { ip: string; checked_at: number; expires_at: number; status: 'succeeded' | 'partial' | 'failed'; databases: QualityDatabase[] }
export type DiagnosticRecord = { id: string; status: 'queued' | 'running' | 'succeeded' | 'failed'; job: { plugin: string; options: { ip_version: string; network_mode: string; upload_report?: string } }; report: { text: string; report_url?: string } | null; error: string | null; created_at: number; updated_at: number; expires_at: number }
export type NodeQuality = { ip_addresses: string[]; quality: IpQuality[]; plugin_ready: boolean; plugin_reason: string | null; reports: DiagnosticRecord[] }
