# 验收清单 · 2026-09-16 批次（I1–I4 ＋ 两处修复）

> 面向维护者的**逐条**验收。每条都有编号：通过打 ✅、不通过打 ❌ 并写一句现象，
> 把编号回给我即可定位（例：「A3 ❌ 开关没灰」）。
>
> **合并记录**：`master` 的 `c3be5a1`（`merge: 合入 I1–I4 … ＋ ADR-0020~0024`），
> 来源分支 `refactor/experimental-capability-switches`（9 提交 / 80 文件 / +14796 −262），
> master 此前无分叉（共同祖先 `d89780a`）。
>
> **图例**：🤖 = 我已机器验证（附命令与结果，可抽查）· 🖐 = **需要你手点**（GUI 我跑不了）
> · 🧪 = 可选受控复现（想验这条再做，步骤含还原方法）

---

## 0. 验收环境

### 0.1 两个运行档（互不影响）

| 档 | 怎么起 | dsh home | 壳数据目录（日志在这） |
|:---|:---|:---|:---|
| **dev（本次验收用）** | 你的终端 `cargo tauri dev`（**现在正在跑**） | `~/.dsh-dock-dev` | `~/Library/Application Support/io.github.realguan.dsh-dock.dev` |
| 正式版（别动） | 已装好的 DSH Dock.app | `~/.dsh` | `~/Library/Application Support/io.github.realguan.dsh-dock` |

> dev 档当前进程：`target/debug/dsh-dock` 构建于 **14:55:11**，已包含本批全部改动；
> 前端由 vite dev server（:1420）实时编译。
> ⚠️ 合入 master 时工作区文件被重写，`cargo tauri dev` 可能已自动重编——**若卡片行为与本文不符，
> 先 Ctrl+C 重启一次 `cargo tauri dev`** 再判定。

### 0.2 入口都在「控制中心」一个窗口里（顶部四个 Tab）

| Tab | 用途 |
|:---|:---|
| **Profile 列表** | Profile 管理；右侧 `SSH 远程工作区向导` 按钮（D 组） |
| **插件中心** | 内部三个子 Tab：`插件市场` / `已安装` / **`实验能力`**（A 组） |
| **会话维护** | 会话列表；`已归档` 档位里有 `取消归档`（B 组） |
| **系统控制台** | 内部子 Tab `偏好与守护` → Preferences 的 **`插件安装源`** 区块（E 组） |

MCP 探测（C 组）在 **Profile 列表 → 选中一个 profile → 详情里的 MCP 服务器行右侧「探测」**。

### 0.3 出问题时看哪（dev 档）

| 文件 | 内容 |
|:---|:---|
| `…dsh-dock.dev/shell.log` | 壳侧全量结构化日志（启动步骤、就绪 URL、SIGKILL 等） |
| `…dsh-dock.dev/dsh-shell.log` | **dsh 子进程的 stdout+stderr**（插件树错误、退出码现场） |
| `…dsh-dock.dev/plugin-op.log` | 每次 `dsh plugin add/remove` 的完整 pnpm 输出（含 `--registry`） |
| `…dsh-dock.dev/plugin-rows.log` | 写行后 `--dump-config` 复核输出 |
| `~/.dsh-dock-dev/profiles/web/cordis.patch.yml` | 挂载行落盘位置（壳每次覆写前留 `.bak-<unix秒>`） |

### 0.4 验收中失败怎么办

把 **编号 + 一句现象 + `dsh-shell.log` 尾部（或 `shell.log` 尾 20 行）** 给我即可；
每条的「期望」列都写成了**可判真假**的句子，不需要你判断实现细节。

---

## 1. 机器已验：这些不用你手点（抽查即可）🤖

在 `master` 的 `c3be5a1` 上实跑：

| 闸门 | 命令 | 结果 |
|:---|:---|:---|
| Rust 格式 | `cd src-tauri && cargo fmt --check` | 干净 |
| Rust lint | `cargo clippy --all-targets -- -D warnings` | 0 警告 |
| Rust 测试 | `cargo test` | **533 passed / 0 failed / 8 ignored**（8 = 真机项，见 §1.5）|
| 前端类型 | `cd frontend && node node_modules/typescript/bin/tsc -b` | 0 错误 |
| 前端 lint | `pnpm run lint` | 0 警告（152 文件） |
| 前端测试 | `pnpm run test` | **440 passed / 51 文件** |
| 生产构建 | `pnpm run build` | 通过 |

