# ADR 索引

> **定位**：本仓库全部架构决策记录（ADR）的索引表。
> **来源**：本表原为 `AGENTS.md` §9 的正文。该表随 ADR 数量**线性增长**，而 §11.4 对
> `AGENTS.md` 设有「全文 ≤ 250 行」的固定预算——2026-09-15 时 `AGENTS.md` 恰为 250 行，
> **连补一行 ADR-0019 索引都会越预算**，属结构性冲突。故按 §11.4 的「回收」机制把索引
> 整体迁出到本文件，`AGENTS.md` §9 只保留「必须先立 ADR」的规则与指向本文件的指针。
> **「必须先立 ADR 再动代码」这条规则仍在 `AGENTS.md` §9**，本文件只承载索引，不承载规则。
> **状态以各 ADR 文件头部为准**（本表不复制状态，避免双源）：`docs/adr/` 模板见
> [`TEMPLATE.md`](./TEMPLATE.md)。

## 索引

| ADR | 一行结论 |
|:---|:---|
| [0001](0001-ready-wait-process-liveness.md) | 就绪等待 = 进程存活感知，非死等 |
| [0002](0002-webview-memory-policy.md) | WebView 长会话内存 = 注入 CSS 缓解 |
| [0003](0003-external-link-and-navigation.md) | 外链 = 系统浏览器兜底 + 白名单拦截 |
| [0004](0004-wsl-guest-dsh-install.md) | WSL 客体内安装，Windows 侧壳不触网 |
| [0005](0005-pnpm-global-bin-dir.md) | pnpm 需注入 global-bin-dir，失败回退 npm |
| [0006](0006-network-surface-and-mirror-chain.md) | 唯一网络面 + 镜像链 + 下载双超时 |
| [0007](0007-update-entry-menu-vs-tray.md) | 更新入口 macOS=菜单 / 非 macOS=托盘 |
| [0008](0008-frontend-framework.md) | React 生态白名单与前端三红线 |
| [0009](0009-profile-manager.md) | Profile 生命周期：创建走 dsh plugin 转发链，其余文件层；pnpm boot 硬依赖 |
| [0010](0010-engine-inversion.md) | 运行时归属倒置：引擎=壳资产（pnpm12 引导），探测层退役；升级全显式、离线可启动 |
| [0011](0011-plugin-management-boundary.md) | 插件职责边界：跨 profile 归插件中心、单 profile 归详情；安装来源三形态白名单（npm / github / tarball），更新检查保持严格 npm 判别 |
| [0012](0012-typed-boot-failure.md) | 启动失败错误类型化：boot 路径引入 `BootFailure` 枚举，子串分类降级为 `from_legacy_detail` 兜底；其余模块 `Result<_, String>` 不动 |
| [0013](0013-default-build-approval.md) | 构建脚本默认批准：profile 级 `dangerouslyAllowAllBuilds`，审批门解析/逐包裁决链退役 |
| [0014](0014-restart-handoff-continuity.md) | 重启/切换交接带：交接意图贯穿两窗 + 启动代际闸门 + 会话槽先收后落 + Windows 进程树收口 |
| [0015](0015-child-process-lifecycle-ownership.md) | 子进程生命周期归属：硬杀收口（unix 生命线 watcher / Windows Job Object）+ 启动期基于内核锁的孤儿清扫；spawn 收敛到 `lifecycle` 单点 seam |
| [0016](0016-wsl-guest-management-plane.md) | 管理面下沉 WSL 客体：控制中心跨环境一致（读/写/原语双侧同构，按运行模式择源） |
| [0017](0017-dsh-project-local-install.md) | dsh 改为 project 内安装 + 自建 shim：结构性绕开 pnpm 全局 hash 符号链接，Windows 普通账户免提权 |
| [0018](0018-dsh-module-proxy-mode-on-windows.md) | Windows 以「模块代理」模式启动 dsh：绕开 dsh 启动期建 481 个符号链接所需特权（ADR-0017 的续篇） |
| [0019](0019-dsh-client-module-proxies-on-windows.md) | Windows 模块代理模式下补齐浏览器客户端模块（client bundles 与 client 声明）（ADR-0018 的续篇） |
| [0020](0020-official-plugin-catalog-and-activation.md) | 官方插件目录与安装激活契约：按目标包是否声明 `dsh.bundle` 分支——声明者由 `dsh plugin add` 自动进 bundle 层栈、壳不写 patch；未声明者须经 `PatchFile` 写 `- insert:` 挂载行。版本钉死、同族 provider 强制「替换」、目录为 dsh-dock 策展集 |
| [0021](0021-archived-session-unarchive-route.md) | 已归档会话「取消归档」走 Host RPC（既有 typert 回环），不触碰 `$DSH_HOME/storages/workspace.json`；壳内入口理由经维护者认可，路线 A 成立 |
| [0022](0022-mcp-probe-network-face.md) | MCP 连通性与能力探测的网络面按 `transport` 分支：`stdio` = `lifecycle` 子进程 + `Kind::Registered`；`streamable-http` = 条目级 `Kind::Exempt`。二者均 2s 量级超时、只读、一次性快照 |
| [0023](0023-ssh-remote-workspace-scope.md) | SSH 远程工作区限 headless / 自建 profile（四包组合、`~/.ssh/config` 后端解析、`BatchMode` 预检）；**明确不为 `web` profile 承诺远端感知视图** |
| [0024](0024-desktop-quick-task-runner.md) | 桌面任务快跑器：OS 全局热键 + 轻量窗口，壳不解析 `--json`、职责限「任务转交」。（**2026-09-15 维护者裁定「暂时不做」——未否决，当前批次不实施，`roadmap` §5 边界问题留白**） |

## 维护约定

- 新增 ADR 时，**在本表末尾追加一行**（`| [NNNN](NNNN-kebab.md) | 一行结论 |`）。
- 编号连续、四位零填充；文件名 `NNNN-kebab-case.md`。
- 索引行只写「一行结论」，**不复制状态、不复制决策推理**（推理在 ADR 正文；状态以 ADR 头部为准）。
  例外：已生效的**维护者裁定**可随结论一并记入（形如「（YYYY-MM-DD 维护者裁定 …）」），因为它是
  决策内容本身而非状态镜像——尤其用于标记**暂缓/否决**类条目，避免索引被误读为可实施的待办。
- 索引的增删改与 ADR 文件的增删改**须在同一提交**内完成，避免出现「ADR 存在但无索引」（ADR-0019 曾因此漏登）或「索引指向不存在的文件」。
