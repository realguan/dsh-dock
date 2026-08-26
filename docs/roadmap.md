# DSH Dock 产品路线图

> 状态：活文档，随阶段退出、关键数据更新、资源变化或风险暴露时重排。
> 最后更新：2026-08-26（v0.4.6 发布后重排）
> 适用版本：v0.4.6 起

## 1. 背景与定位

DSH Dock 是 dsh（@deepseek-ai/dsh）的桌面终端——一个极小的 Tauri v2 壳，把 dsh 工作台以独立、可安装、跨平台的桌面应用呈现。

**壳的核心价值主张**：壳是通用机制，产品是数据。壳不感知任何具体产品身份，运行时身份只经 `product.manifest.json` 进入。

本路线图覆盖两类方向：
- **壳自身能力扩展**：执行环境、工程化、平台集成、稳定性
- **dsh 管理可视化**：基于 dsh 的文件驱动架构（`$DSH_HOME` 下的 profile / settings / 会话 / 凭据等全部是文件），壳在不修改 dsh 源码的前提下，提供全局、跨 profile、离线可用的管理能力

### dsh 架构关键事实（管理扩展的可行性基础）

经源码探索确认（dsh v0.1.1-rc.2，197 个 `@deepseek-ai/dsh-*` 子包）：

| 事实 | 对壳的意义 |
|:---|:---|
| 所有状态在 `$DSH_HOME` 下以文件存储 | 壳可纯文件读写完成管理，不需 dsh 运行时 |
| 每个 profile = 独立 pnpm workspace（`package.json` + `cordis.patch.yml` + `node_modules/`） | 壳可创建/复制/删除 profile，可解析插件列表 |
| `dsh plugin --profile <name> add/remove/update` CLI 已存在 | 壳封装此 CLI 即可做插件管理 UI |
| `dsh --profile <name> --dump-config` 输出完整组合配置树 | 壳可展示配置来源分层 |
| `settings.yaml` 支持 chokidar 热重载（100ms 防抖） | 壳修改设置后 dsh 自动生效，无需重启 |
| Cordis 插件行可按 `id` 覆盖 config 或 `disabled: true` | 壳可启用/禁用插件，可编辑插件配置 |
| dsh 内部有插件/设置 UI，但局限于当前运行的 profile | 壳的独特价值 = 全局 / 跨 profile / 离线管理 |

---

## 2. 排序原则与硬约束

### 排序问题定义

- **排序对象**：DSH Dock 下一阶段所有候选功能方向
- **要优化的结果**：壳的用户价值（管理便捷性、稳定性、跨平台覆盖）+ 开发者价值（工程化、可维护性）
- **时间表达**：Now / Next / Later（顺序明确，不指定具体日期——无真实资源承诺时不凭空分配工期）
- **最终决策者**：项目维护者

### 硬约束（不可违反，不参与普通功能评分）

1. **不修改 dsh 源码**：所有管理扩展通过文件读写 + `dsh` CLI 调用实现
2. **壳保持薄**：不引入前端框架 / 构建器 / 数据库 / IPC 总线 / 领域服务
3. **最小面原则**：新增 IPC 命令须三处同步（build.rs + capabilities/default.json + lib.rs）并在 AGENTS §7 登记
4. **无状态库**：不引入新的核心态持久化（现有 `settings.json` 的 `defaultMode` 是唯一例外）
5. **增量生成**：一次会话只做一个明确意图，不做跨模块批量改动
6. ~~WSL 实机验证是已写代码的未验证风险~~ → **已完成**（v0.4.6，WSL 哨兵文件方案验证通过，见下方「已完成」章节）

### 优先级判断维度

- **用户价值**：解决什么痛点、影响多少用户、使用频率
- **壳的独特性**：dsh 内部 UI 能否做到？壳是否提供了 dsh 做不到的全局/跨 profile/离线能力？
- **依赖关系**：是否依赖其他功能先完成？是否是其他功能的前置？
- **实现成本**：代码量、测试难度、跨平台复杂度
- **风险**：稳定性风险、安全风险、与 dsh 版本耦合风险

---

## 3. 已完成（v0.4.6 及之前）

### WSL Windows 实机验证闭环 ✅