**机器闸门覆盖的契约**（漏一处即红，不需要人记）：

- IPC 四处同步（`ipc.rs::COMMANDS` → `lib.rs` handler → `capabilities/default.json` → `lib/tauri.ts`），
  现 **63 条命令**（含本批新增 `unarchive_session` / `probe_mcp_server` / `list_ssh_hosts` /
  `probe_ssh_target` / `generate_ssh_profile` / `apply_official_patch_row` / `remove_official_patch_row`）；
- IPC 结构体形状 Rust↔TS（`ipc-shapes.json`）——本次 `BootErrorPayload` 加字段就是被它抓到的；
- 网络面登记（`network_gate.rs`：MCP 探测的 streamable-http 为**条目级**豁免，非整文件）；
- 前端禁双源/禁裸中文/禁直呼 `invoke`/破坏性操作确认等既有门禁。

真机等价实验（我做的，可重跑）：

| 实验 | 结果 |
|:---|:---|
| 克隆 dev home 写入 chrome-devtools 行**不带 config** | dsh 退出码 1、34s，错误栈 = 你遇到的那份 |
| 同一行**带** `config: {mode: launch, headless: true}` | `dsh web: http://127.0.0.1:49906/?token=…` **正常就绪** |

---

## 1.5 我已完成的验收（2026-09-16，逐条给证据）

> 这一节是我**自己跑过的**部分，逐条对应下面的编号。结论列只有三种：
> ✅ = 已验过（含证据）· ⚠️ = 逻辑/文件/后端已验，**视觉或点击**仍需你一眼 ·
> 🖐 = 只有你能验（需要活跃 Host / 真实远端主机 / Windows / 你的手）
>
> 复跑方式都在命令里；新增的真机项以 `#[ignore]` 常驻仓库，不再靠人记。

