// 中文文案常量（i18n 预留；frontend-migration §3.6）。
// STEPS 三元组逐字取自 ui/index.html（第三列 hint 是步骤旁灰色说明，
// 不可丢弃）；headlines 是另一套独立文案（与步骤名不同文），两者不得
// 互相推断。未知错误动作 id 回退展示 id 原文——新宿主/新失败模式无需改组件。
export const t = {
  boot: {
    steps: [
      { no: "01", name: "环境检测", hint: "PATH · 版本闸 · 平台" },
      { no: "02", name: "宿主解析", hint: "local：system → bundle → download" },
      { no: "03", name: "启动 DSH", hint: "--port 0" },
      { no: "04", name: "等待就绪", hint: "解析访问地址（慢速冷启动可稍候）" },
      { no: "05", name: "进入工作台", hint: "WebView 导航" },
    ],
    headlines: [
      "检查运行环境",
      "确定启动方式",
      "启动 DSH",
      "等待工作台就绪",
      "即将进入工作台",
    ],
  },
  // 错误动作文案表：boot:error payload 的 actions[] 只给 id，文案在此映射；
  // 组件层以 t.error.actions[id] ?? id 兜底。
  error: {
    fallbackTitle: "启动失败",
    actions: {
      retry: "重试",
      upgrade: "升级 DSH 并重试",
      upgrade_only: "后台升级",
      reselect: "返回重选",
    } as Record<string, string>,
  },
  mode: {
    title: "选择运行环境",
    local: "本机运行",
    wsl: "WSL 中运行",
    setDefault: "设为默认方式",
    next: "下一步",
  },
  selector: {
    title: "选择工作台",
    subtitle: "多个 webUi 工作台并存，选择本次启动进入哪一个",
  },
  about: {
    title: "关于与更新",
    tagline: "更新中心 · 桌面客户端与运行环境",
    // 客户端状态机文案（键 = UpdatePhase，failed/done 附带数据的行在组件内插值）
    clientLabel: "桌面客户端",
    phases: {
      idle: "待定",
      checking: "检测中",
      available: "可用",
      upToDate: "最新",
      downloading: "下载中",
      installing: "安装中",
      relaunching: "重启中",
      done: "完成",
      failed: "失败",
    },
    lines: {
      idle: "点击「检查更新」查询官方发布源",
      checking: "正在检查官方更新源",
      upToDate: "已是最新",
      downloading: "正在下载新版本",
      installing: "正在安装新版本",
      relaunching: "即将重启进入新版本",
      failedTitle: "更新失败",
    },
    foundNew: "发现新版",
    releaseNotes: "发布说明",
    downloadBtn: "下载并安装",
    preparing: "准备安装…",
    checkBtn: "检查更新",
    updatedDone: "已更新到",
    restartNote: "客户端更新由官方 Releases 签名分发，安装后自动重启。",
    // 运行环境两维度
    envTitle: "运行环境",
    dshLabel: "DSH",
    nodeLabel: "Node 运行时",
    notDetected: "未检出",
    detecting: "检测中…",
    hasNew: "有新版",
    latestIsNewest: "已是最新",
    latestOfficial: "官方最新",
    notYetLocal: "本地尚未检出",
    checkFailedNet: "DSH 检测失败（网络不可达）",
    nodeFromSystem: "来自你的系统",
    nodeManaged: "应用托管 · 随启动自动准备",
    nodeUnknown: "尚未确定",
    dshUpgradeNote: "升级 DSH 只动 pnpm/npm 全局包，不触碰你的数据与配置。",
    upgrading: "升级中…",
    upgradeFailed: "升级失败",
    btnCheck: "检查",
    btnUpgrade: "升级",
    // 工作台入口
    openInBrowser: "在浏览器中打开",
    workbenchNotReady: "工作台尚未就绪",
  },
} as const

export type AppCopy = typeof t