| 字段 | 内容 |
|:---|:---|
| **完成版本** | v0.4.6（commit 542ba54 + 327f753） |
| **验证结果** | 用户实机启动 WSL Ubuntu-24.04 dsh web 触发并验证通过 |
| **发现的问题** | wsl.exe 把客体内 dsh 的 stdout/stderr 转发到 Windows 侧时存在内部缓冲——直到 wsl.exe 退出才 flush，导致壳轮询日志 90s 内读空，BOOT_TIMEOUT 触发 Stalled |
| **修复方案** | 哨兵文件绕开 wsl.exe 输出缓冲：GUEST_BOOT 用 `tee` 镜像 dsh 输出到客体内 `/tmp/dsh-dock-ready`（tee 行缓冲，无 wsl.exe 介入，实时可读）；Executor trait 新增 `read_ready_marker()`；WslExecutor 经 `wsl.exe -e cat` 读取哨兵文件；`wait_for_ready` 加 marker 闭包，每轮 marker 优先命中 |
| **附带修复** | UTF-16LE 日志解码（9a52a1e）、WSL 仅 Windows 非 Windows 零感知（549367b）、nvm/fnm PATH 兼容（5dc34d6）、缺 dsh 自动安装（5dc34d6） |
| **测试** | shell.rs 新增 2 条 marker 优先级测试 + 3 条旧测试加 marker 占位；cargo test 全绿 |
| **遗留** | teardown 的 `wsl --terminate` 兜底经实机验证不需要（stop 标志文件 + wsl.exe 退出已足够干净） |

---

## 4. Roadmap

### Now — 夯实基础（当前阶段）

> 阶段目标：建立工程化基线，补齐测试覆盖，为后续大规模管理功能开发铺路。WSL 实机验证已完成（见上方「已完成」）。

#### 4.1 工程化基线（rustfmt + clippy + 覆盖率）

| 字段 | 内容 |
|:---|:---|
| **目标结果** | CI 自动检查代码格式和 lint，覆盖率有基线数据 |
| **当前问题** | AGENTS.md 标注 `[待补充]`：无 `rustfmt.toml`/`clippy.toml`，CI 只跑 `cargo test`。后续管理功能会新增大量代码，没有自动化基线会积累技术债 |
| **关键行动** | 提交锁定 edition 2021 风格的 `rustfmt.toml`（先跑一次 `cargo fmt` 确认 diff 可控，遵循「不引入全仓格式化 diff」原则）；CI 加 `cargo fmt --check` + `cargo clippy -D warnings`（先本地修掉现有 warning 再接闸门）；接入 `cargo-llvm-cov` 出基线报告（先出数，不定阈值） |
| **依赖** | 无 |
| **结果信号** | CI 新增检查步骤全绿；覆盖率基线数据产出 |
| **退出条件** | CI 三平台 fmt/clippy/test 全绿，覆盖率基线报告可查 |
| **重排触发器** | 若 `cargo fmt` 产生不可控的全仓 diff，暂停并评估是否需要分阶段格式化 |

#### 4.2 updater.rs 测试补充

| 字段 | 内容 |
|:---|:---|
| **目标结果** | 自更新模块有基础回归保护 |
| **当前问题** | `updater.rs` 是当前唯一零测试的模块，且涉及「安装前停 dsh → 安装 → 重启」的跨平台分叉逻辑（Windows `exit(0)` 跳过 `RunEvent::Exit` 的孤儿风险是刻意处理的），没有回归保护 |
| **关键行动** | `ClientUpdate` 状态机序列化/反序列化测试（纯函数）；`set_state` 事件目标窗口过滤逻辑（只发 main/about，不发 remote dsh 页面）；跨平台分叉编译期测试；真实 download/install 路径走手动验证（AGENTS 约定），不强行单测 |
| **依赖** | 无 |
| **结果信号** | updater.rs 新增测试全绿 |
| **退出条件** | 状态机和事件过滤逻辑有测试覆盖 |
| **重排触发器** | 无 |

---

### Next — 核心能力扩展（下一阶段）

> 阶段目标：交付壳的独特价值——全局、跨 profile、离线可用的 dsh 管理能力。Profile 管理是入口，插件管理是高频操作。

#### 4.3 Profile 管理器（壳的独特定位，最高用户价值）

