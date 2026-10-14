# ADR-0034：保留档 `desktop` 收窄为「只禁生命周期面」——内容面经引擎 bootstrap 的载体通道放行

- **日期**：2026-10-14
- **状态**：已接受（维护者裁定）
- **提出人**：guan（AI 协作：dsh-dock agent）
- **相关方**：`src-tauri/src/{engines,profiles,plugins,mcp}.rs`、`frontend/src/components/profiles/**`、
  `frontend/src/pages/ProfileManager.tsx`、`frontend/src/components/market/MarketplaceView.tsx`、
  `docs/contracts/dsh-behavior-ledger.md`（复现点 23 修订 / 26 新增）
- **关联**：ADR-0017（引擎 shim）/ ADR-0018（Windows bootstrap）/ ADR-0019（模块代理增强）；
  取代 2026-10-06 落地的「一切写入口前置拒绝」口径（`eaa0d0c`，复现点 23 ③）

---

## 1. 背景与问题

2026-10-06 发现 dsh 对 `--profile desktop` 有**纯名字守卫**（`lib/bin.js:35-37`），
于是壳把 `desktop` 定为保留名，**一切写入口前置拒绝**：启动/切换、设为默认、创建、
重命名两头、删除、市场安装、MCP profile 层、插件启停、兜底放空、策展挂载行、配置复制目标。
只读面（列表 / 详情 / 插件清点）保留，唯一允许的写 = 复制成非保留名。

**维护者 2026-10-14 裁定**（原话）：「desktop profile 虽然不支持启动重启，但是该 profile
下的管理功能还是应该需要支持的，不能都禁用了」；细化答复：「启动、删除、重命名、设置为
默认 profile 这一类的操作都禁用。插件、实验能力、MCP 等都正常使用」。

即：2026-10-06 的口径**过度收口**——把「dsh 拒绝外部启动它」这一个事实，放大成了整档只读。

### 1.1 现状事实（2026-10-14 本机实测，dsh 0.2.0-rc.2 = 壳引擎）

| 通道 | 行为 |
|:--|:--|
| `dsh --profile desktop`（启动） | **无条件**拒绝（`bin.js:112`），载体开关也救不了 |
| `dsh plugin --profile desktop …` | 拒绝（`bin.js:119`），**除非**调用方以程序化选项传 `manageDesktopProfile: true` |
| `dsh --profile desktop --dump-config` | **无条件**拒绝（dump 走同一主 action） |
| `runCli({manageDesktopProfile:true})` + `plugin …` | **exit 0**（实测 `version-exemptions` 打印 `{}`；经壳 shim 跑真实内核 `why ds-bot` 输出依赖树） |
| `dsh-app-boot::loadProfileDirectory(dir)` | 公开 API，自述用途即「应用自有的 profile 目录」；组合树输出与命令行 dump **逐字节相同** |

关键区分：**守卫只存在于命令行解析器**，dsh 自己在库层提供了两条受支持通道——
`runCli` 的 `manageDesktopProfile` 选项（上游自述语义：*"permit Desktop's installed carrier
to manage its reserved profile's plugins"*）与 `loadProfileDirectory`（按**目录**而非名字加载）。

## 2. 约束与硬指标

1. **不复刻 dsh 逻辑**（红线 1 的精神）：宁可调用 dsh 自己的代码，也不重写其组合/插件管理语义。
2. **不静默降级**（红线 3）：技术不可能的面必须显式报错并说明原因。
3. **壳驱动 dsh 的既有形态**：ADR-0017/0018 已经确立「壳经自写 shim 调 `runCli()`」，
   本决策不引入新的机制类别，只在既有 bootstrap 上加两个哨兵模式。
4. **生命周期面必须继续关闭**：启动守卫连载体开关也拦；改名/删除动的是官方客户端自己的
   档案（破坏性），设为默认会造出「每次启动都失败」的常驻故障。
5. **保留档判定单源**：`profiles::{is_reserved_profile, ensure_not_reserved_profile}` +
   前端镜像 `lib/profiles.ts::isReservedProfileName`，不新增第二份名单。

## 3. 备选方案及评估

### 方案 A：只放开纯文件面（MCP / 启停 / 策展行） —— ❌ 否决

- 思路：不经 dsh CLI 的写入口放行；依赖 CLI 的（装卸/更新、组合树行表）继续拒绝。
- 否决理由：插件列表 tab 仍然基本只读（启停要行 id、装卸要 pnpm 转发）⇒ **没有兑现
  维护者的裁定**（「插件、实验能力、MCP 等都正常使用」）。