| 编号 | 我验了什么（证据） | 结论 |
|:--|:--|:--|
| **A1** | 四卡数据由后端 `resolve_capabilities` 算全（Rust 24 项 + 前端 19 项 + 结构门禁 14 项）；**你已实际点过**（`plugin-op.log` 07:15–07:16 六次操作成功） | ⚠️ 布局/文案视觉待你一眼 |
| **A2** | 后端：本机无 `cua-driver`（`command -v` 空）→ PATH 探测判缺（Rust `missing_driver_blocks_only_the_variant_that_needs_it`）；写行守卫同源拒绝；前端门禁断言 `disabled` 含 `blocked !== null` 且红字在卡面（`experimentalCapabilitiesGate.test.ts` ⑦） | ✅ 逻辑已验，视觉待你一眼 |
| **A3** | **真机**：你的 playwright 行现为 `config: {mode: launch, headless: true}`（patch 原文）；克隆体实验证明**带 config 就就绪、去掉即退出码 1**；Rust 写入器/回填测试 3 项 | ✅ 完整验过（含你的真机操作） |
| **A4** | **真机**：你点过两次 replace（chrome-devtools→playwright、cua-driver-mcp→native）——`plugin-op.log` 有 `remove`/`add` 记录、patch 已无坏行、15:37 就绪 | ✅ 功能已验；层类「关闭即移除」文案视觉待你一眼 |
| **A5** | Rust `subset_variant_is_subsumed_not_conflicting` + `genuinely_exclusive_variants_still_conflict`；前端门禁 ⑤（「已包含」不提供独立开关） | ⚠️ 逻辑已验，视觉待你一眼 |
| **A6** | 诊断链**端到端**（含你 16:0x 的真机复现）：
① 你手工写行但包已卸载 → 失败措辞是 **`failed to import loader entry`**，我的解析器原先只认 `apply` → 分类落兜底（标题「DSH 工作台启动失败」+「重试」）——**这是你这次验收抓到的真缺陷**，已修（两种措辞都认，单测用你的真机原文）；
② 二次实测该路径 **44s** 才退出，原宽限 45s 只剩 1 秒余量 → 已放宽到 60s（20+40）；
③ 坏 profile 下 `--dump-config` 仍可用（exit 0）→ 一键隔离的"删后自证"能过；
④ 壳自有行才下发隔离（单测） | ✅ 缺陷已修 + 链路已验证；🖐 按钮点击请你来 |
| **A7** | 你当前 native/playwright 组合即 On 态；「包在行缺 → 修复补 config」由 `ensure_catalog_insert_row_backfills_missing_config` 钉住 | ⚠️ 逻辑已验 |
| **B1 / B2** | 代码核实：命令层**只有回环 RPC、无任何文件写入路径**（`workspace.json` 不被碰）；5 项单测（wire keys / 响应解析 / 业务错误 / 形状漂移 / 方法名） | 🖐 需活跃 Host + 你的点击 |
| **C1** | **真机端到端通过**：真实 `@modelcontextprotocol/server-everything@2026.8.31` → 协议 `2025-11-25`、**13 工具 / 7 资源 / 2 模板**、3.76s（新增可复跑 `#[ignore]` 测试） | ✅ |
| **C2** | **真机通过**：`http://127.0.0.1:9/mcp` → **2.19ms** 明确失败（有界、不挂死，新增 `#[ignore]` 测试） | ✅ |
| **C3** | 代码核实：`forgetProbe` 在**保存**与**删除**两条路径各调用一次 + 换 profile 整批丢弃（`useEffect([profileName])`）；前端测试钉住失败分支不折叠 | ✅ 逻辑 |
| **C4** | 后端显式报错分支存在（`commands/console.rs:248`「暂不支持 WSL 客体档…」），**无回落本地**分支 | 🖐 需你在 WSL 档点一次 |
| **D1** | 文案原文即「Web 工作台的文件树、编辑器与终端**不会**因此变成远端感知——上游明确不支持该形态」 | ✅ 文本无过度承诺 |
| **D2** | **真机通过**：本机 `~/.ssh/config` 解析出 **13 个 alias**，且 `Include` 的降级说明如实给出（新增可复跑 `#[ignore]` 测试） | ✅ |
| **D3 / D4 / D5** | 需要**真实 Linux/macOS 远端主机**（helper 部署 + SHA-256）／Windows 机器 | 🖐 留给你 |
| **E1** | 三选项键齐备（`PreferencesPane` 7 处引用）+ `settings.rs` 两键 + 默认 `auto` | ✅ 逻辑 |
| **E2** | **真机证据**：`plugin-op.log` 今日 **6 次**安装全部带 `--registry https://registry.npmjs.org`；`~/.npmrc` **未被改写**（仍 npmmirror + `@moresec` 私有源 + `strict-ssl=false`） | ✅ |
| **F1** | 全仓 `content-visibility` 命中 **0**；`injected/memory-policy.js` 已删；ADR-0002 已记修订 | ✅ 机械；滑动手感待你 |
| **F2** | 失败复现时 `dsh-shell.log` **非空（20565 字节栈）**；正常启动未被拖慢（你 15:37 那次 12s 就绪）；`stall_grace` 回归单测 | ✅ 逻辑 + 计时 |
| **G1–G5** | ADR 0020–0024 全在且 `docs/adr/README.md` 索引 24 行含一行结论；登记册 **63** 条；复现点 20/21；今日广播 4 条；`AGENTS.md` **219** 行（≤250） | ✅ |

**本节新增的可复跑真机项**（`#[ignore]`，缺环境变量即跳过）：

```sh
# C1：真实 stdio MCP 服务器端到端
DSH_MCP_E2E_CMD=node \
DSH_MCP_E2E_ARGS=/abs/path/node_modules/@modelcontextprotocol/server-everything/dist/index.js \
  cargo test --lib mcp_probe::tests::real_stdio_server -- --ignored --nocapture

# C2：不可达端点有界失败
cargo test --lib mcp_probe::tests::unreachable_http -- --ignored --nocapture

# D2：本机真实 ~/.ssh/config 解析 + 诚实降级
DSH_SSH_E2E=1 cargo test --lib ssh_config::tests::real_user_config -- --ignored --nocapture
```

**闸门复核（本轮改动后）**：Rust `fmt` / `clippy -D warnings` 干净、`cargo test`
**533 passed / 0 failed / 8 ignored**；前端 `tsc` / `oxlint` 干净、`vitest` **440 passed**、生产构建通过。

