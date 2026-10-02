// Explicit public API fixtures. This is test data, not the backend projection implementation.
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
  const common = (kind, item, entry, exit) => ({ kind, id: item.id, name: item.name, entry, exit,
    available: item.available !== false && [entry, ...(exit ? [exit] : [])].every(endpoint => endpoint.enabled && endpoint.plugin_enabled && !endpoint.node_deleted && !endpoint.server_deleted),
    unavailable_reasons: item.unavailable_reasons ?? [], policy_group_ids: item.policy_group_ids ?? [], user_count: item.user_count ?? 0,
    chain_refs: kind === 'direct' ? chains.filter(chain => chain.exit_node_id === item.id).map(chain => ({ id: chain.id, name: chain.name, role: 'exit' })) : [] })
  return [
    ...nodes.filter(node => !node.node_deleted && !servers.find(server => server.id === node.server_id)?.server_deleted && !chains.some(chain => chain.entry_node_id === node.id)).map(node => common('direct', node, endpoint(node.id), null)),
    ...chains.map(chain => common('chain', chain, endpoint(chain.entry_node_id), endpoint(chain.exit_node_id))),
  ]
}