| 字段 | 内容 |
|:---|:---|
| **目标结果** | 用户可在壳内可视化管理所有 dsh profile：列出、创建、复制、重命名、删除、切换默认启动 profile、查看详情 |
| **当前问题** | profile 是 dsh 的核心组织单元（每个 profile = 独立插件组合 + 配置 + 数据），但创建/复制/切换目前只能手动操作目录。dsh 内部 UI 只能管当前运行的 profile，且 dsh 未启动时无法操作。壳提供全局视角是独特价值 |
| **关键行动** | ① 列出 profile：扫描 `$DSH_HOME/profiles/`，解析每个 profile 的 `package.json`（bundles + dependencies）、`cordis.patch.yml`、依赖状态、最后使用时间；② 创建 profile：从模板（dsh 内置 agent-presets：code/minimal/standard/cordis）或从现有 profile 复制初始化 pnpm workspace；③ 复制/重命名/删除：安全文件操作（删除前确认，重命名处理 pnpm workspace 引用）；④ 切换默认启动 profile：当前壳硬编码 `--profile web`，改为可配置并持久化到 `settings.json`；⑤ Profile 详情页：展示 `package.json`、`cordis.patch.yml`、`dsh --dump-config` 完整组合配置树（标注每层来源）、依赖完整性检查 |
| **依赖** | Now 阶段的工程化基线（新增大量代码需要 fmt/clippy 闸门）；需要新增 `serde_yaml` 依赖（当前只有 serde_json）用于 YAML 读写 |
| **结果信号** | 用户可在壳内完成 profile 的全生命周期管理，无需手动操作文件目录；默认启动 profile 可切换 |
| **退出条件** | 列出/创建/复制/删除/切换默认/详情六个核心能力可用，文件操作有纯函数单测覆盖 |
| **重排触发器** | 若 dsh 未来版本改变 profile 目录结构或 pnpm workspace 布局，需适配；若 dsh 内部新增了全局 profile 管理 UI，需重新评估壳的增量价值 |

**为什么先做 Profile 而不是 Plugin**：Profile 是其他管理功能的组织入口——插件管理、设置编辑、MCP 配置都依附于某个 profile。先建立 profile 选择/切换机制，后续功能才能复用。且创建/复制 profile 是 dsh 内部 UI 完全做不到的（需要操作文件系统），壳的独特性最强。

#### 4.4 插件管理器（跨 profile 操作是壳的独特能力）

| 字段 | 内容 |
|:---|:---|
| **目标结果** | 用户可在壳内管理指定 profile 的插件：列出、安装、卸载、更新、启用/禁用、跨 profile 复制、npm 搜索 |
| **当前问题** | dsh 内部有插件管理 UI（`dsh-client-ui-settings-plugins`），但只能管当前运行的 profile。壳可以：在 profile 未运行时管理、跨 profile 复制插件配置、批量操作多个 profile。启用/禁用插件（通过 `cordis.patch.yml` 的 `disabled: true`）是壳的独特能力——dsh 内部 UI 可能不支持禁用任意插件 |
| **关键行动** | ① 列出插件：从 profile 的 `package.json` 解析 `dependencies` + `dsh.profile.bundles`，区分官方 bundle（`@deepseek-ai/dsh-*`）vs 第三方插件，从 `node_modules/<pkg>/package.json` 读取版本/描述；运行时状态（active/loading/failed）需 dsh 运行时通过 Typert RPC 获取，未运行时只显示静态信息；② 安装/卸载/更新：封装 `dsh plugin --profile <name> add/remove/update <pkg>` CLI，显示进度；③ 启用/禁用：修改 `cordis.patch.yml` 添加/移除 `{id: "<plugin-id>", disabled: true}`，需要 `--dump-config` 获取所有插件行的 id 列表；④ 跨 profile 复制：把 profile A 的某个插件配置（package.json 依赖 + cordis.patch.yml 覆盖）复制到 profile B；⑤ npm 搜索：调用 npm registry API 搜索 `dsh-` 前缀包，一键安装 |
| **依赖** | Profile 管理器（3.4）——需要先选 profile 再管插件；`serde_yaml` 依赖 |
| **结果信号** | 用户可在壳内完成插件的安装/卸载/启用禁用/跨 profile 复制，无需手动编辑 `package.json` 或 `cordis.patch.yml` |
| **退出条件** | 列出/安装/卸载/启用禁用/跨 profile 复制五个核心能力可用，CLI 调用有错误处理和进度反馈 |
| **重排触发器** | 若 dsh 未来版本改变插件配置格式（如从 cordis.patch.yml 迁移到其他格式），需适配；若 `dsh plugin` CLI 接口变化，需更新封装 |

