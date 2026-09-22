// 中文文案常量（i18n 预留；frontend-migration §3.6）。
// STEPS 三元组逐字取自 ui/index.html（第三列 hint 是步骤旁灰色说明，
// 不可丢弃）；headlines 是另一套独立文案（与步骤名不同文），两者不得
// 互相推断。未知错误动作 id 回退展示 id 原文——新宿主/新失败模式无需改组件。
// 2026-09-07：boot 面向广泛用户去术语化（PATH/spawn/tier/WebView 等内部
// 词汇不再外露；headlines 仍被 BootSelector 消费，仅微调）。
export const t = {
  boot: {
    progressLabel: "启动进行中",
    steps: [
      { no: "01", name: "环境检测", hint: "检查电脑环境是否满足运行条件" },
      { no: "02", name: "准备引擎", hint: "准备 DSH 运行所需组件（首次使用需下载）" },
      { no: "03", name: "启动工作台", hint: "正在启动 DSH 服务" },
      { no: "04", name: "等待就绪", hint: "即将可用，首次启动可能较慢" },
      { no: "05", name: "进入工作台", hint: "马上为你打开工作台界面" },
    ],
    headlines: [
      "检查运行引擎",
      "准备运行环境",
      "启动工作台",
      "等待服务就绪",
      "即将进入工作台",
    ],
    stRunning: "运行中",
    copyDetail: "复制详情",
    copied: "已复制",
    progressAria: "启动进度：{done} / {total}",
    dlPackages: "{done} / {total} 个包",
    // 下载卡静态文案（2026-09-18 收口：原为 DownloadProgress 组件内联中文，
    // en 用户可见中文＝真 i18n 缺陷）。
    dlKindNode: "Node.js 运行时",
    dlKindDsh: "DSH 引擎",
    dlEngineBadge: "引擎在线引导",
    dlTransferred: (bytes: string) => `已传输 ${bytes}`,
    dlMirrorNote: "官方镜像链下载 · 完整性校验",
    dlSelfContainedNote: "自包含引擎 · 首启就绪后离线直通",
    wslOpen: "在 WSL 中打开",
    wslOpenTip: "在 WSL2 发行版内运行 DSH（需 Windows + WSL2）",
    wslFailed: "WSL 切换失败",
    localOpen: "在本机中打开",
    localOpenTip: "切换回本机模式运行 DSH",
    localFailed: "本机切换失败",
    controlCenter: "控制中心",
    controlCenterTip: "打开控制中心面板（管理 Profile / 插件 / 凭据 / 诊断）",
    launchingTitle: "正在启动工作台…",
    launchingSub: "正在连接本地服务，即将进入工作台界面",
    viewTimeline: "查看启动详情",
    hideTimeline: "收起详情",
  },
  // 交接（ADR-0014）：一次重启/切换的四段贯穿状态。同一条导轨同时出现在
  // 控制中心与主窗口启动屏（同一视图模型），计时器跨窗口、跨文档连续。
  handoff: {
    titleStart: (name: string) => `正在启动「${name}」`,
    titleRestart: (name: string) => `正在重启「${name}」`,
    titleSwitch: (name: string) => `正在切换到「${name}」`,
    stageStopping: "停止旧会话",
    stageBooting: "启动新会话",
    stageWaiting: "等待就绪",
    stageEntering: "进入工作台",
    phaseStopping: "正在停止当前会话…",
    phaseBooting: "正在准备并启动新会话…",
    phaseWaiting: "服务已启动，正在等待就绪…",
    phaseEntering: "已就绪，正在打开工作台界面…",
    phaseReady: "工作台已就绪",
    phaseFailed: "启动中断——详情见主窗口错误卡",
    elapsedTitle: "本次操作已用时（跨窗口连续计时）",
    focusWorkbench: "查看进度",
    enterWorkbench: "进入工作台",
    viewFailure: "查看错误详情",
    dismissFailure: "收起这条失败提示",
  },
  // 错误动作文案表：boot:error payload 的 actions[] 只给 id，文案在此映射；
  // 组件层以 t.error.actions[id] ?? id 兜底。
  error: {
    fallbackTitle: "启动失败",
    // 动作失败详情（2026-09-11 task-27）：原为组件内 `` `${t.error.actionFailed}：${msg}` ``——
    // 组合串里的分隔符是**全角冒号**，en 用户会看到 `Action Failed：...`。
    // 按本仓既有惯例改为组合键（同 `market.installFailed`）：分隔符随语言定义，
    // zh 全角冒号无空格 / en 半角冒号 + 空格。原 `actionFailed` 单独标签已无消费者，一并删除。
    actionFailedDetail: (msg: string) => `动作失败：${msg}`,
    // 剪贴板写入失败（2026-09-08，批次 0b）：绝不静默假装「已复制」
    copyFailed: "复制失败——请手动选中内容复制",
    actions: {
      retry: "重试",
      upgrade: "升级 DSH 并重试",
      upgrade_only: "后台升级",
      reselect: "返回重选",
      // v1.2.0 D1（task-52）：Windows 本地模式因符号链接特权失败时的出路。
      // 契约：`boot_failure.rs:135` `vec!["boot_in_wsl", "retry"]`；
      // 无此键会兜底成英文 id `boot_in_wsl`（`actionLabel` 的 `?? id`）。
      // 文案取自 D1 §5 建议：zh「改用 WSL 模式打开」。
      boot_in_wsl: "改用 WSL 模式打开",
      // 2026-09-16：插件的挂载行把插件树搞挂时的**就地**出路（只移除那一行 + 重启）。
      // 契约：`boot_failure.rs::with_quarantine`（下发到 `advancedActions`，**首屏不展示**）；
      // 无此键会兜底成英文 id。
      quarantine_plugin_row: "只移除出错的那一行并重启",
      // ADR-0026 安全模式（2026-09-16 第二版裁定）：首屏**只剩这一个**动作，
      // 语义 = 在 profile 配置里把所有三方插件写成 disabled（覆写前备份），随后正常启动。
      safe_mode: "停用全部三方插件并启动",
      safe_mode_reset: "备份并放空插件配置后启动",
    } as Record<string, string>,
    // 动作 → **它会造成什么**（首屏与按钮并排显示；2026-09-16 维护者裁定）。
    // 与 `ErrorCard.tsx` 的 ACTION_IPC 集合一一对应：新增动作必须同时补两侧文案。
    impacts: {
      safe_mode: "在配置里停用全部三方插件（覆写前备份），进应用后按需在「插件」页重新打开",
      safe_mode_reset: "插件配置已写坏（连行都读不出来）时用：先备份为 .bak-<时间戳>，再放空该文件",
      quarantine_plugin_row: "从 cordis.patch.yml 删掉出错的那一行（覆写前自动备份）",
      retry: "重新走一遍同样的启动流程",
      upgrade: "先升级 DSH 再重试，升级只动 pnpm/npm 全局，不碰你的数据",
      upgrade_only: "只升级 DSH、不打断当前会话",
      boot_in_wsl: "改在 WSL 里启动，绕开本地模式的权限限制",
      reselect: "回到 profile 选择页",
    } as Record<string, string>,
    // 「其它出路」区标题（2026-09-16）：首屏只有一个按钮，其余出路收进"展开详情"。
    advancedLabel: "其它出路（一般用不到）",
    safeModeResetTitle: "备份并放空插件配置后再启动？",
    safeModeResetNote: "用于插件配置已写坏、连行都枚举不出来的情况。",
    safeModeResetPointBackup: "你的 cordis.patch.yml 会先备份为 .bak-<时间戳>，可随时手动还原",
    safeModeResetPointScope: "该 profile 的全部插件挂载行会失效，工作台以「只有随包能力」的形态启动",
    safeModeResetConfirm: "备份并继续",
    // 失败详情后缀（2026-09-11 task-25）：原为组件内联字面量（zh-CN 全角括号、
    // en 侧需 ASCII 括号 + 前置空格），故并入字典。两处同源拼接：
    // `pages/BootSelector.tsx` 与 `components/boot/ErrorCard.tsx`。
    reselectHint: "（可返回重选）",
    // 错误卡收起/展开（v1.2.0 实测 2.1）：诊断卡此前只能看不能关，卡在已迁移成功的
    // 画面上与向导互相顶着。折叠只折内容——卡头（警示图标 + 标题 + 序号）保持可见，
    // 故「错误信息不得被误藏」仍成立。
    collapseDetail: "收起",
    expandDetail: "展开",
    // 错误卡静态标签（2026-09-08，ADR-0012 顺手收口硬编码中文）
    diagHeader: "DIAG 诊断控制台",
    cardHeader: "启动中断",
    suggestionLabel: "修复建议：",
    // 诊断日志折叠区（2026-09-11 task-27）：原为 ErrorCard 内联字面量
    // （en 用户可见中文）。`copyLog` 与 `boot.copyDetail`（BootStep 的「复制详情」）
    // 语义不同——这里复制的是原始诊断日志，故独立成键而非复用。
    // 「已复制」状态复用既有 `boot.copied`（同一 boot 流程族的通用串，避免重复键）。
    copyLog: "复制日志",
    rawLogSummary: (n: number) => `原始诊断日志 · 尾部 ${n} 行`,
    // 分类文案（ADR-0012）：按 failure.kind 取；取不到回退后端 title/suggestion
    kinds: {
      credentials_mismatch: {
        title: "宿主 DSH 与您的凭据格式不匹配",
        suggestion:
          "通常是 DSH 版本过旧：升级到官方最新版可解决（升级只动 pnpm/npm 全局，不碰您的数据）。",
      },
      incompatible_options: {
        title: "宿主 DSH 参数不兼容",
        suggestion: "请升级您的 DSH 到支持当前终端行为的版本。",
      },
      network_unavailable: {
        title: "网络不可用",
        suggestion: "实时下载需要网络连接；检查网络后重试。",
      },
      // v1.2.0 D1（task-52）：与 Rust `boot_failure.rs::title/suggestion` 对齐。
      // 设计要点（D1 §4）：**先否定错误方向**——用户的自然排查方向是网络，
      // 真因是 Windows 符号链接特权；不点破就会继续白费时间。
      symlink_privilege_required: {
        title: "系统权限不足：无法创建符号链接",
        suggestion:
          "这不是网络问题：引擎需要创建符号链接，而当前 Windows 账户没有该权限。最省事的办法是改用「WSL」运行环境（在启动页选择，或在控制中心设为默认）——已装好的 WSL 发行版不需要该权限。若必须用本地模式：以管理员身份运行本应用，或在「设置 → 系统 → 开发者选项」开启开发者模式后重试。",
      },
      unknown: {
        title: "DSH 工作台启动失败",
        suggestion: "详情见日志；可重试，若持续请反馈。",
      },
    } as Record<string, { title: string; suggestion: string }>,
  },
  mode: {
    title: "选择运行环境",
    subline: "Windows 支持在本地原生运行或在 WSL2 发行版内运行；首次启动请选择默认环境",
    local: "本地模式（Native Windows）",
    localDesc: "直接在 Windows 原生环境中运行，首启自动引导 Node 与 DSH；秒级极速直启、低内存占用",
    localBadge: "推荐 · 原生极速",
    wsl: "WSL 模式（WSL2 Linux）",
    wslDesc: "在 WSL2 发行版内运行 DSH（需 Windows + WSL2，客体内自动铺设 Node 与 DSH 引擎）",
    wslBadge: "Linux 隔离环境",
    selectedNotice: "已选定此模式",
    setDefault: "设为默认运行环境（下次自动以此模式启动）",
    changeAnytime: "随时可在设置或托盘菜单中更改",
    next: "开始",
    starting: "正在初始化运行环境…",
    failed: "运行环境启动失败",
  },
  selector: {
    title: "选择工作台",
    subtitle: "多个 webUi 工作台并存，选择本次启动进入哪一个",
    headline: "进入哪个工作台？",
    subline: "选择本次要启动的工作空间 · 可设为默认以便下次直接进入",
    pickHint: "官方 Web 工作台开箱即用；其余为你自定义装配的 webUi 工作台。",
    launchingPrefix: "正在启动「",
    launchingSuffix: "」",
    preparingTitle: "正在准备运行引擎",
    preparingSub: "首启自动引导 Node 与 DSH 运行时 · 仅需一次",
    problemHeadline: "启动遇到问题",
    // profile 元数据映射：已知名给正式标题/描述，未知名回退 CUSTOM 形态
    // 2026-09-11（task-27）：删除 `tag` 字段与 `customTag` 键——task-23 解耦后它们
    // 已无任何消费者（唯一残留是组件里把字典值搬进回退对象的死代码，同批删除）。
    // 这使「字典值当控制流令牌」在**结构上**不可能复发：判据无处可读。
    items: {
      web: { title: "官方 Web 工作台", desc: "DSH 官方维护的 Web 界面" },
    } as Record<string, { title: string; desc: string }>,
    customDesc: "自定义装配的 webUi 工作台",
    // 顶栏版本芯片短文案
    chipDshNew: "有新版",
    chipDshOk: "已是最新",
    chipDetecting: "检测中",
    chipCheckFailed: "检测失败",
    chipClientNew: "客户端有新版",
    chipClientUpdating: "客户端更新",
    chipClientUpdatingRun: "客户端更新中…",
    // 全新工作台启动台（Workbench Launchpad）
    rememberChoice: "记住我的选择，下次启动直接进入此工作台",
    rememberChoiceSub: "开启后下次启动将跳过此页面直接进入所选工作台，随时可在工作台管理中心更改",
    setDefaultAction: "设为默认",
    defaultSetSuccess: "已设为默认工作台",
    currentDefaultNotice: "当前默认工作台",
    recommendedBadge: "推荐",
    quickKeysHint: "按数字键 1-9 快速选择启动",
    createWorkbench: "创建全新工作台",
    createWorkbenchDesc: "克隆模板或在控制中心定制独立工作空间",
    manageWorkbenches: "工作台管理中心",
    engineReady: "引擎环境已就绪",
    pluginsCount: "{count} 个插件",
    defaultBadge: "默认",
    // 官方 web 工作台且无第三方依赖时的卡片底栏说明（2026-09-11 task-25）：
    // 原为组件内联字面量（en 用户可见中文），可达条件见 pages/BootSelector.tsx。
    officialReadyToUse: "官方开箱即用",
    enterWorkbench: "进入工作台",
    enterDefaultWorkbench: "进入默认工作台",
    launching: "正在启动…",
    // 工作台列表读取中（2026-09-18 收口新增加载态）
    profilesLoading: "正在读取工作台列表…",
  },
  updateBanner: {
    // 升级提示条（ADR-0010 升级呈现；非阻断、可忽略同版本）
    dshTitle: "dsh 有新版本 v{latest}（当前 v{current}）",
    dshConsequence: "升级可获得安全修复与改进；长期不升级可能导致未来版本门槛抬升后无法直接追新。",
    clientTitle: "桌面客户端有新版本 v{latest}",
    clientConsequence: "建议升级以获得问题修复与体验改进。",
    dismissTip: "忽略此版本（不再提醒）",
    entryHint: "可从菜单 / 托盘的「关于」进入更新中心。",
  },
  about: {
    workbenchLabel: "工作台实例",
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
      idle: "尚未检查更新",
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
    nodeFromEngine: "壳引擎 · 随应用内置管理",
    nodeFromSystem: "来自你的系统",
    nodeManaged: "应用托管 · 随启动自动准备",
    // 未安装态（2026-09-10）：如实报"没装"，并把**计划**版本放进括号——
    // 绝不把计划版本渲染成已装版本（v1.1.0 Windows 实测 1.2 的"说谎"修复）。
    nodeNotInstalled: "未安装",
    nodeNotInstalledPlanned: (v: string) => `未安装（计划 ${v}）`,
    dshUpgradeNote: "升级 DSH 只动 pnpm/npm 全局包，不触碰你的数据与配置。",
    upgrading: "升级中…",
    upgradeFailed: "升级失败",
    upgradeRunning: "正在全局安装（pnpm/npm），可能需要数分钟…",
    btnCheck: "检查更新",
    btnUpgrade: "升级",
    // 版本选择器（2026-09-09）：「有新版」只按可升级口径（稳定/rc）判定；
    // 预览版经版本列表显式选择安装，选择权交给用户。
    btnUpgradeTo: "升级到",
    installingVersion: "正在安装",
    previewReleased: "预览版已发布",
    versionListEntry: "查看全部版本",
    noteUpgraded: "已升级到",
    noteAlreadyLatest: "已是最新",
    onPreview: "预览运行中 · 稳定口径",
    versionListTitle: "DSH 版本列表",
    versionListHint: "预览版可能不稳定，安装后可随时回退。",
    filterAll: "全部",
    filterUpgradable: "稳定·候选",
    filterPreview: "预览",
    channelStable: "稳定",
    channelRc: "候选",
    channelAlpha: "预览",
    channelOther: "预览",
    rowInstall: "安装",
    rowRollback: "回退",
    rowCurrent: "当前",
    listLoadFailed: "版本列表获取失败",
    listRetry: "重试",
    listEmpty: "没有符合筛选的版本",
    cancelBtn: "取消",
    confirmAlphaTitle: "安装预览版",
    confirmAlphaNote: "是预览版本，可能不稳定或依赖未完整发布导致无法启动。",
    confirmAlphaPoints: [
      "预览版仅建议用于尝鲜与问题反馈，不建议在生产环境使用",
      "遇到问题可随时从版本列表回退到稳定版本",
    ],
    confirmRollbackTitle: "回退到旧版本",
    confirmRollbackNote: "比当前已装版本旧，回退将重装全局 dsh 包。",
    confirmRollbackPoints: [
      "回退只重装全局 dsh 包，个人数据与配置不受影响",
      "新版本创建的会话可能无法被旧版本打开",
    ],
    confirmInstallAnyway: "仍要安装",
    confirmRollbackOk: "确认回退",
    // 工作台入口
    openInBrowser: "在浏览器中打开",
    workbenchNotReady: "工作台尚未就绪",
    liveSessionActive: "工作台实例就绪",
    copyDiagnostics: "复制诊断信息",
    diagnosticsCopied: "诊断信息已复制到剪贴板",
    officialChannel: "官方发布通道",
    // 2026-09-18 UI 收口：原为 about/ 组件内硬编码中文，逐一入字典
    notesExpand: "展开全部",
    notesCollapse: "收起",
    fetchingRelease: "正在获取资源…",
    workbenchBridgeHint: "启动 DSH 后将自动建立本地 HTTP 桥接",
    versionFilterLabel: "版本筛选",
  },
  // 控制中心（4.3 前端刀）。
  profiles: {
    aboutEntry: "关于 / 更新",
    aboutEntryTip: "打开关于窗口（本机没有托盘 / 菜单栏入口时的备用入口）",
    moreActions: "更多操作",
    listLabel: "Profile 列表",
    detailWorkspaceLabel: "Profile 详情工作区",
    distributeTargetLabel: "选择目标 Profile",
    distributeWithConfig: "连带复制配置行",
    mcpDisable: "停用该 MCP 服务",
    // 行内启停的无障碍名：一屏多行时必须说清是哪一条（读屏只念这一个字符串）。
    mcpEnableAria: (name: string) => `启用 MCP 服务 ${name}`,
    mcpDisableAria: (name: string) => `停用 MCP 服务 ${name}`,
    mcpEnvRemove: "删除该环境变量",
    title: "控制中心",
    subtitle: "多工作台管理、插件生态矩阵、会话自愈维护与系统控制台",
    refresh: "刷新",
    refreshing: "刷新中…",
    refreshData: "刷新控制台数据",
    dataRefreshed: "已刷新控制台数据与状态",
    reloadingWorkbench: "正在重载主工作台…",
    launchingProfile: "正在启动服务…",
    switchToDsh: "返回工作台",
    switchToDshTip: "快速切换至 DSH 主工作台（快捷键：⌘ + , / Ctrl + , 双向切换）",
    createBtn: "新建工作台",
    loadFailed: "profile 列表读取失败——请确认 dsh 环境后重试",
    retryLoad: "重试",
    // 列表行
    tagMaterialized: "已创建",
    tagTemplate: "可首启",
    tagNoUi: "无界面",
    templateHint: "内置模板 · 首次启动自动创建",
    defaultBadge: "默认启动",
    runningBadge: "运行中",
    setDefault: "设为默认启动",
    defaultIs: "当前默认",
    launch: "启动",
    launchWorking: "启动中…",
    metaBundles: (n: number) => `${n} 个插件`,
    metaDeps: (n: number) => (n > 0 ? `${n} 项依赖` : "无额外依赖"),
    metaSep: "·",
    // 详情
    detailTitle: (name: string) => `「${name}」详情`,
    detailPackage: "清单名",
    detailBundles: "插件组合（dsh.profile.bundles）",
    detailDeps: "插件列表（dependencies）",
    detailPatch: "cordis.patch.yml 原文",
    detailPatchNone: "尚无 patch 层（首次启动时由 dsh 生成）",
    detailEmptyDeps: "本 Profile 还没有任何插件",
    detailClose: "关闭",
    // 插件清单（4.4①）：静态清单 + 运行态快照（复现点 11）
    pluginNotInstalled: "未安装",
    pluginAddBtn: "添加插件",
    pluginAddTitle: (name: string) => `添加插件到「${name}」`,
    pluginAddDesc: "在当前工作台直接浏览市场插件、从其他工作台导入或输入 npm 包名安装",
    pluginAddTabMarket: "插件市场",
    pluginAddTabImport: "从其他导入",
    pluginAddTabCustom: "自定义安装",
    pluginAddMarketSearch: "搜索社区插件名称、描述或标签...",
    pluginAddAlreadyInstalled: "已安装于此工作台",
    pluginAddInstallAction: "安装",
    pluginAddInstalling: "安装中…",
    pluginAddCustomPlaceholder: "输入 npm 包名或 spec，例如 dsh-plugin-xxx 或 @scope/pkg@^1.0.0",
    pluginAddCustomSubmit: "安装到此工作台",
    pluginAddCustomHint: "支持公开发布的 npm 包名、版本范围或预发布 tag",
    pluginAddEmptyMarket: "没有找到匹配的社区插件",
    pluginAddLoadMarketFailed: "插件市场加载失败，请检查网络后重试",
    pluginAddMarketInstallDone: (name: string) => `插件「${name}」已加入安装队列`,
    pluginAddCategoryLabel: "按分类筛选",
    pluginAddMarketLoading: "正在拉取社区插件矩阵…",
    pluginAddMoreHidden: (n: number) => `另有 ${n} 个未列出——用搜索或分类缩小范围`,
    pluginAddImportDone: "导入队列已处理完毕",
    pluginAddImportOk: "成功",
    pluginAddImportFrom: "来自",
    pluginAddSpecLabel: "npm 包名或版本范围",
    pluginDesktopRuntimeToggle: "展开/收起底座说明",
    // 插件安装/卸载/更新（4.4②）：dsh plugin 转发链
    pluginInstallBusy: "安装中…（需要下载，可能数十秒到数分钟）",
    pluginUninstall: "卸载",
    pluginUninstallConfirm: (pkg: string) => `确定卸载插件「${pkg}」？`,
    pluginUninstallPoints: [
      "由 dsh plugin remove 执行，从该 Profile 的依赖中移除",
      "插件自身的配置文件不在此次删除范围内",
      "该 Profile 正在运行的话，重启后生效；再次使用需重新安装",
    ],
    pluginUpdate: "更新",
    // 开关文案（2026-09-17 维护者实机提问「为什么既有已禁用又有已停用」后重写）：
    // 旧文案把「重启后生效」写死在开关标签上，**实测不成立**——web 形态 profile 的
    // `dsh.profile.patchReload = live`，dsh 用 chokidar 盯 cordis.patch.yml，克隆实机
    // 实测：改文件后 0.43s 内 fiber 注销、0.44s 内重建。现在标签只讲动作；是否已生效
    // 由行内运行态徽标如实呈现（生效中 → 已生效 / 未生效 → 重启后生效），不再许一个
    // 可能与事实相反的承诺。
    pluginDisable: "禁用",
    pluginEnable: "启用",
    pluginToggleHint:
      "开关写入该 Profile 的配置：有的 Profile 会立即生效（web 形态实测约半秒），有的要等重启该 Profile——行内徽标会告诉你到底生效没有。",
    pluginDisabled: "已禁用",
    pluginDisabledHint:
      "配置里已禁用（写入该 Profile 的 cordis.patch.yml）——dsh 不会加载这一行。",
    // 运行态徽标：只描述「运行中的 dsh 现在是什么样」，与配置侧徽标（已禁用）分家
    chip: {
      active: "运行中",
      loading: "加载中",
      failed: "失败",
      unloaded: "未加载",
      notApplied: "未生效",
      applying: "生效中",
    },
    chipHint: {
      active: "运行中的 dsh 里这一行已装好并处于活动状态。",
      loading: "正在装载（装好后自动变「运行中」）。",
      failed: "装载失败：插件没跑起来（常见原因：包不可解析、apply 抛错）。",
      unloaded:
        "配置里是启用的，但本次会话里没有它的实例——多因导入失败（包名/依赖不可解析）。",
      notApplied:
        "运行中的 dsh 还没应用这次改动（这个 Profile 采用「启动时生效」的重载策略）：重启该 Profile 后生效。",
      applying: "配置已写入，正在等运行中的 dsh 应用这次开关（实测约半秒）。",
    },
    toggleApplied: (pkg: string, on: boolean) =>
      `${on ? "已启用" : "已禁用"} ${pkg}（已生效）`,
    toggleRestart: (pkg: string, on: boolean) =>
      `${on ? "已启用" : "已禁用"} ${pkg}（重启该 Profile 后生效）`,
    // 无可观测运行态的行（补丁包：贡献行用的是各自的 name，包名不成条目）：观测不到
    // 就不许承诺生效时机，只报"配置已写入"。
    toggleDone: (pkg: string, on: boolean) => `${on ? "已启用" : "已禁用"} ${pkg}（配置已写入）`,
    // 安全模式横幅（ADR-0026，2026-09-16 第四版交互，维护者按 PM 口径重写）：
    // ① 只在"确实以安全模式进入过"时出现（记账驱动），且可关闭、关后同轮不再打扰；
    // ② 文案只讲**发生了什么 + 去哪儿开回来**：不提备份（不做整份恢复，提它全是噪音），
    //    也不把用户往「实验能力」引（那里只有策展能力，而安全模式停的是**全部**三方插件）。
    safeModeTitle: "安全模式",
    safeModeBody: (n: number) =>
      `为让 DSH 能正常启动，全部三方插件已停用（当前 ${n} 个）。把你需要的插件到「插件」页重新打开即可。`,
    safeModeDismiss: "不再提示",
    safeModeDismissFailed: (msg: string) => `保存「不再提示」失败：${msg}`,
    pluginOpBusyRemove: "卸载中…",
    pluginOpBusyUpdate: "更新中…",
    // 更新检查（4.4④）：registry dist-tags 口径 + 版本选择
    checkUpdatesBtn: "检查更新",
    updateCheckStarted: "已发起检查更新，结果见菜单徽标与关于页",
    checkingBtn: "查询中…",
    updateChecked: (r: { checked: number; failed: number }) =>
      `已查 ${r.checked} 个插件${r.failed ? ` · ${r.failed} 个查询失败` : ""}`,
    allUpToDate: "全部最新",
    updateHint: "选择版本更新",
    pickVersionTitle: (pkg: string) => `「${pkg}」可选版本`,
    versionLatest: "最新",
    versionCurrent: "当前",
    versionsLoadFailed: "版本列表查询失败",
    // 2026-09-19 审美批次 3：原句「会话运行中 · 7 行运行中 · …」把"运行中"说了两遍
    // （会话态 + 行数），且会话态详情头部已有徽标。改成纯计数图例。
    runtimeSummary: (s: { active: number; failed: number; loading: number; disabled: number }) =>
      `运行 ${s.active}${s.failed ? ` · 失败 ${s.failed}` : ""}${s.loading ? ` · 加载中 ${s.loading}` : ""}${s.disabled ? ` · 停用 ${s.disabled}` : ""}`,
    runtimeUnavailable: "该 profile 当前未运行，仅显示静态清单",
    runtimeFailedSummary: (n: number) => `运行时 ${n} 个条目加载失败（详见系统控制台日志）`,
    // 官方桌面运行时说明（2026-09-11）：原内联「内含 240+ 项本地预置底座服务」——
    // 数字随 dsh 版本漂移且无壳侧事实源（本机 desktop-packages.json 为 239 项），
    // 故改不依赖具体数量。句子含两个 `<code>` 标识符，字典只存纯文本片段：
    // Prefix + <code>@deepseek-ai/dsh-desktop-runtime</code> + Mid +
    // <code>desktop-packages/</code> + Suffix，翻译时保持首尾括号与空格。
    desktopRuntimeDescPrefix: "该 Profile 归属于 DeepSeek 官方桌面客户端（",
    desktopRuntimeDescMid: "），内含本地预置的底座服务与核心组件（存放在 ",
    desktopRuntimeDescSuffix: " 本地包目录）。",
    // 创建
    createTitle: "新建 Profile",
    createNameLabel: "Profile 名字",
    createNamePlaceholder: "如 my-workbench",
    createNameHelp: "同一名字再次创建即重试安装（幂等）；已完整存在的名字会被拒绝",
    createTemplateHint: (name: string) =>
      `「${name}」是内置模板：将以官方模板初始化（${name} = 基础 + ${name === "web" ? "Web" : "Headless"} 工作台）`,
    createDefaultHint: "将创建基础 + Web 工作台（与出厂 web 模板同构、零下载），可设为默认启动",
    createSubmit: "创建",
    createBusy: "创建中…（本地初始化，通常秒级完成）",
    createDoneReady: "创建完成：基础 + Web 工作台就绪，可立即启动或设为默认",
    createDonePending: "已创建，但插件安装未完成——可稍后重试创建（幂等）",
    createDoneFailed: "创建未完成",
    createAgain: "再试一次",
    // 复制 / 重命名
    copyTitle: (name: string) => `复制「${name}」`,
    renameTitle: (name: string) => `重命名「${name}」`,
    newNameLabel: "新名字",
    copyNote: "复制会排除 node_modules（新目录由 pnpm 重新安装），其余文件逐字照搬",
    renameNote: "重命名后 node_modules 将被清理，dsh 下次启动自动重装",
    submitCopy: "创建副本",
    submitRename: "确认重命名",
    actionRename: "重命名",
    actionDelete: "删除",
    busyShort: "处理中…",
    renameSame: "新名字与当前名字相同",
    nameOccupied: "该名字已被占用",
    renameDone: (name: string) => `已重命名为「${name}」`,
    copyDone: (name: string) => `已创建副本「${name}」`,
    warningsLabel: "需要人工检查",
    opFailed: "操作失败",
    // 删除确认（要素按 ADR-0009 §2/§4 要求逐条列明）
    deleteTitle: (name: string) => `删除「${name}」？`,
    deleteNote: "此操作不可撤销",
    deletePoint: [
      "只删除该 profile 目录，不会删除会话、凭据等全局数据",
      "确保没有其他 dsh 实例（含终端自启）正在使用该 profile",
      "若它是内置模板名（web/headless），删除后首次使用将重新物化",
    ] as readonly string[],
    deleteConfirm: (name: string) => `确认删除 ${name}`,
    deleteBusy: "删除中…",
    deleteDoneCleared: "已删除；它曾是默认启动 profile，已回退为 web",
    deleteDone: "已删除",
    // 设默认
    setDefaultDone: (name: string) => `已设「${name}」为默认启动 profile`,
    // 切换（4.3⑥）：停当前会话以目标 profile 重启；不写默认，失败可重试
    switchTitle: (name: string) => `切换到「${name}」？`,
    switchNote: "将停止当前 dsh 并以该 profile 重启，进行中的任务会中断（会话历史已落盘，不受影响）；默认启动设置不变。",
    switchFrom: (active: string) => `当前运行：「${active}」`,
    switchConfirm: (name: string) => `切换到「${name}」`,
    switchBusy: "切换中…",
    switchDone: (name: string) => `已开始切换到「${name}」，进度见主窗口`,
    // 重启（4.4③）：同 profile 走切换链；恒弹确认（必杀运行中会话）
    restart: "重启",
    restartTitle: (name: string) => `重启「${name}」？`,
    restartNote: "将停止当前 dsh 并以同一 profile 重新启动，进行中的任务会中断（会话历史已落盘，不受影响）；刚安装/卸载/禁用的插件借此生效。",
    restartConfirm: (name: string) => `重启「${name}」`,
    restartBusy: "正在重启…",
    restartDone: (name: string) => `已触发「${name}」重启，进度见主窗口`,
    // 视图切换（4.4④ 收口）：管理器页内视图
    viewProfiles: "Profile 列表",
    viewPluginHub: "插件中心",
    viewPlugins: "已安装插件",
    viewMarket: "插件市场",
    viewSessions: "会话维护",
    viewConsole: "系统控制台",
    overviewSubtitle: "全部 profile 的第三方插件分布",
    overviewSourceCount: (n: number) => `${n} 个 profile`,
    overviewEmpty: "各 profile 还没有安装第三方插件",
    overviewEmptyHint: "在任一 profile 的详情里安装后，这里会自动汇总",
    overviewNotInstalled: "未安装",
    // 从其他 profile 安装（4.4④）：多选批量 + 可选连配置（写入例外 #4）
    importLoading: "正在扫描其他 profile…",
    importEmpty: "其他 profile 还没有已安装的第三方插件",
    importConfig: "连配置",
    importNoConfig: "无配置可带",
    importSelected: (n: number) => `已选 ${n} 项`,
    importStart: (n: number) => `安装 ${n} 项`,
    importRunning: (i: number, total: number) => `正在安装 ${i}/${total}`,
    importDone: (ok: number, fail: number) =>
      `从其他 profile 安装完成：${ok} 成功${fail ? ` · ${fail} 失败` : ""}`,
    // 扩展文案（Master-Detail 工作台与交互升级）
    searchPlaceholder: "搜索 Profile...",
    searchPluginsPlaceholder: "搜索已装插件...",
    searchAllPluginsPlaceholder: "搜索全域插件名称、描述或 Profile...",
    tabPlugins: "插件列表",
    // 实验能力（ADR-0028：自插件中心迁入，与插件列表同页并列）。
    tabCaps: "实验能力",
    // ── 插件列表三类合一（2026-09-20，ADR-0028 第二批：底座 / 第三方 / 实验性一行，
    //    「底座组合」tab 退役）────────────────────────────────────────────
    // 筛选 chips（与三个行内标记同名）。「内置」筛选含 dsh 自带的实验层——它们
    // 也带「内置」标，三类计数因此可以重叠（合计 ≥ 总数），不是分区。
    listFilterBuiltin: "内置",
    listFilterThirdParty: "社区",
    listFilterExperimental: "实验性",
    listFilterEmpty: "这一类下暂时没有插件",
    // 实验性行在插件列表里**只读**（2026-09-21 维护者裁定：操作唯一入口 = 「实验能力」）。
    // 不说清这一点，用户只会看到一行行没有开关的插件，以为是界面坏了。
    experimentalReadOnlyNote:
      "实验性插件的启停、换后端与移除统一在「实验能力」里进行（本页只如实列出装了哪些包）",
    experimentalManageEntry: "前往实验能力",
    listFilterShowAll: (n: number) => `显示全部 ${n} 个插件`,
    // 三个行内标记（每个插件恰好一个主标记；dsh 自带的实验层额外补一枚「内置」）。
    // 2026-09-21 三次修订：能力名**不再挂在这枚实验标上**——它上移到行内主标题旁的
    // 辅助灰字（维护者："插件名取原本的名字，中文为辅"）。标只负责"类别"这一件事，
    // 且已按类筛过时整枚不渲染（"已经按 tab 分类了，就不需要展示 tab 本身这类的标签"）。
    tagBuiltin: "内置",
    tagBuiltinHint: "随 dsh 安装自带的内置层：不可卸载；要开关请到 dsh 自己的插件页",
    tagThirdParty: "社区",
    tagThirdPartyHint: "来自社区（npm / GitHub）的插件：可启停、可更新、可卸载",
    tagExperimental: "实验性",
    tagExperimentalHint: (cap: string) =>
      `属于「${cap}」实验能力的插件：启停、换后端与移除都在「实验能力」里`,
    // 2026-09-21 三次修订：能力卡片退役（实验性插件的操作唯一入口 = 「实验能力」tab），
    // 随之回收的键：tagCapabilityBase(Hint) / tagBackend(Hint) / goToggle* /
    // capActiveBackend(Hint) / capNotToggleable(Hint) / capSwitchBackend(Hint) /
    // capVariantSwitched / capVariantSwitchFailed / capToggle* / capState*Hint / capUnlocks。
    // 状态与后端（状态 + 生效后端合并为单枚胶囊；措辞唯一来源 = StateBadge）。
    capStateOn: "已启用",
    capStateDisabled: "已停用",
    capStateOff: "未启用",
    capStateConflict: "后端冲突",
    capStatePartial: "需要修复",
    // 系统预置底座折叠附录（方案一：与用户扩展物理分开）。
    // 行内 `···` 溢出菜单的无障碍名（一屏多行，必须说清是哪一行）。
    rowMoreActions: (pkg: string) => `${pkg}：更多操作`,
    // 能力目录读取失败（如 WSL 客体档暂不支持）时，插件列表如实说"标记不可用"，
    // 不把没标的实验包默默显示成第三方。
    capsTagUnavailable: "实验能力目录读取失败：「实验性」标记暂不可用（其余标记不受影响）",
    // 内置层的说明。**只有确实知道的那两层才具体说**——原「底座组合」tab 的同一组
    // 文案（2026-09-17 真机截图：agent-team 两层曾被写成「Web 界面渲染器」）。
    bundleDescBase: "Cordis 底座与通用服务插件集合",
    bundleDescWebApp: "Web 界面与交互控制台渲染器",
    bundleDescShipped: "随 dsh 安装自带的一层：在 dsh 自己的插件页里开关",
    tabPatch: "Patch YAML",
    tabMcp: "MCP 扩展",
    mcpTitle: "MCP 服务器管理器",
    mcpSubtitle: "管理 Model Context Protocol 工具与外部服务扩展（含生效范围）",
    mcpEmpty: "当前 Profile 尚未配置任何 MCP 服务",
    mcpAddBtn: "添加 MCP 服务",
    mcpLoadFailed: "MCP 配置读取失败",
    mcpRetry: "重试",
    mcpEditBtn: "编辑",
    mcpDeleteBtn: "删除",
    mcpDeleteConfirm: (name: string) => `确定移除 MCP 服务「${name}」？`,
    // ── 生效范围（2026-09-18，dsh-app-boot 实查：profile 层 → 全局层 → overlays） ──
    mcpScopeProfile: "仅本 Profile",
    mcpScopeProfileHint: "写在 profiles/<名>/cordis.patch.yml：只有这个 Profile 会加载。",
    mcpScopeGlobal: "所有 Profile",
    mcpScopeGlobalHint: "写在 $DSH_HOME/cordis.patch.yml：每个 Profile 启动都会加载。",
    mcpScopeLabel: "生效范围",
    // 长句一律做**悬浮内容**，表面只留短结论（2026-09-19 裁定，见 ui/info-tip.tsx）。
    /** 跨层同名 = 两条都会加载，后加载的那条实例化失败（上游 serverName 是加载期预留）。 */
    mcpScopeConflict: (name: string, rank: number) =>
      `「${name}」在「仅本 Profile」和「所有 Profile」两层各有一条，这一行按加载序是第 ${rank} 条：两条都会插入，后加载的那条会因 serverName 已预留而实例化失败。请删掉其中一条。`,
    mcpScopeConflictTag: (rank: number) => `跨层重名 · 加载序第 ${rank} 条`,
    /** 同层同名 = 同一个文件里两行都叫这个名字（手抄重复、复制粘贴最常见）。 */
    mcpDupSameScope: (name: string, rank: number) =>
      `本层有两条「${name}」，这一行是第 ${rank} 条：两条都会插入，后加载的那条会因 serverName 已预留而实例化失败。停用或删掉多余的一条即可（启停、探测、删除都只作用于这一行）。`,
    mcpDupSameScopeTag: (rank: number) => `本层重名 · 第 ${rank} 条`,
    // ── `!!js` 表达式行（2026-09-18 立，2026-09-19 收进悬浮）：表面短结论 + 完整口径 ──
    mcpExprTag: "含表达式",
    mcpExprShort: "含 !!js 表达式：只允许启用/停用",
    mcpExprHint:
      "这一行的值写了 !!js 表达式（从环境变量取密钥），界面里显示的是它展开后的文本、不是要填进去的值。结构化保存会把它展平成字面量、密钥引用从此失效，因此这类行只允许切换启用/停用；要改它的配置，请直接在文本编辑器里改所在层的 cordis.patch.yml。",
    // ── 密钥显示（env / headers 常装 token，默认打码） ──
    mcpSecretReveal: "点击显示该值",
    mcpSecretHide: "点击隐藏该值",
    // MCP 能力探测（2026-09-15，ADR-0022 stdio 分支）
    mcpProbeBtn: "探测",
    mcpProbeResult: (srv: string, tools: number, resources: number, templates: number) =>
      `「${srv}」能力：Tools ${tools} · Resources ${resources} · Templates ${templates}`,
    mcpProbeFailed: (srv: string, reason: string) => `「${srv}」探测失败：${reason}`,
    // 2026-09-15（R1b）：探测结果落成行内折叠卡，需要有表头/分节/协议行文案。
    mcpProbeToggle: "展开或收起能力清单",
    mcpProbeAt: (time: string) => `快照 ${time}`,
    mcpProbeProtocol: "协议版本",
    mcpProbeTools: "工具",
    mcpProbeResources: "资源",
    mcpProbeTemplates: "资源模板",
    // SSH 远程工作区向导（2026-09-15，ADR-0023）
    mcpDeleteNote: "将从所在层的 cordis.patch.yml 中安全删除（覆写前自动备份）。",
    /** 同层同名两行时，确认框必须说出是哪一行（行 id 就是文件里那一行的 id）。 */
    mcpDeleteRowId: (rowId: string) => `本层同名有多条，要删的是行 id「${rowId}」这一行。`,
    // 装配状态（2026-09-18）：配置存在 ≠ 已加载。enabled 是配置级生效值，
    // fiberPhase 才说明有没有存活实例（null = 没有实例，既可能是停用也可能是导入失败）。
    // 2026-09-19：表面只放 `*Tag` 短结论，下面四句完整判定进悬浮。
    mcpRuntimeActiveTag: "已加载",
    mcpRuntimeDisabledTag: "运行态停用",
    mcpRuntimeFailedTag: "加载失败",
    mcpRuntimeNotLoadedTag: "未加载",
    mcpRuntimeActive: "当前会话已加载：工具已在模型侧可用",
    mcpRuntimeDisabled: "运行态显示该行已停用（配置未生效）",
    mcpRuntimeFailed: "运行态显示加载失败：请看启动日志里的该行报错",
    mcpRuntimeNotLoaded: (phase: string | null) =>
      phase
        ? `运行态显示 ${phase}：尚未就绪`
        : "运行态显示该行没有存活实例（配置已启用但没加载起来；常见原因：命令不可达、缺依赖、或需要重启会话）",
    mcpModalTitle: "配置 MCP 服务",
    mcpModalDesc:
      "配置服务标识、生效范围、运行命令、参数与环境变量；保存后写入对应层的 cordis.patch.yml。",
    mcpPresetTitle: "快速应用常用预设",
    mcpServerName: "服务标识 (Server Name)",
    mcpNameHint:
      "会进工具名 mcp__<标识>__<工具>，因此只能是字母、数字、下划线或连字符，最长 32 个字符（中文名会让整行插件加载被拒）。",
    mcpNameInvalid: (name: string) => `服务标识「${name}」不符合上游约束：仅限字母、数字、下划线与连字符，最长 32 个字符。`,
    mcpNameTaken: (name: string) =>
      `所选生效范围里已经有「${name}」这一条：新增会改写它而不是再加一条。要换一条请改标识，要改这条请用编辑。`,
    // ── 连接方式（2026-09-18）：两种传输的字段完全不同，选错等于填了两份空配置 ──
    mcpTransportLabel: "连接方式 (Transport)",
    mcpTransportStdio: "本地命令 (stdio)",
    mcpTransportHttp: "远程端点 (streamable-http)",
    mcpTransportStdioHint: "由 dsh 启动一个本地子进程，通过标准输入输出通信：填命令、参数与环境变量。",
    mcpTransportHttpHint: "连接一个已经跑着的 HTTP 端点：填 URL 与请求头，不需要命令。",
    mcpUrl: "MCP 端点 URL",
    mcpUrlHint: "形如 http://127.0.0.1:8000/mcp 或 https://example.com/mcp，必须是 http(s):// 开头。",
    mcpUrlRequired: "streamable-http 必须填 MCP 端点 URL",
    mcpUrlMissing: "该行未配置端点 URL（加载时必定失败）",
    mcpHeaders: "请求头 (Headers)",
    mcpHeadersLabel: "请求头",
    mcpAddHeader: "+ 添加请求头",
    mcpHeaderRemove: "删除该请求头",
    mcpNoHeaders: "无需附加请求头",
    mcpCommand: "启动命令 (Command)",
    mcpArgs: "命令参数 (Args)",
    mcpCwd: "工作目录 (cwd)",
    mcpCwdHint:
      "stdio 子进程的启动目录，可留空。若用「绝对路径 node + 绝对路径入口脚本」写法（完全不依赖 PATH），必须填这里让脚本能解析自己的依赖。",
    mcpCommandHint:
      "命令由 dsh 的子进程解析，它的 PATH 比登录 shell 窄得多：随壳内置的引擎目录里只有 pnpm（没有 npx）。所以一次性拉起包请写 pnpm dlx，命令名不必是版本管理器里的 npx。装在本机的服务写绝对路径（如 /usr/bin/node），并把它所在目录填进下面的工作目录。",
    mcpArgsHint: "按空格分隔；含空格的路径用引号包起来算一个参数，例如 \"/Program Files/node/server.js\"。",
    mcpEnv: "环境变量 (ENV, KEY=VALUE 每行一个)",
    mcpSaveBtn: "保存服务",
    mcpSaveSuccess: (name: string) => `MCP 服务「${name}」已成功保存至 cordis.patch.yml`,
    mcpDeleteSuccess: "MCP 服务已成功删除",
    emptySelectTitle: "未选定 Profile",
    emptySelectSubtitle: "在左侧选择一个 Profile 或新建工作台，即可在此配置插件、查看底座与管理 YAML Patch。",
    quickDistribute: "分发到...",
    distributeTitle: (pkg: string) => `将「${pkg}」分发安装到其他 Profile`,
    distributeNote: "选择目标 Profile，将以相同版本执行安装并可连带迁移配置",
    distributeDone: (pkg: string, target: string) => `已成功将「${pkg}」安装至「${target}」`,
    distributeConfigFailed: (pkg: string, target: string, reason: string) =>
      `「${pkg}」已安装至「${target}」，但配置迁移失败：${reason}`,
    distributeConfigSkipped: (pkg: string, target: string) =>
      `「${pkg}」已安装至「${target}」；目标已有同名配置行，未覆盖`,
    copyPatchSuccess: "Patch YAML 内容已复制到剪贴板",
    rawYamlHint: "此为 dsh 生成的 cordis.patch.yml 权威底层配置（只读查看与诊断）",
    filterAll: "全部",
    filterMaterialized: "已创建",
    filterTemplates: "模板",
    // ==== 2026-09-18 Profiles 区 UI 收口：内联硬编码中文迁入 ====
    actionDetail: "查看详情",
    searchEmpty: "未找到匹配的 Profile",
    mcpLoading: "正在读取 MCP 服务配置...",
    mcpEmptyHint: "支持一键添加 GitHub、Postgres、Brave Search 等 MCP 官方工具库。",
    mcpEnabledTag: "已启用",
    mcpNameRequired: "请输入服务名称",
    mcpCopyPrefix: "复制工具前缀",
    mcpPrefixCopied: (prefix: string) => `已复制工具匹配前缀：${prefix}`,
    mcpApplyPreset: "应用预设",
    mcpAddEnv: "+ 添加变量",
    mcpNoEnv: "无需特殊环境变量",
    mcpNamePlaceholder: "例如 github, filesystem, postgres",
    mcpPresetDescs: {
      github: "搜索代码、管理 Issue、PR 与提交历史",
      filesystem: "安全的本地指定目录文件读写沙箱能力",
      postgres: "连接 PostgreSQL 数据库并执行安全只读/读写 SQL",
      "brave-search": "通过 Brave Search API 进行实时全网检索",
    },
    overviewAllProfiles: "全部 Profile",
    overviewCount: (n: number) => `共 ${n} 个插件`,
    overviewPerPage: (n: number) => `${n} 条/页`,
    overviewScanLoading: "正在扫描全域 Profile 插件矩阵...",
    overviewEmptySearch: "未找到匹配的插件",
    overviewEmptySearchHint: "尝试调整搜索关键词或重置 Profile 筛选条件。",
    overviewLoadFailed: "加载全域插件失败",
    overviewNoDesc: "暂无插件描述说明",
    overviewInstalledTo: (n: number) => `已安装到 (${n}):`,
    overviewInstalledOn: (name: string) => `已安装于 ${name}`,
    overviewRefCount: (n: number) => `${n} 处引用`,
    overviewPrevPage: "上一页",
    overviewNextPage: "下一页",
    overviewPageInfo: (i: number, total: number) => `第 ${i} / ${total} 页`,
    expandAllSources: (n: number) => `展开全部 (${n})`,
    collapseSources: "收起",
    distributePickPlaceholder: "请选择目标 Profile...",
    distributeAllInstalled: "所有已物化 Profile 均已安装此插件",
    distributeTargetVersion: "目标版本",
    distributeWithConfigDesc: "从首个来源 Profile 的 cordis.patch.yml 原样同步配置条目",
    distributeEnqueueBtn: "加入分发队列",
    importConfigCopyFailed: (reason: string) => `插件已安装，但配置行复制失败：${reason}`,
    // ==== 2026-09-18 Profiles 区 UI 收口：ProfileDetailPane 内联硬编码中文迁入 ====
    manifestNameLabel: "清单名: ",
    detailLoading: "正在加载配置档案...",
    // 未物化 profile（目录还不存在：内置模板名首次启动 / 首次 plugin add 才创建）。
    // 2026-09-21 真机 bug：这种档的四个读取全都必失败，而其中能力目录那条把 Rust 的
    // 原始 OS 错误（`No such file or directory (os error 2)`）直接铺给了用户。
    // 现在整页不发请求，只给这一句人话 + 唯一的出路（启动一次）。
    notMaterializedTag: "未物化",
    notMaterializedTitle: (name: string) => `profile「${name}」尚未物化`,
    notMaterializedBody:
      "工作目录还不存在：内置模板名（web / headless）首次启动或首次添加插件后才会创建。之后这里才有插件清单、实验能力与配置。",
    searchNoPlugin: "未匹配到搜索词对应的已装插件",
    desktopRuntimeName: "DeepSeek 官方桌面客户端底座运行时",
    desktopRuntimeTag: "官方桌面版",
    desktopRuntimeNote:
      "注：这些核心组件由客户端底座统一部署维护，属于内置底座体系，不可卸载；它们与 dsh 内置层一样列在下方并带「内置」标记。你安装的第三方与实验能力插件同列，各带自己的标记。",
    copyYaml: "复制 YAML",
  },
  sessions: {
    title: "会话维护与自愈",
    subtitle: "检测、诊断与自愈修复 DSH 会话日志异常（如序列号断裂、交叉并发落盘等）",
    scanBtn: "刷新会话",
    repairAllBtn: "一键全量体检与自愈",
    repairing: "正在自愈修复中…",
    searchPlaceholder: "搜索会话名称、ID 或项目名...",
    filterAll: "全部会话",
    filterNeedsRepair: "仅看异常",
    filterArchived: "已归档",
    groupByProject: "按项目分组",
    allProjects: "全部项目",
    workspacePath: "工作区路径",
    openInFinder: "在文件管理器中打开",
    totalCount: (n: number) => `共发现 ${n} 个会话`,
    healthyCount: (n: number) => `${n} 个健康`,
    needsRepairCount: (n: number) => `${n} 个需修复`,
    repairBtn: "一键修复",
    repairAllDisabled: "仅异常会话需要修复",
    noTitle: "未命名会话",
    statusHealthy: "健康",
    statusNeedsRepair: "需自愈",
    statusUnknown: "状态未知",
    statusRunning: "运行中",
    statusRunningDesc: "会话正在被 dsh 使用，结束后才能安全修复",
    statusHealthyDesc: "会话日志序列完整，可正常打开",
    statusNeedsRepairDesc: "检测到重放重叠/序列异常/世代分叉等可修复异常，建议修复后可查看完整会话",
    statusUnknownDesc: "无法判定健康状态（可能为活跃会话或引擎未就绪）",
    statusUnknownDescWithReason: "无法自动判定健康状态——下方是脚本给出的具体原因",
    repairNeedHint: "仅异常会话可修复",
    backupTag: "已备份",
    // 归档与元数据透出（2026-09-07）：归档口径对齐 dsh 侧栏（默认隐藏），
    // 元数据字段均为信息性展示。
    archivedTag: "已归档",
    archivedTagDesc: "会话已在 dsh 中归档（默认隐藏，可在此查看与修复）",
    subagentTag: "子代理",
    subagentTagDesc: "子代理会话（由其他会话委派产生）",
    endStateStop: "正常结束",
    endStateInterrupted: "回合中断",
    endStateAborted: "已中止",
    endStateError: "异常结束",
    endStateOpen: "未收尾",
    eventCountTitle: "展开后的事件总数",
    agentPresetTitle: "代理预设",
    createdAtTitle: "创建时间",
    validatorHint: (v: string) => `（校验器：${v}）`,
    compressedTag: "Zstd 压缩",
    plainTag: "Plaintext",
    repairSuccess: (id: string) => `会话「${id}」自愈修复完成`,
    repairAllSuccess: "全量会话扫描与自愈完成！",
    emptyList: "暂未发现会话记录",
    emptyFilter: "没有匹配的会话",
    lastUpdated: "最后活跃",
    fileSize: "体积",
    path: "存储路径",
    deleteBtn: "删除",
    deleteConfirmTitle: (id: string) => `确定删除会话「${id}」？`,
    deleteConfirmNote: "此操作将永久删除该会话记录文件及其本地历史，不可撤销。",
    deleteSuccess: "会话记录已成功删除",
    copyPath: "复制路径",
    pathCopied: "会话路径已复制",
    unarchiveBtn: "取消归档",
    unarchiveSuccess: "会话已取消归档，回到活跃列表",
    // 长列表渐进披露（2026-09-18 边界收口）：列表按最后活跃倒序，放开的是"更早"
    showMoreSessions: (n: number) => `显示更早会话（还有 ${n} 条）`,
    // ==== 2026-09-18 Profiles 区 UI 收口：内联硬编码中文迁入 ====
    timeUnknown: "未知",
    timeJustNow: "刚刚",
    timeMinutes: (n: number) => `${n} 分钟前`,
    timeHours: (n: number) => `${n} 小时前`,
    timeDays: (n: number) => `${n} 天前`,
    repairFailed: "修复失败",
    repairAllFailed: "全量修复失败",
    openDirFailed: (reason: string) => `打开目录失败：${reason}`,
    loadingScanning: "正在扫描 DSH 会话记录...",
    viewFlat: "平铺列表",
    statProjects: "工作区项目数",
    statTotal: "会话总数",
    statHealthy: "健康就绪",
    statRunning: "运行中",
    statNeedsRepair: "待修复异常",
    copyIdTitle: "复制会话 ID",
    groupMeta: (n: number, size: string) => `${n} 个会话 · ${size}`,
  },
  console: {
    toastClose: "关闭通知",
    // 2026-09-18 UI 收口：健康大盘卡片标题与共享状态文案（原为组件内硬编码中文）
    copied: "已复制",
    logsLoading: "正在读取日志流…",
    diagNodeTitle: "Node.js 运行时",
    diagPnpmTitle: "pnpm 包管理",
    diagDshTitle: "DSH 核心引擎",
    diagStorageTitle: "存储总览",
    diagNotDetected: "未检出",
    diagNoPath: "无路径",
    diagPnpmGlobalReady: "已全局就绪",
    diagPnpmMissing: "缺失",
    diagDshOfficialReady: "官方源 (已就绪)",
    diagDshOfficialMissing: "官方源 (未检出)",
    storageDistributionTitle: "DSH_HOME 存储空间分布",
    storageProfilesLabel: "Profile 工作台",
    storageSessionsLabel: "会话数据 (Sessions)",
    storageCacheOtherLabel: "系统缓存与其他",
    navLabel: "系统控制台导航",
    detailLabel: "系统控制台详情区",
    pullLatestLogs: "刷新日志",
    rereadCredentials: "刷新凭据",
    title: "系统控制台与诊断",
    subtitle: "偏好设置、模型凭据、DSH 引擎配置、崩溃自动守护、健康大盘与实时日志",
    tabPreferences: "偏好与守护",
    tabCredentials: "模型凭据",
    tabDshSettings: "DSH 引擎配置",
    tabDiagnostics: "健康大盘",
    tabLogs: "运行日志",
    // 凭据安全管理（4.5）
    credentialsTitle: "大模型与服务凭据",
    credentialsSubtitle: "安全管理 $DSH_HOME/.credentials.yaml（原子写入 + 脱敏存储）",
    configuredProviders: "已配置提供商",
    availableProviders: "可用模型提供商",
    configuredTag: "已配置",
    notConfiguredTag: "未配置",
    editKey: "配置 Key",
    deleteKey: "清除",
    keyModalTitle: (p: string) => `配置 ${p} API Key`,
    keyModalDesc: "请输入新的 API Key。保存后将原子写入 .credentials.yaml（Unix 下并维持 0600 权限）。",
    // 输入框无障碍名称（placeholder「sk-...」不足以当标签，2026-09-08 批次 B2）
    keyInputLabel: "API Key",
    keySaved: "API Key 已成功更新",
    keyRemoved: "API Key 已清除",
    keyClearConfirm: (label: string) => `确定清除「${label}」的 API Key 吗？`,
    rawYamlToggle: "切换 YAML 原文模式",
    keyMasked: "已脱敏显示",
    toggleMask: "显示明文",
    hidePlain: "脱敏隐藏",
    saveCredentials: "保存凭据配置",
    credentialsOverwriteConfirm: "确定覆盖保存 .credentials.yaml？",
    credentialsOverwritePoints: [
      "以编辑框当前内容整体重写该文件（不是增量合并）",
      "原文件会先备份为 .credentials.yaml.bak-<时间戳>",
      "写回保持原子替换（Unix 下并维持 0600 权限）",
    ],
    credentialsSaved: "凭据文件已安全保存",
    credentialsSaveFailed: "保存凭据失败",
    credentialsEmpty: "当前尚未配置任何凭据（文件未创建或为空）",
    insertTemplate: "快速插入模板",
    permHint:
      "权限保障：Unix 下此文件受 0600 文件级安全保护，仅本机当前用户可读写；Windows 下依赖用户 profile 的默认 ACL（未显式收紧）",
    // DSH 引擎全局设置
    dshSettingsTitle: "DSH 引擎全局配置",
    dshSettingsSubtitle: "管理 $DSH_HOME/settings.yaml 权威核心运行策略与全局默认参数",
    dshSettingsSaved: "DSH 设置已成功保存",
    dshSettingsSave: "保存配置",
    dshSettingsOverwriteConfirm: "确定覆盖保存 settings.yaml？",
    dshSettingsOverwritePoints: [
      "以编辑框当前内容整体重写 $DSH_HOME/settings.yaml（不是增量合并）",
      "原文件会先备份为 settings.yaml.bak-<时间戳>",
      "内容写坏会直接影响 dsh 启动——保存前建议先「刷新」比对现状",
    ],
    // 偏好与语言
    preferencesSection: "客户端偏好",
    localeLabel: "界面语言",
    localeDesc: "选择桌面壳界面的显示语言；更改后立即生效",
    // 2026-09-11（task-18）：两侧各自对「自己的语言」纯净——原值夹带英文括注
    // `(System Default)` / `(Chinese Simplified)`，zh 语境下冗余（en 侧同名键由
    // 门禁 `enUsNoLeak.test.ts` 反向覆盖）。
    localeSystem: "跟随系统语言",
    localeZh: "简体中文",
    localeEn: "English (US)",
    // 语言卡片副标题（2026-09-11 task-20；2026-09-11 task-22 修正）
    // 原为组件内硬编码。中文卡的「默认」主张已**删除**——产品默认偏好是「跟随系统」
    // （i18nStore `preference: "system"`；Rust `settings.rs` 的 `locale: Option<String>`
    // 默认 None = 跟随操作系统语言），对 navigator.language 非 zh 的用户，把简体中文
    // 标为「默认」是**谎报默认语言** ⇒ 用户可见事实失真（与「2700+」「240+」同类）。
    // 该主张无法靠改措辞救活：撇开「默认」后只剩「内置字典」这类实现细节，无用户价值。
    // 故中文卡与英文卡均无副标题，**仅系统卡保留 `localeSystemHint`**——
    // 该条为真且有用：preference === "system" 时确实经 resolveSystemLocale() 探测系统语言。
    localeSystemHint: "自动检测",
    // 下次启动的运行环境（v1.2.0 实测 1.3）：后端 `defaultMode` 早已具备
    // （`settings.rs:42`：None = 首次运行先出运行环境选择）。缺口是「**应用窗口内
    // 无入口**」——此前窗口内唯一路径是 URL 参数 `?default=1`（BootIndex.tsx）；
    // Windows 托盘菜单「打开方式」也会写 default_mode（boot.rs:777），故并非完全不可达。
    // 本组键即该窗口内入口的文案。
    // 取值语义与 Rust 一一对应：null = 每次询问；"local" / "wsl" = 直接启动。
    bootModeSection: "下次启动的运行环境",
    bootModeDesc: "选择每次打开应用时默认进入的运行环境；只影响启动方式，不改变当前正在运行的会话。",
    bootModeAsk: "每次询问",
    bootModeAskHint: "启动时先显示运行环境选择页",
    bootModeLocal: "本机运行",
    bootModeLocalHint: "直接以本机环境启动",
    bootModeWsl: "WSL2 内运行",
    bootModeWslHint: "直接以 WSL2 发行版内的环境启动",
    bootModeFallbackHint: "若本机环境初始化失败（例如 Windows 无符号链接权限），可把默认切到「WSL2 内运行」绕开该限制。",
    // 崩溃守护
    guardianSection: "高可用与崩溃守护",
    autoRestartLabel: "崩溃自动恢复守护",
    autoRestartDesc: "当检测到 DSH 会话进程意外退出时，后台自动拉起恢复；60 秒内若连续崩溃达到 3 次将自动触发熔断保护，停止重试并弹出诊断错误卡。",
    autoRestartEnabled: "已开启守护（含 3次/60s 熔断保护）",
    autoRestartDisabled: "未开启（默认手动重试）",
    // 熔断机制图解卡（2026-09-11 task-20）：原为组件内硬编码（en 用户可见中文）。
    // 数值单位随语言不同（「60 秒」vs "60s"），故标签与取值一并入字典。
    breakerTitle: "智能熔断保护协议（Circuit Breaker）",
    breakerWindowLabel: "监控窗口：",
    breakerWindowValue: "60 秒滑动窗口",
    breakerThresholdLabel: "熔断阈值：",
    breakerThresholdValue: "连续 3 次崩溃",
    breakerActionLabel: "熔断后动作：",
    breakerActionValue: "停机并弹诊断卡",
    // 悬浮胶囊与快捷键
    // 插件安装源（ADR-0006 §6，2026-09-16）：镜像/官方各有各的缺，故默认自动换源。
    pluginRegistrySection: "插件安装源",
    pluginRegistryDesc: "装插件时从哪里取包。两个源各有各的缺：镜像可能没同步新包，官方源在国内可能慢或连不上。",
    pluginRegistryAuto: "自动（推荐）",
    pluginRegistryAutoHint: "先试官方源，失败自动换另一个源；成功的源会被记住，下次直接命中。",
    pluginRegistryOfficial: "只用官方源",
    pluginRegistryOfficialHint: "registry.npmjs.org：包最全，但国内可能慢或不通。",
    pluginRegistryConfigured: "只用本机配置的源",
    pluginRegistryConfiguredHint: "沿用你 npm 配置里的源（如 npmmirror）：快，但可能缺刚发布的新包。",
    pluginRegistryOfficialShort: "官方源",
    pluginRegistryConfiguredShort: "本机配置的源",
    pluginRegistryLastGood: (name: string) => `上次可用：${name}`,
    switcherSection: "工作台快速切换与胶囊挂件",
    floatingSwitcherLabel: "工作台悬浮胶囊",
    floatingSwitcherDesc: "在 DSH 工作台顶部居中显示控制中心入口胶囊（支持鼠标自由拖拽移动；关闭后依然可通过快捷键呼出）",
    floatingSwitcherEnabled: "已开启悬浮胶囊",
    floatingSwitcherDisabled: "已关闭（仅使用快捷键）",
    shortcutLabel: "控制中心呼出快捷键",
    shortcutDesc: "在 DSH 主工作台与控制中心之间切换的快捷键（应用内生效，窗口需在前台）",
    shortcutDefault: "默认快捷键（⌘ + , / Ctrl + ,）",
    shortcutShiftP: "命令面板风格（⌘ + ⇧ + P / Ctrl + ⇧ + P）",
    saveSuccess: "设置已保存",
    saveFailed: "设置保存失败",
    // 读取失败时停用写入：settings.json 是整体覆盖写，拿不到真实基线就回写
    // 会清空其他键（2026-09-08 裁定，见 lib/shellSettings.ts）
    settingsLoadFailed: "偏好设置读取失败——为避免覆盖写坏其他配置，已停用保存",
    settingsLoading: "正在加载偏好设置…",
    retryLoad: "重试",
    diagnosticsLoadFailed: "诊断数据采集失败",
    // 诊断大盘
    diagnosticsTitle: "运行环境诊断大盘",
    diagnosticsSubtitle: "环境健康实时采集 · 真实解析路径与存储分布",
    refreshDiagnostics: "刷新体检",
    nodeCard: "Node.js 运行时",
    pnpmCard: "pnpm 全局包管理",
    dshCard: "DSH 核心引擎",
    storageCard: "存储空间分布",
    statusReady: "就绪",
    statusMissing: "未检出",
    sourceLabel: "来源",
    pathLabel: "解析路径",
    versionLabel: "版本",
    copyReport: "复制诊断报告",
    reportCopied: "诊断报告已复制到剪贴板",
    profilesUsage: (n: number, size: string) => `${n} 个 Profile · 占用 ${size}`,
    sessionsUsage: (n: number, size: string) => `${n} 个会话 · 占用 ${size}`,
    totalUsage: (size: string) => `总存储占用：${size}`,
    // 日志查看器
    logsTitle: "实时日志查看器",
    logsSubtitle: "多源日志聚合分析与排障",
    sourceShell: "DSH Dock 壳日志",
    sourceDsh: "DSH 运行时日志",
    sourceRepair: "会话自愈日志",
    searchPlaceholder: "过滤日志关键字...",
    autoScroll: "自动滚底",
    copyLogs: "复制日志",
    logsCopied: "日志已复制到剪贴板",
    clearLogs: "清屏",
    totalLines: (n: number) => `共 ${n} 行`,
    truncatedHint: "已截取最新 500 行日志",
    emptyLogs: "暂无日志内容",
    // 2026-09-18 UI 收口：原为凭据/引擎配置面板内硬编码中文，逐一入字典。
    // permMaskNote 刻意以「。」开头——既有键 permHint 按追加式纪律不改动。
    cardView: "卡片视图",
    permTitle: "文件权限与前端脱敏",
    permMaskNote: "。前端界面绝不持有全量明文 API Key，仅显示脱敏掩码。",
    keyNotConfigured: "尚未配置 API Key",
    saveApiKey: "保存 API Key",
    editKeyExisting: "修改 Key",
    copyYaml: "复制 YAML",
    reload: "重新加载",
    yamlFormat: "YAML 格式",
  },
  // 插件市场 (awesome-dsh-plugin 社区 Registry)
  // 2026-09-11 裁定：市场文案不写死插件数量——曾硬编码「2700+」而 Registry 实际
  // count 已增长，属用户可见失真。固定文案一律不内嵌数字（加载中态数量不可知，
  // 尤其不得谎报）；需展示数量的走函数入参（totalPlugins / pageInfo / categoryCount），
  // 由组件以 registry 真实数据求值。
  market: {
    clearSearch: "清空搜索",
    title: "社区插件市场",
    subtitle: "基于 awesome-dsh-plugin 官方聚合的社区插件与扩展生态",
    searchPlaceholder: "搜索插件名称、功能描述、NPM 包名或作者...",
    allCategories: "全部分类",
    // 分类选项（筛选下拉里的单项）：与市场页的分类 pill 同一套「标签 + 计数」形态
    // （2026-09-21 维护者真机："全部分类选项要与插件中心那边的分类保持一致"）。
    categoryOption: (label: string, n: number) => `${label}（${n}）`,
    categoryCount: (n: number) => `${n} 个分类`,
    totalPlugins: (n: number) => `${n} 款插件`,
    sortStars: "最多星标",
    sortDownloads: "最高下载",
    sortNewest: "最新上架",
    sortName: "名称排序",
    filterInstalled: "仅看已装",
    filterAll: "全部插件",
    installBtn: "安装",
    reinstallBtn: "重新安装",
    installToBtn: (prof: string) => `安装到 ${prof}`,
    distributeBtn: "分发",
    installedIn: (count: number) => `已装于 ${count} 个 Profile`,
    installedBadge: "已安装",
    installedInProfile: (prof: string) => `已安装在 ${prof}`,
    installedWillOverwrite: "已在此 Profile 安装（将执行覆盖/重装）",
    installedWillReinstall: "已在此 Profile 安装（将覆盖重装）",
    notInstalled: "未安装",
    noDescription: "暂无描述",
    officialCoreTitle: "DSH 官方核心插件",
    // 实验能力开关（2026-09-16，ADR-0020 §7：目录行 → 能力开关）
    // 文案纪律（2026-09-17 v3 版面重做后）：左栏**清单行**只出现"扫一眼就要知道"的东西
    // （名称 / 状态 / 当前插件名 / 开关）；价值、插件清单、前置、钉版本、行 id 全在右栏
    // 常驻的**详情面**里——那里不再需要"展开"，所以这些不再是"折叠区文案"。
    capTitle: "实验能力",
    // 2026-09-17 维护者裁定「实验功能模块要突出是 dsh 官方实验功能，插件的名字和描述也要
    // 以官方为主」：面板与每张卡都带「DSH 官方」标记，说明来源与我们的角色；包名 = 官方名，
    // 包简介 = 包自己 package.json 的 description（装好后读本地文件，不转述不翻译）。
    capOfficialBadge: "DSH 官方",
    capOfficialNote:
      "这些是 DeepSeek 官方发布的 dsh 实验功能（@deepseek-ai/* 包）：dsh-dock 只做策展与开关，不是社区插件。",
    // 两栏（清单 / 详情面）：左栏标题、右栏无障碍名、窄窗口下钻的返回。
    capListLabel: "能力清单",
    capPaneLabel: (name: string) => `${name}的详情`,
    capPluginsLabel: "插件",
    // 「切换后端」溢出菜单（v4，左栏行尾）：触发钮的可访问名 + 菜单标题。
    capMenuBackendLabel: "选择后端",
    capBackendExclusiveNote: "同能力的后端互斥，同一时刻只生效一个：切换即让位",
    // 菜单项里标记"这个正在生效"（点它 = 无操作，菜单项本就禁用）。
    capVariantActive: "当前生效",
    capSwitchToThis: "切换到此档",
    // 详情面「插件」节：后端以插件名逐行列出；共同前置与共用基座包只讲一次。
    capBasePackages: (pkgs: string) => `各档共用基座包 ${pkgs}`,
    capAlsoInstalls: (pkgs: string) => `另装共用包 ${pkgs}`,
    capDesc: "DeepSeek 官方以实验包形式发布的 dsh 能力：开启即安装并挂载，随时可以关掉。",
    // 能力说明的单条悬浮文本（summary = 这是什么；unlocks = 开启后多出什么）。
    // 2026-09-21 三次修订：两者原先平铺在展开体里（一段正文 + 一节带标题的正文），
    // 把"选后端"这件正事挤到下面；现合并成头部 ⓘ 里的一条。
    capTipText: (summary: string, unlocks: string) => `${summary} 开启后：${unlocks}`,
    capLoadFailed: "读取实验能力失败",
    capReload: "重试",
    capSummary: (on: number, total: number) => `已启用 ${on} / ${total} 项`,
    capSummaryNeedsWork: (n: number) => `${n} 项需要处理`,
    // 「已停用」是**用户自己关的**（关而不卸），不是需要处理的问题——单列一档说明。
    capSummaryDisabled: (n: number) => `${n} 项已停用`,
    capSummaryOff: (n: number) => `${n} 项未启用`,
    capRestartHint: "配置已变更：重启该 Profile 后才会生效",
    capRestartNow: "立即重启",
    capStateOn: "已启用",
    capStateOff: "未启用",
    capStateDisabled: "已停用",
    capStatePartial: "需要修复",
    capStateConflict: "后端冲突",
    capSwitchLabel: (name: string) => `${name}的开关`,
    capOtherActive: (label: string) => `当前生效的是「${label}」；开启本插件会先替换它`,
    capOtherReady: (label: string) => `当前已就位的是「${label}」（已停用）；开启本插件会先拆掉它`,
    capOffIsRemove: "该能力由 profile 层提供：关闭即移除",
    capStateSubsumed: "已包含",
    capSubsumedBy: (label: string) =>
      `本插件是「${label}」的基础层，已随之就位；开关与移除统一由「${label}」控制`,
    capRunning: (index: number, total: number) => `正在处理（第 ${index}/${total} 步）`,
    capOpInstall: (pkg: string) => `安装 ${pkg}`,
    capOpRemove: (pkg: string) => `移除 ${pkg}`,
    capOpEnsureRow: (pkg: string) => `写入 ${pkg} 的配置行`,
    capOpDeleteRow: (pkg: string) => `清理 ${pkg} 的配置行`,
    capOpDisableRow: "停用配置行",
    capOpEnableRow: "启用配置行",
    capFailed: "没有全部完成",
    capFailNetwork: "两个包源都没连上（网络抖动、代理或超时）。检查网络后重试即可。",
    capFailNotFound: "两个包源上都没有这个包或这个版本——确认包名/版本号，或稍后重试（镜像同步有延迟）。",
    capFailBuildApproval: "pnpm 拦下了构建脚本：需要在 profile 的 pnpm-workspace.yaml 里批准后重试。",
    capFailUnknown: "安装失败，展开原始输出看具体原因。",
    capFailRawToggle: "原始输出",
    capResume: "继续剩余步骤",
    capQueueHint: "包下载与失败重试见「下载管理」",
    capRepair: "修复",
    capReadyToEnable: "已就位，可直接开启",
    capPinned: (spec: string) => `钉版本 ${spec}`,
    capRemoveBtn: "移除并卸载",
    capRemoveExplainedSoft: "关闭只停用配置行、保留已下载的包；彻底移除才卸载。",
    capRemoveExplainedLayer: "该能力由 profile 层提供，关闭即移除（会一并清理层列表）。",
    capCancel: "取消",
    capConfirmTitle: (name: string) => `开启「${name}」？`,
    capConfirmNote: "将安装下列包并写入该 Profile 的配置，可能需要几分钟。",
    capConfirmStart: "开始启用",
    capNoPrereq: "没有额外前置条件",
    capReplaceFrom: (label: string) => `会先移除当前生效的「${label}」后端（卸载其包并清理配置行），且不会自动装回`,
    capRemoveTitle: (name: string) => `移除「${name}」？`,
    capRemoveNote: "这是不可撤销的破坏性操作。",
    capRemoveNoteLayer: "该能力由 profile 层提供：关闭它必须移除该层，包会被卸载。",
    capRemoveConfirm: "移除",
    capRemovePointPackages: (n: number) => `卸载 ${n} 个包（重新启用需要重新下载）`,
    capReplaceDisplaced: (pkgs: string) =>
      `会先移除已就位的后端包：${pkgs}（卸载后不会自动装回）`,
    capRemovePointRows: "删除 dsh-dock 写入的配置行（不清会留下指向空包的悬空行）",
    capRemovePointLayer: "从 profile 的层列表里移除该层",
    capDone: "已生效",
    // 标点与整句都进字典：组件里不许留全角标点（2026-09-17 独立复核实测：
    // 切到 English 会看到 `Could not read experimental capabilities：…` 这类中文标点）。
    capValueSep: "：",
    capWhyJoin: "；",
    capDoneFor: (name: string) => `${name}：已生效`,
    capPartialFor: (name: string, done: number, total: number) =>
      `${name}：已完成 ${done}/${total}，可继续剩余步骤`,
    capFailedOn: (name: string) => `「${name}」失败`,
    capRowHasFailure: "这一项有还没处理的失败，原因在右侧详情面里",
    capPartial: (done: number, total: number) => `已完成 ${done}/${total}，可继续剩余步骤`,
    installModalTitle: (pkg: string) => `安装插件「${pkg}」`,
    installModalDesc: "选择要安装的目标 Profile。安装后将自动写入该 Profile 的 package.json 并通过 pnpm 自动构建安装。",
    selectProfile: "选择目标 Profile",
    installSpecLabel: "安装源",
    sourceNpm: "NPM 官方包",
    sourceGithub: "GitHub 仓库",
    sourceAutoDetected: "已自动识别规范",
    installingBusy: "正在通过 pnpm 安装插件，可能需要几十秒…",
    installSuccess: (pkg: string, prof: string) => `插件「${pkg}」已成功安装至 Profile「${prof}」！重启该 Profile 后生效。`,
    installFailed: (msg: string) => `安装失败：${msg}`,
    viewReadme: "GitHub",
    viewNpm: "NPM",
    openOfficialDoc: "主页",
    loadingRegistry: "正在连接社区 Registry 加载插件目录…",
    loadFailed: "加载插件市场目录失败",
    retry: "重试",
    noResults: "未找到符合条件的插件",
    noResultsHint: "尝试更换搜索词或清除分类筛选条件",
    clearFilters: "清除所有筛选条件",
    paginationPrev: "上一页",
    paginationNext: "下一页",
    pageInfo: (current: number, total: number, count: number) => `第 ${current} / ${total} 页（共 ${count} 款）`,
    pageSize: "每页",
    cacheHit: "本地缓存命中",
    refreshRegistry: "刷新市场",
    loadingBtn: "加载中…",
    openLinkFailed: (msg: string) => `打开链接失败: ${msg}`,
    author: "作者",
    addedDate: "上架时间",
    subtabMarket: "插件市场",
    subtabInstalled: "已安装",
    expandCategories: (n: number) => `展开全部 (${n})`,
    collapseCategories: "收起分类",
    manualInstallBtn: "手动安装",
    manualInstallTitle: "手动安装插件",
    manualInstallDesc: "输入 NPM 包名、GitHub 仓库地址或 Tarball 规范，并选择要安装到的目标 Profile。",
    manualInstallSpecPlaceholder: "例如：@deepseek-ai/dsh-pet 或 github:user/repo",
    manualInstallSubmit: "安装到所选 Profile",
    // 下载管理队列（095 #4 / ADR-0011 队列形态：审批门内联审核）
    installQueued: (pkg: string, prof: string) => `已加入下载队列：「${pkg}」→ ${prof}`,
    // 2026-09-15（R2）：卸载也走同一队列——同族 provider 让位等场景要能在面板
    // 看见、失败能重试；文案单独给，不借安装文案。
    queueRemoveQueued: (pkg: string, prof: string) => `已加入下载队列：卸载「${pkg}」← ${prof}`,
    queueRemoveDone: (pkg: string, prof: string) => `已从 Profile「${prof}」卸载「${pkg}」`,
    queueTitle: "下载管理",
    queueEmpty: "暂无下载任务",
    queueClearDone: "清除已完成",
    queueStatusQueued: "排队中",
    queueStatusInstalling: "安装中",
    queueStatusDone: "已安装",
    queueStatusRemoved: "已卸载",
    queueStatusFailed: "失败",
    queueDismiss: "放弃",
    queueRetry: "重试",
    queueFailedNotice: (pkg: string, detail: string) => `「${pkg}」安装失败：${detail}`,
    queueRemoveFailedNotice: (pkg: string, detail: string) => `「${pkg}」卸载失败：${detail}`,
  },
  // 破坏性操作确认对话框的通用文案（2026-09-08，U9）
  confirm: {
    cancel: "取消",
  },
  // 解释性悬浮（ui/info-tip.tsx，2026-09-19）：图标按钮没有可见文字，名称只能靠 aria。
  // 同一屏会有十几个「查看说明」，所以带上它说明的是**哪一项**（读屏 tab 过去能对上）。
  tip: {
    aria: "查看该项说明",
    ariaFor: (subject: string) => `查看「${subject}」的说明`,
  },
} as const

type RecursiveString<T> = {
  [K in keyof T]: T[K] extends (...args: infer A) => unknown
    ? (...args: A) => string
    : T[K] extends readonly (infer E)[]
      ? E extends object
        ? readonly RecursiveString<E>[]
        : readonly string[]
      : T[K] extends object
        ? RecursiveString<T[K]>
        : string
}

export type AppCopy = RecursiveString<typeof t>