### 方案 B：自建行枚举 + 直接 pnpm 复现 `dsh plugin` —— ❌ 否决

- 思路：按各 bundle 自带 patch 复现行表；安装/卸载/更新改为在 profile 目录直接跑 pnpm。
- 否决理由：违反约束 1——把 dsh 的插件管理器（版本兼容门、profile 写锁、失败回滚、
  `dsh.profile.bundles` reconcile）**重写一遍**，dsh 升级即静默漂移；且跳过兼容门可能把
  不兼容插件装进官方档（复现点 24 ① 表明上游正是靠这道门）。

### 方案 C：profile 目录别名 + 命令行 dump —— ❌ 否决

- 思路：`profiles/desktop-x` 符号链接指向 `desktop`，用别名跑 `--dump-config`。
- 否决理由：污染 profile 命名空间（官方客户端自己也会列这个档），且属对守卫的
  **变形绕行**（AGENTS §8.7）——同样绕守卫，方案 D 用的是上游**自己提供**的通道，更干净。

### 方案 D：引擎 bootstrap 两个哨兵模式（载体开关 + 公开 API） —— ✅ 最终采纳

- 思路：在既有 `dsh-boot.mjs`（ADR-0018）上加两个模式，只对保留档使用：
  - `--dsh-dock-manage <dsh args…>` → `runCli({ manageDesktopProfile: true })`（插件装卸/更新）；
  - `--dsh-dock-dump <profileName>` → `loadProfileDirectory(binName, dir, installAnchor, {userLayer:true})`
    ＋ `loadOptionalPatches` ＋ `renderConfigDump`（组合树行表，供启停/实验能力/写后自证）——
    签名逐字对齐实现与复现点 26，`installAnchor` = dsh 自己的 `package.json`。
- 优点：**零复刻**（跑的都是 dsh 自己的代码）；兼容门 / 写锁 / 回滚 / reconcile 全保留；
  组合树与命令行 dump **逐字节相同**（实测 `cmp` 全等）⇒ 下游解析器一行不改。
- 代价/风险：依赖 dsh 的程序化选项与 app-boot 公开 API 的稳定性 ⇒ 登记复现点 26 的
  升级复核项；`--dump-config` 的等价性靠"bundle→profile→home 层"的 shim 侧复排，
  且**不能读 profile 根配置**——它是 dsh 的草稿文件，一次 boot 会把组合树写进去，
  而 CLI 会先 `prepareProfile` 重置成 `[]`；shim 改为组合到临时空 `[]` 文件
  （`renderConfigDump` 只用 basename 作标签 ⇒ 输出仍逐字节相同）。直接读根配置的
  第一版实测**树翻倍**（196 → 391 行 id，2026-10-14 活跃档实测），已修。
- 对照约束：①零复刻 ✅；②客体世界无壳 shim ⇒ 显式缺口报错 ✅；③沿用既有 bootstrap ✅；
  ④启动/改名/删除/设为默认仍由 `ensure_not_reserved_profile` 前置拒绝 ✅；⑤判定仍单源 ✅。

### 方案 E：维持 2026-10-06 全禁 —— ❌ 否决

- 否决理由：维护者已明确裁定这是过度收口；且 UI 上「官方保留 = 整页只读」与事实不符
  （MCP / patch / 插件清单都能读，只是操作被壳自己挡住）。

## 4. 决策

**保留档 = 只禁生命周期面**：

| 面 | 动作 | 处置 |
|:--|:--|:--|
| 生命周期 | 启动 / 重启 / 切换、设为默认、创建同名档、重命名两头、删除 | **继续前置拒绝**（`ensure_not_reserved_profile`） |
| 内容 | 插件安装 / 卸载 / 更新、插件启停、实验能力挂载行、MCP profile 层、配置行复制（目标） | **放行** |
| 破坏性修复 | `quarantine_patch`（备份并放空用户层） | **继续拒绝**（清的是官方客户端自己的用户层，不在裁定列举的内容面内） |
| 只读 | 列表 / 详情 / 插件清点 / patch 视窗 / 复制成非保留名 | 不变 |

实现落点：

1. `engines.rs::dsh_bootstrap_mjs`：新增 `--dsh-dock-dump` / `--dsh-dock-manage` 两个哨兵
   （未命中哨兵时行为与从前**完全一致**；Windows `dsh.cmd` 仍指同一份 bootstrap）。
