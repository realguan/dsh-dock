// 资源型取数统一入口约定（frontend-migration §2 / §11）。
//
// 一期薄实现：页面播种/刷新直接经 api.* 透传，本文件只固定「取数都从
// resource 层走」的调用位点，方便将来在同一处接入缓存。不预先引入
// TanStack Query——触发条件见 §11（首个 Profile 管理页出现时立 Query
// micro-ADR 再接入）。
import { api } from "./tauri"
import type { ClientUpdate, UpdateStatus } from "@/types/ipc"

export const resource = {
  /** 启动/选择器页播种三维度版本状态；失败返回 null（页内降级为不可知态） */
  updateStatus: (): Promise<UpdateStatus | null> =>
    api.getUpdateStatus().catch(() => null),

  /** About 页播种客户端自更新状态机快照 */
  clientUpdate: (): Promise<ClientUpdate | null> =>
    api.getClientUpdate().catch(() => null),
}
