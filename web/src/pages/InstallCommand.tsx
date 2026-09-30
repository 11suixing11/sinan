import { CopyField, ErrorNotice } from '../components'
import type { Enrollment } from '../types'

export default function InstallCommand({ enrollment }: { enrollment: Enrollment }) {
  return <><p className="helper">先按部署文档独立核对发布公钥并准备可信 sinan-bootstrap，再以 root 身份执行命令。升级会保留设备身份和本地状态。</p>{enrollment.install_command ? <><p className="helper">目标 Agent 版本：{enrollment.installation?.version}</p><CopyField text={enrollment.install_command} label="复制安装命令" /></> : <ErrorNotice message={enrollment.warning ?? '请先导入已签名的 Agent 制品'} />}</>
}
