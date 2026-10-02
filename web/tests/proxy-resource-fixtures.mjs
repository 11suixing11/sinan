import { createHash } from 'node:crypto'
// Explicit public API fixtures. This is test data, not the backend projection implementation.
export const pathFixtureUuid = value => `00000000-0000-4000-8000-${String(value).padStart(12, '0')}`
export function legacyPathFields(exit) {
  const hops = [{ kind: 'managed', position: 1, node_id: exit.id, endpoint_version_id: pathFixtureUuid(exit.id), endpoint: exit }]
  return { settings_revision: 1, path_kind: 'legacy', hops, path_state: { desired_generation: 1, candidate_generation: null, applied_generation: 1, recovery_generation: null, minimum_generation: 0, phase: 'legacy', capabilities: { tcp: true, udp: true }, last_error: null, dependencies: [], probe: null, generations: [{ generation: 1, state: 'desired', hops }, { generation: 1, state: 'applied', hops }] } }
}
export function publicResourceFields(kind, exit) { return kind === 'chain' ? legacyPathFields(exit) : { settings_revision: 1, path_kind: null, hops: [], path_state: null } }
export function orderedResourceFixture({ id, name, entry, hops, path_state, ...changes }) {
  if (!hops?.length) throw new Error('An ordered fixture requires an explicit immutable hop vector')
  const frozen = structuredClone(hops)
  return { kind: 'chain', id, name, entry, exit: frozen[frozen.length - 1].kind === 'managed' ? structuredClone(frozen[frozen.length - 1].endpoint) : null, available: true, unavailable_reasons: [], policy_group_ids: [], user_count: 0, chain_refs: [], settings_revision: 1, path_kind: 'ordered', hops: frozen,
    path_state: path_state ?? { desired_generation: 1, candidate_generation: 1, applied_generation: null, recovery_generation: null, minimum_generation: 0, phase: 'preparing_dependencies', capabilities: { tcp: true, udp: false }, last_error: null, dependencies: [], probe: null, generations: [{ generation: 1, state: 'desired', hops: frozen }, { generation: 1, state: 'candidate', hops: frozen }] }, ...changes }
}
export function proxyResourceFixtures(nodes, servers, chains = []) {
  const endpoint = id => {
    const node = nodes.find(node => node.id === id)
    if (!node) throw new Error(`Fixture endpoint #${id} is missing`)
    const server = servers.find(server => server.id === node.server_id)
    if (!server) throw new Error(`Fixture server #${node.server_id} is missing`)
    return { id: node.id, name: node.name, server_id: node.server_id, server_name: server.name, protocol: node.protocol,
      port: node.port, public_port: node.settings?.public_port ?? node.port, public_host: node.public_host, sni: node.sni,
      enabled: node.enabled !== false, node_deleted: node.node_deleted === true, server_deleted: server.server_deleted === true,
      plugin_enabled: server.enabled, online: server.online, desired_revision: server.installation?.target_rev ?? null,
      applied_revision: server.installation?.applied_rev ?? null, applied_observed_at: server.installation ? 1000 : null }
  }
  const common = (kind, item, entry, exit) => ({ kind, id: item.id, name: item.name, entry, exit, ...publicResourceFields(kind, exit),
    available: item.available !== false && [entry, ...(exit ? [exit] : [])].every(endpoint => endpoint.enabled && endpoint.plugin_enabled && !endpoint.node_deleted && !endpoint.server_deleted),
    unavailable_reasons: item.unavailable_reasons ?? [], policy_group_ids: item.policy_group_ids ?? [], user_count: item.user_count ?? 0,
    chain_refs: kind === 'direct' ? chains.filter(chain => chain.exit_node_id === item.id).map(chain => ({ id: chain.id, name: chain.name, role: 'exit', generation: 1, hop_position: 1, state: 'applied' })) : [] })
  return [
    ...nodes.filter(node => !node.node_deleted && !servers.find(server => server.id === node.server_id)?.server_deleted && !chains.some(chain => chain.entry_node_id === node.id)).map(node => common('direct', node, endpoint(node.id), null)),
    ...chains.map(chain => {
      const value = common('chain', chain, endpoint(chain.entry_node_id), endpoint(chain.exit_node_id))
      return chain.path_kind === 'ordered' ? { ...value, ...orderedResourceFixture({ id: value.id, name: value.name, entry: value.entry, hops: value.hops, available: value.available, unavailable_reasons: value.unavailable_reasons, policy_group_ids: value.policy_group_ids, user_count: value.user_count }) } : value
    }),
  ]
}

// The existing flat API remains a separate numeric contract in combined-page tests.
export function flatResourceFixtures(nodes, servers, chains = []) {
  return proxyResourceFixtures(nodes, servers, chains).filter(resource => resource.kind !== 'chain' || resource.path_kind !== 'ordered').map(resource => ({
    kind: resource.kind, id: resource.id, name: resource.name, server_id: resource.entry.server_id, server_name: resource.entry.server_name,
    public_host: resource.entry.public_host, port: resource.entry.port, protocol: resource.entry.protocol, enabled: resource.entry.enabled, available: resource.available,
    role: resource.kind === 'chain' ? 'chain_entry' : resource.chain_refs.length ? 'managed_hop' : 'direct', entry_node_id: resource.kind === 'chain' ? resource.entry.id : null,
    tcp: true, udp: true, legacy: resource.kind === 'chain', active_generation: resource.kind === 'chain' ? 1 : null, pending_generation: null,
    minimum_generation: 0, stage: resource.kind === 'chain' ? 'active' : 'direct', last_error: null,
    reference_count: resource.policy_group_ids.length + resource.user_count + resource.chain_refs.length,
    entry_eligible: resource.kind === 'direct' && resource.entry.protocol === 'vless-reality' && resource.available && !resource.chain_refs.length && !resource.policy_group_ids.length && !resource.user_count,
    managed_server_ids: [...new Set([resource.entry.server_id, ...resource.hops.filter(hop => hop.kind === 'managed').map(hop => hop.endpoint.server_id)])],
    managed_middle_server_ids: resource.hops.filter(hop => hop.kind === 'managed' && hop.position < resource.hops.length).map(hop => hop.endpoint.server_id),
    managed_exit_server_ids: resource.exit ? [resource.exit.server_id] : [],
  }))
}

// Public synthetic catalog DTOs; these do not implement backend/private token generation.
export function catalogResourceFixtures(resources) {
  return resources.map(resource => ({ ...resource, original_name: resource.name, tags: [], note: '', sort_order: resource.id, revision: createHash('sha256').update(JSON.stringify(resource)).digest('hex'), metadata_revision: 0 }))
}