### 你三张截图给出的结论（2026-09-16 第二轮）

| 截图 | 判定 |
|:--|:--|
| ① 桌面控制「需要修复」+ 开关灰 + 红字「本机 PATH 中找不到「cua-driver」…」 | **A2 通过** ✅（前置门与文案都对） |
| ② 多智能体协同「已包含」+「开关与移除统一由「Web 档」控制」+ 无独立开关 | **A5 通过** ✅ |
| ③ 启动失败诊断台（`ERR_MODULE_NOT_FOUND`） | **抓到 1 个真缺陷**（见 A6 ①），已修；另暴露两处 UI 问题（见下） |

**截图①顺带暴露的两处 UI 问题（已修，`ca`→新提交）**：

1. **卡片徽标被"当前选中的后端"带偏**：能力其实在生效（native 档在跑），只因你点了被挡住
   的那一档，整张卡报「需要修复」。改为一律说**能力**的实话；选中档的状态由 chip 表达；
   被挡的档在 chip 上加 ⚠（悬停即是原因）。
2. **被前置门挡住的档仍有「修复」按钮**（假按钮）：修复=装包+写行，而后端两道都会拒。
   已加 `!blocked` 条件——出路写在红字里（先装 `cua-driver` 或换自包含档）。

### 留给你的（就这些）

1. **A2 / A5 的视觉确认**：桌面控制那档开关是灰的且有红字原因；开 Agent Teams Web 档后自建档显示「已包含」。
2. **A6 点一次「移除该行并重启」**（受控复现步骤见下；想跳过也行，诊断链我已验）。
3. **B1 / B2**：有活跃 Host 时点「取消归档」；无 Host 时应报错而非改文件。
4. **C4**：WSL 档点一次「探测」。
5. **D3 / D4 / D5**：需要真实远端主机与 Windows 机器；`docs/executor.md` 的 **F1–F9 / G1–G11** 全表同理。
6. **F1 手感**：长列表左右不再截断、滑动顺滑（我只能验到代码层）。

## 2. A 组 · 实验能力开关（I1，ADR-0020）

> 入口：**控制中心 → 插件中心 → 子 Tab「实验能力」**。四条能力：多智能体协同 / 浏览器操作 /
> 桌面控制 / 自动安全审查。改配置后需**重启该 Profile** 才生效（面板上有重启提示条）。

### A1 🖐 面板形态与状态真实
- **步骤**：打开上述入口，逐个展开四张卡的「详情」。
- **期望**：① 四张卡都在，各带状态徽标（未装 = 未启用）；② 浏览器卡有三个后端 chips（Playwright /
  Chrome DevTools / Stagehand），桌面卡有两个（复用已装 cua-driver / 随包自带运行时）；
  ③ 「详情」里能看到每步的包名、钉住的版本（`pkg@0.1.6-alpha.1`）、激活方式。
- **判定**：☐

### A2 🖐 前置未满足 → 开关**禁用**并说明原因（本次修复的核心之一）
- **前置**：本机没有 `cua-driver`（已确认 PATH 查无）。
- **步骤**：桌面控制 → 选「复用已装 cua-driver」。
- **期望**：开关**灰色不可点**，卡面（标题下方，不用展开）出现红字：
  「本机 PATH 中找不到「cua-driver」——装上会让 dsh 在加载插件时直接失败、工作台起不来…」；
  切到「随包自带运行时」那一档，**没有**这句话、开关可点。
- **判定**：☐

### A3 🖐 装一个能装的：浏览器操作 → Chrome DevTools
- **前置**：该包已在你的 dev home 装好（14:20 装过），当前**没有**挂载行（事故后被我摘掉）。
- **步骤**：选该后端 → 打开开关 → 等进度轨走完。
- **期望**：① 卡片状态变「生效」；② 提示重启；③ 打开
  `~/.dsh-dock-dev/profiles/web/cordis.patch.yml`，该行**带 `config`**：
  `mode: launch` 与 `headless: true`；④ `plugin-op.log` 里这次安装带 `--registry`（见 E 组）。
- **判定**：☐

