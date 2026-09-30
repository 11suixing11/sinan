import { useState } from 'react'
import { CopyField, Field } from '../components'
import type { Enrollment } from '../types'

export default function InstallCommand({ enrollment }: { enrollment: Enrollment }) {
  const [platform, setPlatform] = useState('unix')
  const command = platform === 'windows' ? enrollment.windows_install_command : platform === 'freebsd' ? enrollment.freebsd_install_command : enrollment.install_command
  return <><Field label="服务器系统"><select value={platform} onChange={event => setPlatform(event.target.value)}><option value="unix">Linux / macOS</option><option value="freebsd">FreeBSD</option><option value="windows">Windows</option></select></Field><p className="helper">{platform === 'windows' ? '在管理员 PowerShell 中执行。' : '以 root 身份执行。'}重复安装会保留设备身份和本地状态。</p><CopyField text={command} label="复制安装命令" /></>
}
