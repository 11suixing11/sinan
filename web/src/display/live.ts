import type { Server } from '../types'

export type LiveServer = Pick<Server, 'id' | 'online' | 'last_seen' | 'last_heartbeat_at' | 'metrics_stale' | 'metrics_sampled_at' | 'metrics_received_at' | 'metrics_persisted_at' | 'latest_metrics'>
export type LiveSnapshot = { served_at: number; public_view: boolean; servers: LiveServer[] }

export function mergeLiveServers(metadata: Server[], snapshot: LiveSnapshot): Server[] {
  const live = new Map(snapshot.servers.map(server => [server.id, server]))
  return metadata.filter(server => !server.asset_settings?.hidden && live.has(server.id) && Boolean(server.public_view) === snapshot.public_view).map(server => {
    const next = live.get(server.id)!
    const newer = next.metrics_sampled_at !== null && next.metrics_sampled_at >= (server.metrics_sampled_at ?? 0)
    // Only the explicit live projection can update metadata. Never spread a
    // remote record into a public server or replay an older metric sample.
    return { ...server, served_at: snapshot.served_at, online: next.online, last_seen: next.last_seen, last_heartbeat_at: next.last_heartbeat_at, metrics_stale: next.metrics_stale,
      metrics_received_at: next.metrics_received_at, metrics_persisted_at: next.metrics_persisted_at,
      ...(newer ? { metrics_sampled_at: next.metrics_sampled_at, latest_metrics: next.latest_metrics } : {}) }
  })
}