### A4 🖐 关闭语义要说真话
- **步骤**：把 A3 刚开的 Chrome DevTools 关掉（同一开关）；再对「多智能体协同」按一下开关（先别确认）。
- **期望**：① 浏览器/桌面族是**行级**能力 → 关掉是秒级（不跑 pnpm）；② 多智能体协同/自动安全审查含
  profile 层 → 面板**在按下之前**就写着「关闭即移除」，且确认框说明会跑卸载。
- **判定**：☐

### A5 🖐 子集档不误报冲突
- **步骤**：多智能体协同 → 选「Web 档」开启；再看「自建档（无 Web 界面）」那一档。
- **期望**：自建档显示「已包含」、**不给独立开关**、也**不报「后端冲突」**（它的包正是 Web 档的基础层）。
- **判定**：☐

### A6 🧪 插件行把 dsh 搞挂 → 错误卡**点名**并可一键隔离（本次修复的另一半）
> 这是**受控复现**你昨天那次事故：手工写一条**缺 `config`** 的挂载行，看壳能不能自己诊断 + 一键修好。
- **步骤**：
  1. 备份：`cp -p ~/.dsh-dock-dev/profiles/web/cordis.patch.yml /tmp/patch.bak`
  2. 在该文件的 `- insert:` 列表里**手工追加**（注意：**故意不写 config**）：
     ```yaml
       - id: dsh-dock--deepseek-ai-dsh-experimental-browser-use-chrome-devtools-mcp
         name: '@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp'
     ```
  3. 重启该 Profile（面板的重启，或重启 dev app）。
- **期望**：① 启动失败，但错误卡标题是「**实验插件行导致启动失败**」；② 建议里**点名行 id**
  （`dsh-dock--…chrome-devtools-mcp`）与上游原因（`Cannot read properties of undefined (reading 'mode')`）；
  ③ 底部按钮是「**移除该行并重启**」（**没有**「重试」——重试必然再失败）；④ 点它 → 应用自己回来，
  patch 文件里那一行消失；⑤ 同时看 `…dsh-dock.dev/dsh-shell.log`：**不再为空**，能看到 dsh 的真实错误栈。
- **还原**：点完按钮即已还原；若中途放弃，用步骤 1 的备份覆盖回去。
- **判定**：☐

### A7 🖐 装完提示与「已装但行缺」的修复
- **步骤**：在 dev home 里保留一个「包已装、行缺失」的档（当前桌面控制就是：cua-driver-mcp 的包在、
  行被我摘了/或被前置门挡住）。
- **期望**：卡片显示「需要修复」而不是假装生效；可点的「修复」会把行**补齐**（含必需 `config`），
  且不会出现两个「修复/继续剩余步骤」这种重复按钮。
- **判定**：☐

---

## 3. B 组 · 已归档会话的取消归档（I2，ADR-0021）

### B1 🖐 壳内闭环
- **前置**：有活跃 Host（dev app 已就绪）。
- **步骤**：控制中心 → 会话维护 → 档位切到「已归档」→ 对某条会话点「取消归档」。
- **期望**：toast「会话已取消归档，回到活跃列表」；刷新后该会话出现在活跃档、不再带归档徽章。
- **判定**：☐

### B2 🖐 无 Host 时如实报错（不绕路改文件）
- **步骤**：停掉会话（或让 Host 未运行时打开会话维护），再点「取消归档」。
- **期望**：报「请先启动 DSH」一类的明确错误；**不得**静默去改
  `~/.dsh-dock-dev/storages/workspace.json`（该文件是 dsh 内存状态的投影，壳直写会 last-write-wins 覆盖）。
- **判定**：☐

### B3 📌 已知留白（不用验）
上游自带的「在 Web 设置页打开」次级入口**本批未做**（方案里是可选项）；上游 Web 设置页本身
仍可取消归档，壳内入口只是减少窗口切换。

---

## 4. C 组 · MCP 能力探测（I3，ADR-0022）

> 入口：**Profile 列表 → 选中 profile → 详情里的 MCP 服务器行 → 「探测」**。
> 完整 9 项清单在 [`docs/executor.md`](./executor.md) 的「MCP 能力探测实机验证清单 F1–F9」（**待跑**）。
> 这里只列最小集，**建议至少跑 C1–C3**。

