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
    checkUpdate: "检查更新",
    applyUpdate: "确认更新",
    upToDate: "已是最新版本",
    available: "有可用更新",
    downloading: "正在下载更新",
    installing: "正在安装",
    relaunching: "即将重启",
    failed: "更新失败",
    restartToApply: "重启应用以完成更新",
  },
} as const

export type AppCopy = typeof t