2. `profiles.rs::run_reserved_forward`：`node <engines>/bin/dsh-boot.mjs <哨兵> <args…>`。
3. `plugins.rs::reserved_cli_route`：**世界择源单源**（Local ⇒ Shim；Wsl ⇒ 显式缺口错误），
   插件变更与组合树共用。
4. 放开 `set_plugin_disabled(_in_guest)` / `ensure|remove_catalog_insert_row(_in_guest)` /
   `copy_plugin_config_*(target)` / `mcp::ensure_writable_scope(Profile)` 的保留档拒绝。
5. 前端：把一刀切 `reserved` 禁用改为**逐项门控**——行内动作与「添加插件」恢复可用，
   `FirstPartyPlugins` / `McpManager` 的 `readOnly` 属性整体删除（不再有 true 的调用方），
   市场安装目标不再过滤保留档；生命周期入口（启动/重启/设为默认/改名/删除）与徽标文案
   改用新文案 `reservedLifecycleHint`（横幅 `reservedHint` 讲清两面）。
6. **通道不可用要说清 + 保留档没有"重启"**（2026-10-14 复核补记，不改变本决策）：
   两个哨兵模式都依赖 dsh 的程序化选项 / 公开 API ⇒ **引擎比 0.2.0-rc.2 旧时通道不可用**。
   此时转发链的两个消费点（插件变更 / 组合树）把上游名字守卫文本或 bootstrap 自述前缀
   翻成中文 + 出路（`profiles::reserved_channel_gap_hint`，升级 dsh 引擎），并保留原始
   输出尾部作证据；bootstrap 侧对档名做 `/ \ . ..` 纵深校验、对缺失的公开 API 显式报错
   （不抛裸 TypeError）。另：保留档内容面既然可写，就会有待重启标记——但它的生命周期面
   关闭，故待重启提示在该档**不给「立即重启」按钮**，改为说明「改动在官方客户端下次
   启动时生效」（前端 `ui/restart-hint.tsx` + 后端 `classify_op_outcome` 同一口径）。

## 5. 后果

- **用户可见**：`desktop` 档的插件列表、实验能力、MCP 与其它档一致可用；启动/重启/设为
  默认/改名/删除仍禁用并给出原因；「官方保留」徽标保留（它仍是官方客户端的档）。
- **写入面**：壳现在会真的改写 `~/.dsh/profiles/desktop/**`（经 `PatchFile` 的覆写前备份 +
  原子替换，或经 dsh 自己的插件管理器）。这是维护者明确要的能力，但**官方客户端的配置
  从此可被本面板改动**——它下次启动即读到这些改动。
- **平台差异（显式）**：WSL 客体世界没有壳 shim ⇒ 该世界的保留档**插件装卸/更新与组合树
  查询**报明确缺口错误（不静默降级）；纯文件面（启停/实验能力/MCP/复制）两侧一致。
- **复现负担**：新增复现点 26（两个通道 + 逐字节等价性），升级 dsh 时必须逐条复核
  （选项名、`loadProfileDirectory` 签名、`renderConfigDump` 语义、根配置文件名）。
- **旁证（非本决策范围）**：本次实测发现 `dsh-app-boot` 0.2.0-rc.2 **不再导出**
  `healProfilesModuleFallback`，故 bootstrap 的 ADR-0019 客户端代理增强在该版本上是
  **静默 no-op**（异常被 catch 成一条 warning）。这是既有漂移，独立立项，不在本 ADR 内修。

## 6. 验证

- 等价性：同一份 shim 对 `web` 档的 dump 输出 vs `engines/bin/dsh --profile web --dump-config`
  → `cmp`/`md5` 逐字节相同（正式档引擎 0.2.1-alpha.2：1719 行；dev 档引擎 0.2.0-rc.2：1658 行）；
  对**正在被官方客户端运行的** `desktop` 档（`cordis.yml` 里带着 boot 快照）→ exit 0、
  1474 行、196 条 `- id:` 行、47 段落，与 CLI 重置基准后的组合一致。
- 载体通道：`--dsh-dock-manage plugin --profile desktop version-exemptions` → exit 0；
  真实内核只读动词 `why ds-bot` → 输出 `ds-bot@0.1.5 └── dsh-profile-desktop`；
  同一 argv 走普通 CLI → exit 1 英文守卫。
- 回归：`plugins::reserved_cli_route_splits_local_and_guest`（本地/客体分叉）、
  `plugins::patch_writers_allow_reserved_profile_but_quarantine_still_refuses`、
  `mcp::reserved_profile_may_write_both_scopes`；生命周期面仍由
  `profiles.rs` 的保留名测试与前端 `unmaterializedProfileGate` 钉住。