### C1 🖐 stdio 服务器探测成功
- **步骤**：加一个 stdio MCP 服务器（如 `command: npx`，`args: ["-y","@modelcontextprotocol/server-everything"]`），点「探测」。
- **期望**：卡片表头出现 `工具 n · 资源 n · 资源模板 n`；展开能读到 tool 名与描述；显示快照时间。
- **判定**：☐

### C2 🖐 不可达 → 明确错误且不卡 UI
- **步骤**：把 `url` 改成 `http://127.0.0.1:9/mcp`（或在 stdio 档填一个不存在的命令），点「探测」。
- **期望**：**15s 内**返回明确错误（含后端原话），期间界面可正常切换，按钮转圈会停。
- **判定**：☐

### C3 🖐 配置一改，旧结论立即失效
- **步骤**：探测成功后，编辑该服务器的 command/url 并保存。
- **期望**：卡片**立即消失**（不许拿旧清单描述新配置）；切换 profile 同理整批消失。
- **判定**：☐

### C4 🖐 WSL 客体档如实拒绝
- **步骤**：切到 WSL 运行档后再点「探测」。
- **期望**：报「暂不支持 WSL 客体档…请在本地档使用」；**不得**静默回落宿主本地读取。
- **判定**：☐

---

## 5. D 组 · SSH 远程工作区向导（I4，ADR-0023）

> 入口：**控制中心 → Profile 列表 → 右侧「SSH 远程工作区向导」**。
> 完整 11 项清单在 [`docs/executor.md`](./executor.md) 的「SSH 远程工作区实机清单 G1–G11」（**待跑**）。
> 无远端 Linux/macOS 主机时，D1/D2/D5 仍可验；D3/D4 需要有主机。

### D1 🖐 范围声明不许过度承诺
- **期望**：向导第一段明确写着「面向 headless / 自建 profile」「Web 工作台的文件树、编辑器与终端
  **不会**因此变成远端感知——上游明确不支持该形态」。
- **判定**：☐

### D2 🖐 主机枚举来自 `~/.ssh/config`
- **步骤**：向导里的主机下拉；若你的配置里有 `Include` 或 `Match` 段，注意提示。
- **期望**：列出 alias（带 `user@host:port` 形态）；对 `Include`/`Match` 显示「未跟随 / 已忽略」的
  诚实说明，**不静默少列**。
- **判定**：☐

### D3 🖐 非交互预检拦住坏输入
- **步骤**：填一个不可达 alias，或把 helper 的 SHA-256 改成 64 个 `0`，点「非交互预检」。
- **期望**：预检报错并原样透出 ssh 的 stderr（`Permission denied` / `Connection refused` 等）；
  摘要不符时显示「远端实际值 vs 你填的值」两个摘要，且**不出现**「生成并安装」。
- **判定**：☐

### D4 🖐 生成结果正确（有主机时）
- **步骤**：预检全绿后点「生成并安装」，等分钟级完成。
- **期望**：① `profiles/ssh-remote/package.json` 的 `dsh.profile.bundles` 含 `@deepseek-ai/dsh-headless`
  且**不含** `@deepseek-ai/dsh-web-app`；② `cordis.patch.yml` 出现**四条** insert 行
  （`dsh-dock-ssh` / `-fs-ssh` / `-subprocess-ssh` / `-sandbox-ssh`），**只有第一条带 `config` 且恰五键**
  （host / node / helper / helperHash / workspace）；③ 四个 ssh 包版本 = 运行时版本（不是 registry latest）。
- **判定**：☐

### D5 🖐 Windows 宿主硬拦（有 Windows 机器时）
- **期望**：Windows 上向导显示「当前宿主是 Windows…本向导在此不可用」，预检按钮禁用。
- **判定**：☐

---

## 6. E 组 · 插件安装源策略（ADR-0006 §6）

### E1 🖐 三个选项在那里、默认是自动
- **步骤**：控制中心 → 系统控制台 → 子 Tab「偏好与守护」→ 找「插件安装源」。
- **期望**：`自动（推荐）` / `只用官方源` / `只用本机配置的源` 三选一，默认**自动**；
  下面显示「上次可用：官方源 / 本机配置的源」（未用过时可能为空）。
- **判定**：☐

