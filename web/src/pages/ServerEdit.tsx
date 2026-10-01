import { useState } from 'react'
import { api } from '../api'
import { ErrorNotice, Field, Modal } from '../components'
import { useAction } from '../hooks'
import { assetDraft, assetPayload } from '../server-assets'
import type { Server } from '../types'
import ServerAssetFields, { SetupNavigation } from './ServerAssetFields'
import './server-setup.css'

export default function ServerEdit({ server, onClose, onSaved }: { server: Server; onClose: () => void; onSaved: () => void }) {
  const [name, setName] = useState(server.name)
  const [asset, setAsset] = useState(() => assetDraft(server.asset_settings))
  const action = useAction()
  return <Modal title="编辑服务器" onClose={onClose} busy={action.busy} className="server-setup-modal"><form onSubmit={event => { event.preventDefault(); void action.run(() => api(`/api/servers/${server.id}`, 'PATCH', { name: name.trim(), asset_settings: assetPayload(asset) }), onSaved) }}>
    <SetupNavigation />
    <div className="server-setup-body"><fieldset disabled={action.busy}>
      <section className="server-setup-section" id="setup-basics"><Field label="服务器名称"><input required pattern=".*\S.*" maxLength={128} value={name} onChange={event => setName(event.target.value)} /></Field></section>
      <ServerAssetFields value={asset} onChange={setAsset} />
    </fieldset><ErrorNotice message={action.error} /></div>
    <footer className="server-setup-footer"><span>采样与拨测可在服务器详情中调整</span><button type="button" className="button button-secondary" disabled={action.busy} onClick={onClose}>取消</button><button className="button button-primary" disabled={action.busy}>{action.busy ? '正在保存…' : '保存修改'}</button></footer>
  </form></Modal>
}