---

### Later — 体验完善与能力延伸（后续阶段）

> 以下方向价值明确但优先级低于 Now/Next，或依赖 Now/Next 的产出，或用户覆盖面较窄。按主题分组，组内不严格排序——进入时根据当时的用户反馈和资源情况重排。

#### 4.5 设置可视化编辑器

- **目标**：可视化编辑 `settings.yaml`（LLM 提供商、默认模型、主题、语言、对话设置等）+ `.credentials.yaml`（API keys 脱敏管理 + 引用检查），利用 dsh 热重载实现修改即生效
- **壳的独特性**：dsh 内部设置 UI 分散在各命名空间页面，壳提供统一全局视图；dsh 未启动时也能编辑
- **依赖**：Profile 管理器（3.4）；`serde_yaml`；需要理解各命名空间的设置 schema（从 dsh 的 schemestry schema 或 TypeScript 类型推断）
- **风险**：设置 schema 随 dsh 版本变化，需要保持兼容；`.credentials.yaml` 包含敏感信息，查看时必须脱敏

#### 4.6 会话与工作区管理器

- **目标**：列出所有会话（按项目路径分组）、恢复/删除会话、工作区增删管理
- **壳的独特性**：dsh 未启动时也能浏览会话；跨工作区全局视图
- **依赖**：无强依赖
- **注意**：dsh 内部已有会话列表 UI，壳的增量价值相对较小；会话内容是 zstd 压缩的 JSONL，壳只做元数据层面的管理，不解析完整内容

#### 4.7 MCP 服务器管理器

- **目标**：管理 `cordis.patch.yml` 中的 MCP 服务器配置（增删改查），查看 MCP 工具列表和连接状态
- **壳的独特性**：MCP 配置在 cordis.patch.yml 底层，dsh 内部可能有管理 UI 但壳提供更底层的配置编辑
- **依赖**：Profile 管理器（3.4）；需要理解 `dsh-mcp-client` 的配置 schema
- **注意**：MCP 服务器配置格式需从 dsh 源码确认（每个 MCP 服务器是一个 `mcp__<serverName>__` 插件行实例）

#### 4.8 SSH 远程执行器

- **目标**：实现 executor 抽象的 SSH 变体（`SshConfig` 形状已预留），系统 `ssh` 子进程 + 端口隧道，TCP 健康探测定就绪
- **壳的独特性**：executor 抽象的自然完成，让 dsh 可以运行在远程服务器上
- **依赖**：无强依赖；但需要设计会话级 capability 收敛（远端会话拒绝 upgrade 类动作，`docs/executor.md` 已标注安全边界）
- **注意**：用户覆盖面较窄（需要远程服务器）；SSH 配置管理（保存 host/user/port）属于设置扩展，需要配置面板

#### 4.9 WSL 迭代 v2

- **目标**：WSL 多发行版选择 UI、profile 选择、teardown 兜底、缺 node 一键安装引导
- **依赖**：Now 阶段的 WSL 实机验证（3.1）——必须先验证 v1 再扩展 v2
- **注意**：缺 node 不自动安装是刻意设计（用户主权），v2 最多做到「一键调起包管理器安装」，不替用户选版本；客体内 npm 镜像参数不注入（尊重用户客体内 npm 配置）

#### 4.10 代理支持（企业网络环境）

- **目标**：`updates.rs` 网络面支持 HTTP/HTTPS 代理（从环境变量或 settings 读取），错误卡给「检查代理配置」提示
- **依赖**：无强依赖
- **注意**：只在 `updates.rs` 网络面加代理，其他模块不触网（AGENTS 网络面白名单纪律）；WSL 客体内的 npm 代理不注入

#### 4.11 诊断与维护工具