### E2 🖐 源真的被按次传给 pnpm（不改你的 npm 配置）
- **步骤**：保持「自动」，在插件市场装/更新任意一个包；然后看
  `…dsh-dock.dev/plugin-op.log` 里这次的命令行；再看 `~/.npmrc`。
- **期望**：日志里出现 `--registry <某个源>`；`~/.npmrc` **未被改写**；包里成功后会记下「上次可用」。
- **另验**（可选）：把偏好切成「只用官方源」再装一次 → 日志里恒为 `--registry https://registry.npmjs.org`；
  切成「只用本机配置的源」→ **不带** `--registry`（沿用你的 npmmirror / 私有源）。
- **判定**：☐

---

## 7. F 组 · 两处修复

### F1 🖐 WebView 两侧截断 / 触控板滑动卡顿（已移除 content-visibility 策略）
- **步骤**：在控制中心里滚动长列表（会话维护、插件市场、插件清单），左右边缘观察；用触控板连续滑动。
- **期望**：内容**不再被裁掉左右两侧**，滑动顺滑无阶跃。
- **反向证据**：`frontend/src/injected/memory-policy.js` 已删除（该目录现只剩
  `handoff-curtain.js` / `link-hook.js` / `switcher.js`），全仓已无 `content-visibility` 命中，
  ADR-0002 已记修订。
- **判定**：☐

### F2 🖐 启动失败可见性（本次事故的收口）
- **步骤**：正常冷启动一次（应正常就绪）；然后做 A6 的受控复现。
- **期望**：① 正常启动仍在十几秒内就绪，**没有**因为放宽停滞窗口而变慢；② 失败时不再出现
  「详情见日志」而日志为空——错误卡直接给出真实原因与出错行 id。
- **判定**：☐

---

## 8. G 组 · 治理与文档面（抽查其中 2–3 项）

| # | 项 | 看什么 | 判定 |
|:--|:--|:--|:--|
| G1 | ADR 立项 | `docs/adr/0020`…`0024` 五份 + `docs/adr/README.md` 索引（含一行结论） | ☐ |
| G2 | 登记册 | `docs/contracts/ipc-and-network-register.md`：63 条命令、网络面用途（含 MCP streamable-http 的**条目级**豁免） | ☐ |
| G3 | 复现台账 | `docs/contracts/dsh-behavior-ledger.md` 复现点 13–21（新增 21 = 本次插件树事故） | ☐ |
| G4 | 广播留档 | `docs/broadcasts.md` 顶部：本批各条（能力开关重构 / 安装源策略 / 插件行修复 / 本次合并） | ☐ |
| G5 | AGENTS 变更 | `AGENTS.md` §6 持久化键（`pluginRegistry` 等）、§7 登记册迁出、行数仍在 250 内 | ☐ |

---

## 9. H 组 · **明确不在本批**（出现即当缺陷报我，但不属于本批承诺）

| 项 | 状态 |
|:---|:---|
| I5 桌面任务快跑器（ADR-0024） | **暂缓**，未实施 |
| 实验能力面板在 **WSL 客体档** | 显式报错，不回落本地（无客体侧 patch 读写原语） |
| 启动失败后的**自动**隔离 + 自重试 | 未做；本批给的是错误卡上**一键** |
| 取消归档的「在 Web 设置页打开」次级入口 | 未做（可选项） |
| MCP 探测的 `resources/read` 预览、SSRF 策略 | 未做（ADR-0022 明确不解决 SSRF，只限影响面） |
| SSH 远程工作区的会话级 capability 收敛 | 继承 `docs/roadmap.md` §4.8，未做 |
| `docs/executor.md` 的 B1–B7 / C2 / D / F1–F9 / G1–G11 | **待跑**（真实网络与实机项，本批未跑） |

---

## 10. 结果回填

| 组 | 编号 | 结论 | 现象/备注 |
|:--|:--|:--|:--|
| A | A1–A7 | | |
| B | B1–B2 | | |
| C | C1–C4 | | |
| D | D1–D5 | | |
| E | E1–E2 | | |
| F | F1–F2 | | |
| G | G1–G5（抽查） | | |

> 想要更细的实机项（真实 MCP 服务器、真实远端主机）请照 `docs/executor.md` 的 F/G 表跑；
> 跑完把结果回填到那张表里（仓库口径：**没跑过的不得写成已验证**）。
