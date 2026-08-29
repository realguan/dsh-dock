// Profile 管理器状态（4.3 前端刀）。仅承载列表与默认值两项共享态；
// 详情与各对话框状态是对话框局部的（开窗时经 api 播种，不进全局 store）。
// 取数不走 TanStack Query：本页是「读一次 + 变更后手动刷新」形态，
// 无服务态同步诉求——frontend-migration §11 的 Query 触发条件在此裁定延后，
// 若未来出现跨窗口订阅需求再立 micro-ADR。
import { create } from "zustand"
import { api } from "@/lib/tauri"
import type { ProfileSummary } from "@/types/ipc"

interface ProfilesState {
  list: ProfileSummary[]
  /** 默认启动 profile；null = 未设置（读取侧兜底 web） */
  defaultProfile: string | null
  /** 当前会话占用的 profile；null = 无活跃会话（含切换 boot 中） */
  activeProfile: string | null
  loaded: boolean
  loading: boolean
  /** 列表加载失败文案（区别于操作错误——那是对话框/页面局部态） */
  loadError: string | null

  load: () => Promise<void>
}

export const useProfilesStore = create<ProfilesState>((set, get) => ({
  list: [],
  defaultProfile: null,
  activeProfile: null,
  loaded: false,
  loading: false,
  loadError: null,

  load: async () => {
    if (get().loading) return
    set({ loading: true })
    // 列表/默认值/运行中并行播种；默认值与运行中失败不阻塞列表（降级视图）
    const [list, def, active] = await Promise.all([
      api.listProfiles().catch(() => null as unknown as ProfileSummary[]),
      api.getDefaultProfile().catch(() => null),
      api.getActiveProfile().catch(() => null),
    ])
    if (list === null) {
      set({
        loading: false,
        loaded: true,
        loadError: "profile 列表读取失败——请确认 dsh 环境后重试",
      })
      return
    }
    set({
      list,
      defaultProfile: def,
      // getActiveProfile 失败与「无活跃会话」同为 null，视图语义一致（无徽标）
      activeProfile: active,
      loaded: true,
      loading: false,
      loadError: null,
    })
  },
}))