- **目标**：环境诊断（Node/dsh 版本、DSH_HOME 路径、磁盘占用）、profile 完整性检查、重置 profile 依赖（删 node_modules 重装）、日志查看
- **依赖**：Profile 管理器（3.4）
- **注意**：辅助功能，价值在于降低支持成本和用户自助排障

#### 4.12 崩溃自动恢复（可选开关）

- **目标**：dsh 意外崩溃后自动重启（连续崩溃 N 次后停止并出错误卡），默认关闭
- **依赖**：无强依赖
- **注意**：可选开关，默认行为不变（手动重试）；符合「最小持久化例外」的扩展节奏（settings.json 新增 `auto_restart` 字段）

#### 4.13 多语言 / i18n

- **目标**：壳自带 UI（index/mode/selector/about 四页 + 管理页面）支持中英文，优先系统语言，settings 可手动覆盖
- **依赖**：管理页面（Next 阶段）完成后统一做 i18n 更高效
- **注意**：轻量方案（JSON 语言包 + 前端 `t()` 函数，不引入框架）；Rust 侧错误信息也需语言包

#### 4.14 平台工程（macOS 签名公证 / Linux 桌面集成）

- **目标**：macOS 正式 Developer ID 签名 + 公证（当前 ad-hoc，用户首次打开被 Gatekeeper 拦截）；Linux `.desktop` 文件 / 图标主题 / MIME 关联验证
- **依赖**：需要 Apple Developer 证书（macOS）；无代码依赖
- **注意**：发布质量改进，不影响功能；Windows 代码签名（EV 证书）成本更高，后续单独评估

---

## 5. 不做清单（明确后置或不做的方向）

以下方向经评估**不属于壳的职责范围**，或**违反壳的定位约束**，明确不做：

| 方向 | 不做原因 |
|:---|:---|
| 修改 dsh 源码 | 硬约束：壳通过文件读写 + CLI 管理，不 fork/修改 dsh |
| 快照物化 / 版本 pin / 插件集成 / 本地源扫描 | 属于「装配方」（外部打包工具），经 `product.manifest.json` 契约与壳对接，本仓库不承接 |
| dsh 自身功能扩展（会话管理、插件市场、模型配置等） | 是 dsh 本体的事，壳只负责呈现和文件层面的管理 |
| 引入前端框架 / 构建器 / 数据库 | 壳保持薄，UI 是零构建静态页 |
| 引入 IPC 总线 / 领域服务分层 | 壳无 services 分层，用 anyhow 错误处理 |
| 具体产品身份硬编码（名称/图标/插件/凭据） | 壳是通用机制，产品是数据 |
| WSL 内自动安装 Node | 用户主权：安装方式/版本策略属用户决定，壳只给可行动提示 |
| WSL 客体内 npm 镜像参数注入 | 尊重用户客体内 npm 配置 |
| 离线档快照内容生成 | `render-product.sh` 是打包期工具，壳运行时不执行 |
| dsh 会话内容全文搜索 | dsh 有 `dsh-session-query-sqlite` 模块，壳只做元数据管理，不解析会话内容 |

---

## 6. 定期重排机制

本路线图是活文档，在以下事件发生时重排：

1. **阶段退出**：Now 阶段完成后，评估 Next 阶段的进入条件是否满足
2. **关键数据更新**：用户反馈、实机验证结果、dsh 上游版本变化
3. **资源变化**：可用开发时间增减、协作者加入/离开
4. **依赖延期**：某个被依赖的功能延期，影响后续功能的进入条件
5. **风险暴露**：某个方向在实施中发现不可预见的技术风险或架构冲突

重排时保留变更记录：改了什么、基于什么新事实、受影响的承诺、谁作出决定。

---

## 7. 与 AGENTS.md 的关系

本路线图中的所有方向均须遵循 `AGENTS.md` 的编码规范：
- 增量生成：一次会话只做一个明确意图
- 必须附带测试：行为改动有对应测试或验证记录
- 先读后写：动任何文件前先读现状
- 新增 IPC 命令三处同步 + AGENTS §7 登记
- 收尾三件事：`cargo test` 绿 → `git diff` 确认无越界 → 按 CONTRIBUTING 规范提交

影响契约/架构/安全边界的决策须先立 ADR（`docs/adr/`，遵循 TEMPLATE.md），再动代码。
