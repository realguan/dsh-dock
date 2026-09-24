# ADR-0020：官方插件目录与安装激活契约

- **日期**：2026-09-15
- **状态**：**已接受**（2026-09-15 维护者裁定「评审通过」）——I1 增量据此解冻
- **提出人**：guan（AI 协作起草）
- **相关方**：`src-tauri/src/plugins.rs`（`PatchFile`）/ `frontend/src/components/market/*`
  （`PluginHub` · `MarketplaceView` · `MarketPluginCard` · `MarketInstallDialog`）/
  `frontend/src/stores/queueStore.ts` / `frontend/src/types/market.ts` /
  `frontend/src/content/{zh-CN,en-US}.ts` / `docs/contracts/dsh-behavior-ledger.md`
- **关联**：ADR-0009（Profile 生命周期与写入例外册，本 ADR 不改其写入边界）·
  ADR-0011（安装来源三形态白名单，本 ADR 不改其判别）· ADR-0013（构建脚本默认批准）·
  `docs/plans/official-plugins-and-capabilities-plan-review.md`（审核报告 B1/B2/B3）·
  复现台账行 7（`docs/contracts/dsh-behavior-ledger.md:22`）与行 13（`:28`）·
  `docs/roadmap.md:31`（patch 浅覆盖陷阱）

---

## 1. 背景与问题

dsh 官方把绝大多数高级能力以实验性包形式发布在 `packages/experimental/`。社区市场
（awesome-dsh-plugin）不感知它们，因此 dsh-dock 计划在插件中心新增一个官方插件专区；
动手前对上游 `0.1.6-alpha.1` 做了逐点核查，暴露出下列事实，它们直接决定本契约的形态。

1. **上游没有"首批"这个概念。** 政策是**全量发布**（`packages/experimental/README.md:12`
   "All current packages publish under their `@deepseek-ai/dsh-experimental-*` names"；实查
   16 个包；`scripts/experimental-package-policy.ts:2` 的豁免表为空）。故 dsh-dock 可以策展
   精选集，但**它只是 dsh-dock 的策展集**，不是"官方首批"——后者是归属失实。

2. **`dsh plugin add` 不是纯 pnpm 转发器，它自带激活语义。** `apps/cli/src/plugin.ts:1-11`
   的模块文档写明该命令是"initialize the profile on first use, run `pnpm <args...>`, **then
   reconcile the `dsh.profile.bundles` layer list against the installed state**"；`:59-91`
   `reconcilePlugins` 按**已安装状态**（而非依赖 diff）判定：解析到声明 `dsh.bundle` 的依赖
   会**加入层栈**，`:89` 写回 profile 的 `package.json`。本仓库台账行 7（`:22`，2026-08-29
   复核）已登记该机制："**reconcile 会把声明 `dsh.bundle` 的新装依赖追加进 bundles**……
   同一包名 bundles/dependencies 双现属 dsh 数据模型本然"。

3. **实测印证（2026-09-15，抛离式 `DSH_HOME`）**：`dsh plugin --profile <n> add
   @deepseek-ai/dsh-experimental-agent-team-profile` 之后，profile 清单的
   `dsh.profile.bundles` 由 `[base, web-app]` 变为 `[base, web-app, …-agent-team-profile]`
   ——**对声明 `dsh.bundle` 的包，CLI 已经完成激活**。

4. **方案原稿的写 patch 步骤在实测中被证明是"静默失败"**。用 `- name: '<pkg>'` 作
   patch 条目时：`dsh --profile <n> --dump-config` **退出码 0**、stdout 正常输出整棵树，
   但 stderr 打出 `patch: id is required for non-insert patches`，且该包在组合树中
   **命中数为 0**（未挂载）；改为 `- insert:` + `id` 后即正常挂载。**这是最危险的一类缺陷**：
   GUI 会报告"安装成功"，而插件永远不加载。

5. **哪些包声明 `dsh.bundle`（实查 `packages/experimental/*/package.json`）**：

   | 包 | `dsh.bundle` | `dsh plugin add` 后是否自动激活 | 壳必须做什么 |
   |:---|:---:|:---:|:---|
   | `…-experimental-auto-review` | ✅ `./cordis.patch.yml` | ✅ 是 | 只可覆盖配置，**禁止**挂载行 |
   | `…-experimental-agent-team-profile` | ✅ | ✅ 是 | 同上 |
   | `…-experimental-agent-team-web-profile` | ✅ | ✅ 是 | 同上 |
   | `…-experimental-browser-use-playwright-mcp` | ❌ 无 | ❌ 否（仅告警） | **必须写挂载行** |
   | `…-experimental-browser-use-chrome-devtools-mcp` | ❌ | ❌ 否 | 必须写挂载行 |
   | `…-experimental-browser-use-stagehand-native` | ❌ | ❌ 否 | 必须写挂载行 |
   | `…-experimental-computer-use-cua-driver-mcp` | ❌ | ❌ 否 | 必须写挂载行 |
   | `…-experimental-computer-use-cua-driver-native` | ❌ | ❌ 否 | 必须写挂载行 |
   | `…-experimental-agent-team` | ❌ | ❌ 否 | 必须写挂载行 |
   | `…-experimental-tool-agent-team` | ❌ | ❌ 否 | 必须写挂载行 |

   对非 bundle 包，CLI 只安装并在 stderr 明示（`apps/cli/src/plugin.ts:72`）：
   "`<pkg> declares no dsh.bundle — installed as a plain dependency, **not a profile layer**`"。

6. **裸包名会按 `latest` dist-tag 解析，而它已经落后。** 实查 npm registry（2026-09-15）：

   | 包 | `latest` | `alpha` |
   |:---|:---|:---|
   | `…-experimental-agent-team-profile` | **0.1.5-alpha.2** | 0.1.6-alpha.1 |
   | `…-experimental-agent-team` | **0.1.5-alpha.2** | 0.1.6-alpha.1 |
   | `…-experimental-tool-agent-team` | **0.1.5-alpha.2** | 0.1.6-alpha.1 |
   | 其余（`auto-review`、`browser-use-*`、`computer-use-*`） | 0.1.6-alpha.1 | 同 |

   实测：裸名 `add` 实装 **0.1.5-alpha.2**（运行时为 0.1.6-alpha.1）；改用
   `add "<pkg>@0.1.6-alpha.1"` 实装正确版本，**且 reconcile 照常把该包追加进 bundles**——
   即"钉版本"与"自动激活"可叠加。此失败模式与本仓库台账行 7（`:22`）记载的
   dsh-base dist-tag 事故**同源**（原文："**add 裸包名会按 dist-tag `latest` 解析**……
   → 404 + pnpm 递增重试 → 数分钟失败/超时"）。

7. **patch 行的三条硬语义**（写错了不会报错，只会静默走偏）：
   - **行身份是 `id`，不是 `name`**。`vendor/loader/src/config/tree.ts:51` `ensureId()`
     在缺 `id` 时**随机生成**，该行此后**再也无法被后续 patch 命中**。
   - **写 `config` 键 = 整体替换，不深合并**。真实 schema：`PatchOptions`
     （`vendor/include/src/index.ts:130`）与 `EntryOptions`（`vendor/loader/src/config/entry.ts:9`）；
     **不存在 `before`/`after` 排序键**，顺序即数组位置。本仓库 `docs/roadmap.md:31` 已登记
     同一陷阱："一旦写 `config` 键，该行 config **整体替换不深合并**"。
   - 挂载一个新插件必须用 `- insert:` 列表形式（权威示例：
     `packages/bundle/base/README.md:55-59` 的 `tool-str-replace-editor`、
     `docs/user/develop/basic/config.md:37` 的 `hello`）；顶格 `- name:` / `- id:` 只能
     **覆盖/禁用已存在的行**（仓库既有 `set_plugin_disabled` 走的正是这一形态，见台账行 13
     `:28`）。

8. **同族 provider 互斥**：浏览器后端三选一、桌面控制二选一。装第二个会**激活失败**——
   `packages/experimental/computer-use-cua-driver-mcp/README.md:53`："A second computer-use
   provider **fails activation**, including another instance of this package."；浏览器同口径见
   `docs/subsystems/browser-use.md:17`。故目录 UI 若把 5 个 provider 平铺为独立可叠加条目，
   用户装第二个即坏。

9. **配套关系不是直觉形态**：`browser-use-runtime` 是**库、无 `dsh.bundle`、也没有可挂载行**
   （`.../browser-use-runtime/README.md:28`："It has **no plugin entry or mount configuration**"），
   可挂载的配套是 release 服务 `@deepseek-ai/dsh-browser-use` ＋恰好一个 provider；
   **不存在 `computer-use-runtime`**，桌面控制需 `@deepseek-ai/dsh-computer-use`（peer）＋
   `dsh-mcp-client`；Agent Teams 需 `agent-team` ＋ `tool-agent-team`（前者自述"It provides
   **no tools of its own**"，`agent-team/README.md:12`）＋ 持久化
   （`agent-team/src/index.ts:60`），Web 侧以 `agent-team-profile` 为便捷 bundle。

10. **"官方"目前在壳内是启发式，不是权威数据**：`MarketPluginCard.tsx:50` 用
    `owner.includes("deepseek") || name.startsWith("@deepseek-ai/")` 现算，并已渲染官方徽章
    （`:61-62`、`:87`）＋ i18n `t.market.officialCoreTitle`。引入策展目录后若不迁移该卡片，
    即形成双源（AGENTS §11.3 禁双源）。

## 2. 约束与硬指标

1. **红线 1 / 红线 2**：不修改 dsh 源码（激活只能靠 CLI 与自身 patch 文件）；
   安装包不内置依赖，本 ADR 不改变引擎引导链路（ADR-0010）。
2. **写入面唯一**：对 profile `cordis.patch.yml` 的一切写入**必须**经
   `plugins.rs::PatchFile`（AGENTS §6:131：原文保真含行间注释 ＋ 覆写前备份 ＋ 原子替换）。
3. **安装来源白名单不变**：本 ADR 在 ADR-0011 的三形态（npm / `github:` / `https://…tgz`）
   之内运作，**不新增来源形态**，也不放宽 `validate_install_spec`。
4. **版本语义可复算**：期望版本必须可从**已安装的 dsh 运行时**推导；禁止依赖
   `latest` 这类可漂移标签（台账行 7 的既定教训）。
5. **`config` 写入必须先读后写**：因整体替换语义（`docs/roadmap.md:31`），任何改配置的实现
   必须先取该行**完整生效 config** 再整体写回；不得构造"只带想改字段"的 patch 行。
6. **行 `id` 稳定可复算**：壳生成的每个挂载行必须携带确定性 `id`（缺 `id` 会被自动生成，
   该行永久不可再维护）。
7. **dsh 文件系统不变量**（AGENTS §6）：三件套不得生成/复刻；`profiles/node_modules`
   符号链接农场不得直写；不新增超出例外册的写面。
8. **能力须如实呈现**：`packages/experimental/README.md` 明示这些包
   "contracts can change and carry no support promise"，且"Released products outside this
   group must not depend on experimental packages"——目录 UI 必须如实呈现其**实验状态与实际
   可用条件**（含"装得上是否等于用得了"），不得让用户误判。
9. **禁双源**（AGENTS §11.3）："是否官方"只能有一个真相源。
10. **有序组合条目不得并发下发**（2026-09-15 审核补充）：部分条目的正确形态不是"装一个包"，
    而是**严格有序的多步安装**，且 `dsh.profile.bundles` 的**顺序即语义**。已核实依据：
    - `packages/experimental/agent-team-web-profile/README.md`（Install into a profile）：
      "Add the Host and Web Agent Teams layers to an initialized `web` profile **in this order**"
      ——先 `dsh-experimental-agent-team-profile`，再 `dsh-experimental-agent-team-web-profile`；
    - 同 README（Known Limitations）："**Ordered composition** — `dsh-base`、`dsh-web-app`、
      `dsh-experimental-agent-team-profile`、and this package **must remain in that order**"；
    - `packages/experimental/agent-team-web-profile/cordis.patch.yml:1-2` 首行注释：
      "Apply **after** `dsh-web-app` and the host-side `dsh-agent-team-profile` so the browser
      mounts only when the Team service is present"。
    因此：每次 `dsh plugin add` 只接受一个包，组合条目必须**串行**下发；顺序颠倒会导致
    浏览器侧在 Team 服务就位前挂载而失败。`agent-team-web-profile` 的 `dependencies` 已含
    `@deepseek-ai/dsh-experimental-client-ui-agent-team`，壳**不得**再单独安装该 UI 包。

## 3. 备选方案及评估

### 方案 A：按 `dsh.bundle` 分支的激活契约 ＋ 策展目录 ＋ 强制钉版本 —— ✅ 最终采纳

- **思路**：目录条目元数据自带 `declaresBundle` 判定，安装流程分两支：
  - `declaresBundle === true` → 只跑 `dsh plugin --profile <n> add "<pkg>@<ver>"`，
    **不写任何 patch**；需要配置时另发一条 `- id:` 覆盖行（先读全量 config）。
  - `declaresBundle === false` → 跑 `add "<pkg>@<ver>"` **并**经 `PatchFile` 写入挂载行
    `- insert: [{ id: <稳定派生 id>, name: <pkg>, config: … }]`。
  同族 provider 在 UI 层强制互斥（"替换"语义）；配套包由条目声明、由队列串行补齐。
- **优点**：与 CLI 既有语义同向（不与之争夺激活所有权）；两类包各自只有一条正确路径；
  钉版本消除台账行 7 的同源事故；`id` 稳定使行长期可维护。
- **代价/风险**：目录元数据成为需要维护的**新真相源**（须随 dsh 升级复核，见 §6）；
  非 bundle 包的挂载行由壳生成，壳因此承担 patch 结构正确性的责任；
  互斥只能在 UI 层表达（上游无声明式互斥元数据），若用户绕过 UI 手工装第二个仍会失败。
- **对照约束**：①只调 CLI 与 patch 文件、引擎链路不动（红 1/红 2）✔；②一切写入经 `PatchFile` ✔；
  ③仍在三形态白名单内 ✔；④版本由运行时推导并显式展示 ✔；⑤配置改动先读全量再写回 ✔；
  ⑥`id` 由包名规范化派生（可复算，不随调用变化）✔；⑦不新增超出例外册的写面 ✔；
  ⑧条目携带实验状态与真实可用条件 ✔；⑨`isOfficial` 唯一来源为目录元数据 ✔。

### 方案 B：壳恒写 patch 行（不分支） —— ❌ 否决

- **思路**：既然非 bundle 包必须写挂载行，就对所有包统一写一条。
- **否决理由**：违反约束 2（写入必须经 `PatchFile` 且只写该写的）与约束 7（不得破坏 dsh
  文件系统不变量），且两个方向都错：
  - 用 `- insert:` 统一写 → 对声明 `dsh.bundle` 的包造成**重复挂载**（包已由 `bundles` 层挂载，
    用户层再插一行同 `name`、不同 `id` 的实例即两份）。行身份是 `id`
    （`vendor/loader/src/config/tree.ts:51`），同名不同 `id` 不会被去重。
  - 用 `- name:` 统一写 → 对**全部**目标静默失效：实测 stderr
    `patch: id is required for non-insert patches`、退出码 0、命中数 0。

### 方案 C：壳什么都不写，只做 `dsh plugin add` —— ❌ 否决

- **思路**：完全信任 CLI 的 reconcile，壳只负责调用。
- **否决理由**：违反约束 8（能力须如实呈现）——非 bundle 的 6 个包（3 个 browser-use、
  2 个 computer-use、`agent-team`/`tool-agent-team`）**永远不会激活**，CLI 只会在 stderr
  留一行告警（`apps/cli/src/plugin.ts:72`）。而 browser/computer-use 恰是本目录的主要价值面，
  等于交付一个"装得上、用不了、界面不说"的专区。

### 方案 D：装裸包名，把版本选择交给用户 —— ❌ 否决

- **思路**：`add <pkg>` 不钉版本，由用户自行决定是否钉。
- **否决理由**：违反约束 4。dist-tag 漂移已实测复现（Agent Teams 三包 `latest` =
  `0.1.5-alpha.2`，与 0.1.6-alpha.1 运行时错配），且这与台账行 7（`:22`）记载的 dsh-base
  事故同源——该事故的症状是"404 + pnpm 递增重试 → 数分钟失败/超时"，用户无从归因。

### 方案 E：只收录 bundle 类包，放弃非 bundle 包 —— ❌ 否决

- **思路**：让目录只包含 `auto-review` 与两个 `agent-team-*-profile`，因为它们 `add` 即可用，
  壳零 patch 逻辑，实现最省。
- **否决理由**：违反约束 8——目录自称覆盖官方插件，却略去 browser-use 与 computer-use 的
  **全部**条目（5 个 provider），宣称面与实际面不符；且该方案并不能省掉 patch 逻辑
  （配置覆盖仍要写 `- id:` 行），省的只是量。

### 方案 F：推动上游把 6 个包改成声明 `dsh.bundle` —— ❌ 否决（可另行提出）

- **思路**：让上游为 provider 包补 `dsh.bundle.patch`，壳即可对所有包一视同仁。
- **否决理由**：违反约束 1（不修改 dsh 源码），且属上游发布节奏、不受本仓库控制；设为前置会让
  本 ADR 无限期阻塞。**可作为上游诉求另行提出**（见 §5 行动项），但不得作为本决策的前提。

## 4. 最终决策

**dsh-dock 新增一个明确标注为「dsh-dock 策展集」的官方插件目录（不得表述为"官方首批"）；
安装激活契约按目标包是否声明 `dsh.bundle` 分支——声明者只跑
`dsh plugin --profile <n> add "<pkg>@<从已装运行时推导的版本>"` 且壳**不写任何 patch**，
未声明者在此基础上再经 `plugins.rs::PatchFile` 写入一条带稳定可复算 `id` 的
`- insert:` 挂载行；同族 provider（浏览器三选一、桌面控制二选一）在目录 UI 层强制
"替换"而非"叠加"；目录元数据是"是否官方"的唯一真相源，`MarketPluginCard` 的
`owner` 启发式与其徽章/i18n 一并迁移。**

一句话理由：CLI 已对 bundle 包完成激活、只对非 bundle 包缺挂载行，
按包分支是唯一既不重复挂载也不静默漏挂的契约。

## 5. 后果与后续行动项

### 正面后果

- 两类包各自只有一条正确路径，消除"安装成功但插件不加载"的静默失败类别。
- 钉版本消除台账行 7 同源事故在官方包上的复发面。
- 互斥建模把"装第二个 provider 即激活失败"提前到 UI 层拦截，用户不再撞上游错误。
- `id` 稳定使壳生成的行在后续版本中仍可被覆盖/禁用（否则永久不可维护）。
- "官方"获得唯一真相源，AGENTS §11.3 禁双源得到遵守。

### 负面后果 / 新增债务

- **目录元数据是新的维护面**：`declaresBundle`、配套关系、互斥族、版本推导规则四项都需
  随 dsh 升级逐条复核，否则会静默腐化（这正是台账存在的理由）。
- **上游无范本**：6 个非 bundle 包中，5 个 provider 的 README **完全没有 `dsh plugin` 命令**，
  只给手工 YAML 行；唯一给出命令的 `auto-review` 用的是**源码仓相对路径**
  （`auto-review/README.md:33`：`dsh plugin --profile web add ./packages/experimental/auto-review`），
  在 npm 安装场景**不适用**。故本仓库是这些包的首次程序化安装落地者，无上游范本可抄。
- **互斥只在 UI 层**：上游无声明式互斥元数据，绕过 UI 的手工安装仍会失败。
- `experimental` 语义只能"如实呈现"而不能"代为担保"：上游明示无支持承诺，目录需承担
  用户预期管理。

### 行动项

- [ ] **元数据与徽章**（前端）：`types/market.ts` 增 `isOfficial` / `declaresBundle` / `companions` /
  `exclusiveFamily` / `experimental`；`MarketplaceView` + `PluginHub` 加策展集子视图；
  `MarketPluginCard.tsx:50` 的启发式改读目录元数据，保留 `:61-62`/`:87` 徽章与
  `t.market.officialCoreTitle`，补"不得双源判定"单测。
- [ ] **激活分支**（Rust，`plugins.rs`）：新增按 `declaresBundle` 分支的安装入口；
  非 bundle 路径经 `PatchFile` 写 `- insert:` 行，`id` 由包名规范化派生并保证幂等
  （重复安装不得产生第二行）。
- [ ] **配置覆盖**（Rust，`plugins.rs`）：实现"先读该行完整生效 config → 整体写回"的辅助函数，
  禁止构造字段残缺的 `config` 行；补"写回后其余字段逐字不变"单测（锚 `docs/roadmap.md:31`）。
- [ ] **钉版本**（Rust）：从已装 dsh 运行时推导期望版本/tag，安装弹窗显示**确切版本**；补"运行时
  0.1.6-alpha.1 时不得落 0.1.5-alpha.2"回归测试（复现见审核报告附录 A TEST F）。
- [ ] **互斥**（前端）：浏览器三选一、桌面控制二选一，装第二个时呈现"替换（先卸后者）"流程；
  文案写明替换会中止当前 provider 的会话级资源。
- [ ] **台账登记**（文档）：扩充 `dsh-behavior-ledger.md` 行 7（`:22`）与行 13（`:28`），按 5 列表头
  （`:14-15`）登记官方 `dsh-experimental-*` 包集及 `dsh.bundle` 分类与配套，§三 追加复核记录。
- [ ] **文档收口**（文档）：`docs/plans/...-plan-review.md` 的 B1/B2/B3 以本 ADR 收口；并记录本 ADR
  **不需要**改 `docs/contract.md` 或升 `MANIFEST_FORMAT`（决策全在 `$DSH_HOME` 管理面内）。
- [ ] **上游诉求**（可选，非前置）：向 dsh 提出为 provider 包补 `dsh.bundle.patch`（方案 F）。

## 6. 复审条件

- **dsh 升级**（任何版本变化）：逐条复核 §1.5 的 `dsh.bundle` 分类表、§1.6 的 dist-tag 表、
  §1.7 的 `PatchOptions`/`EntryOptions` schema 与 `ensureId` 行为——任一变化都会使本契约静默失效。
- 上游修改 `reconcilePlugins` 的判定依据（现为"按已安装状态"，`apps/cli/src/plugin.ts:59-91`），
  例如改为按依赖 diff 或引入显式激活标记；或 `dsh plugin` 新增激活相关旗标
  （现仅有 `--profile`，其余逐字转发 pnpm，`apps/cli/src/args.ts:190-201`）。
- 任一被收录包**新增或移除** `dsh.bundle` 声明（例如方案 F 落地），或新增/合并 provider
  导致互斥族变化；或上游开始提供声明式互斥/配套元数据——届时目录可退化为消费方。
- AGENTS §11 写入边界或 ADR-0011 来源白名单被修订。

## 7. 第二次修订（2026-09-16）：从「包目录」到「能力开关」

> 触发：维护者实测反馈「实验性功能的实现效果与交互逻辑体验极差」。以下缺陷逐条可复核，
> 不是风格偏好问题。本次修订**取代** §4 中「目录 UI 层强制替换」与 §5 中「安装弹窗」的
> 呈现口径；§1–§3 的事实与评估、§2 的激活契约（按 `dsh.bundle` 分支）**不变**。

### 7.1 缺陷（v1 实测）

| # | 缺陷 | 证据 |
|:--|:---|:---|
| D1 | **粒度错**：同族三选一渲染成三张并列卡片，用户看不到"当前是哪一个"，换后端只能再点一次安装 | `official_catalog.rs:195-264` 8 条并列 `CatalogEntry` |
| D2 | **只有开、没有关**：唯一动作是安装。停用要去「已安装」页按**包**找 toggle；`AutoBundle` 类包（auto-review）命中的是它贡献的行，语义更远 | `OfficialLab.tsx:202-222` 单个安装按钮 |
| D3 | **状态只到"包装没装"**：`installed` 取 `dependencies` 布尔，不反映挂载行是否存在、是否被 `disabled`、同族冲突的当前持有者 | `commands/plugin.rs:106-121`；`CatalogStep.installed` |
| D4 | **实现细节污染第一阅读层**：「需写入挂载行」「由 dsh 自动激活（不写配置行）」「钉版本：`<spec>`」是维护者信息 | `OfficialLab.tsx:56-64` |
| D5 | **卸载残留**：写行有正向原语、**无反向原语**。包被卸掉后 `insert` 行仍在 → 悬空挂载行（dsh 启动加载失败面） | `plugins.rs` 无删行实现（`apply_catalog_insert_row` 只增不减） |
| D6 | `requiresNoteZh` 含 Markdown 反引号，纯文本直出（"`model` 必填"） | `official_catalog.rs:209,227,261` |
| D8 | **子集档被误报为互斥冲突**：Agent Teams 的两档是子集关系（自建档 = `[host 层]` ⊂ Web 档 = `[host 层, web 层]`），装 Web 档时子集档的包必然也齐 → 面板报「后端冲突」，而这是**完全正常的配置**。该误报只对真互斥族（浏览器 3 选 1 / 桌面 2 选 1）成立 | 2026-09-16 真机（`cargo tauri dev` + dev home 实测）暴露 |
| D7 | **重复挂载**：`activation` 是**列目录时**算的，而那时包还没进 `node_modules` → 任何包都被判成"未声明 `dsh.bundle`"。于是 profile 层包（`auto-review` 等）被误判为需要写行，装完多写一条 → **同一插件挂两份实例** | `commands/plugin.rs` 采集 `declared_bundles` 只遍历已装包；`official_catalog::activation_for` 对未装包恒得 `InsertRow` |

### 7.2 决策

1. **呈现单位 = 能力（capability），不是包**。四项：`auto-review` / `browser-use` /
   `computer-use` / `agent-team`。同族后端降级为**能力内的变体（variant）**——互斥由此
   在**同一张卡内**表达为"当前后端"，用户不再需要理解"三选一"。
2. **开关语义 = 行级 `disabled`，不是卸载**（ADR-0009 例外 #3 的既有通道）：
   - ON = 包已装 ∧ 行存在 ∧ 行未 disabled；
   - OFF = `set_plugin_disabled(id, true)`，**保留包**——秒级可逆、不重下 pnpm；
   - 「移除」是**独立的次要动作**：删行 → 逆序卸载包（走确认框）。
   用卸载冒充"关闭"会把开关变成分钟级重操作，且与「已安装」页语义重复。
3. **新增反向原语 `remove_official_patch_row(profile, row_id)`**：按 `id` 删除壳写过的
   挂载行，删除后**回读 dump-config 自证该行已不在组合树**（与写行的自证对称）。
   仅在行确由壳写入时调用。登记册 §一 同步。
4. **状态由后端一次算全**（`CapabilityState` = `off` / `on` / `disabled` / `partial` /
   `conflict`）：状态是「包 × 行 × disabled」的函数，**前端不得凭 `installed` 猜**（§2.8 禁双源）。
5. **实现细节折叠**：spec / 激活方式 / 行 id / 版本提示进「详情」，默认不出现在第一阅读层；
   启用确认框显示**确切版本**（§2.4 的原始要求，v1 未落到弹窗）。
   > **2026-09-17 修订（维护者裁定「实际上就是插件，名字使用插件名就行了」）**：本条的
   > **「包名」一项从折叠区上移到第一阅读层**——一个"后端"本就是**一串插件**（browser-use
   > 三档都是 `dsh-browser-use` 基座 + 各自 provider；Agent Teams 的 Web 档再加
   > `-web-profile`，而"自建档"**就是基座本身**），所以卡片不再用我们发明的
   > 「Web 档 / 复用已装的 cua-driver」称呼后端，改为**行首直接写插件名**，并区分
   > 「该档独有的包」与「另装共用基座包」（规则纯函数化：
   > `experimentalCapabilities::variantPackageRoles`，有单测）。
   > 理由是**可对照**：用户要拿这些名字去对 `node_modules`、`cordis.patch.yml` 与
   > 上游 README，我们自造的名字在这里全是噪音。spec / 激活方式 / 行 id / 版本提示
   > **仍在「详情」**（它们才是排障细节）。同一轮版面规范：刻度只用既有 token、
   > 圆角按角色、品牌色只承担"选中/主操作"、状态色只承担状态、各档共同前置去重成一条、
   > **行内状态徽标按变体自身状态取词**（`activeVariant` 含"已就位但停用"，照抄「已启用」
   > 会与卡片头的「已停用」自相矛盾——版面复盘真抓到）。
   > **2026-09-17 第二次修订（维护者验收 v2 后打回：「进去占用的空间也太多了，一屏只能看到
   > 一个工具，点击详情还又加长卡片内容」）**：**清单与详情分栏**，详情**不再内联展开**。
   > 真机实测（1280×820 默认窗口）v2 整页 1897px、四张卡 341/530/360/278px、一屏只装得下
   > **1** 项。病根不是"卡里字多"，而是把清单与详情塞进同一个纵向流——详情于是只有
   > "内联展开（越点越长）"或"藏起来"两条路，两条都错。v3 定为：
   > · 左栏**清单行**恒紧凑（图标 · 名称 · 状态徽标 · 当前插件名 · 开关），四项一屏全见；
   > · 右栏**详情面常驻**（插件与后端切换 · 前置 · 排障细节 · 移除），没有"展开"这件事，
   >   **点行只换右栏内容，左栏恒不动**（1280×820 实测四行 66/66/66/52px）；右栏不裁剪
   >   内容，故内容多的能力（三后端那一项）整页仍会长出约 70px（820 → 889）；
   >   窄窗口（<1024，应用最小 960）降级为下钻 + 返回；
   > · 详情面内的「实现细节（排障用）」默认收起（它是出问题时才看的东西），
   >   展开只增长右栏——与 v2 被点名的"内联展开顶掉后面卡片"是两件事；
   > · 共同前置与**各档共用基座包**都提到面板级只讲一次（三条变体行里重复三遍
   >   `dsh-browser-use` 把"真正区分各档的那行"淹掉了；规则纯函数化
   >   `commonPrerequisites` / `commonSharedPackages`，有单测）。
   > 同轮真机复盘抓到的 bug：面板用 `variant.prerequisiteMissing !== null` 判"宿主前置缺失"，
   > 而 dev mock 少写了这个字段 → `undefined !== null` 为真 → **四行全亮"前置缺失"红灯、
   > 开关全灰**。改用 `Boolean(...)`，并把 mock 补齐成与 serde 同形（`devMock.ts` +
   > 闸门 §⑨：变体数与 `prerequisiteMissing:` 出现次数必须相等，且至少留一条真值）。
   > 门禁（`experimentalCapabilitiesGate.test.ts` §⑧）钉死这条形态：源码不得再出现
   > `openDetail` / 内联展开 / `AnimatePresence` 高度动画，必须两栏、行内不得有块级段落、
   > 长描述与实现细节不得进清单行。

   > **2026-09-17 第三次修订（维护者：「dsh 新版本已经把智能体团队插件内置了，我们的实验性功能
   > 可以把那儿撤了吧」）——确立「dsh 自带的能力，dock 不代管」**：
   > dsh **0.1.6-alpha.2** 起把官方实验层作为 **optional bundle** 随安装包下发（上游依据：
   > `packages/boot/app-boot/src/profile.ts` 的 `OPTIONAL_BUNDLES` ＋ 设计笔记
   > `.agents/notes/implemented/process/2026-09-15-shipped-optional-bundles.md`）：随包下载、
   > 默认关、在 **dsh 自己的插件页**里开关、且被 dsh 视为 **`not-removable`**。该笔记同时
   > **明确否决**了"由某个面板按名字从 registry 安装官方 bundle"这条替代路径——那正是本模块
   > 原来在做的事。2026-09-17 实测（本机正式档 `dsh 0.1.6-alpha.2`）：Agent Teams 两个 bundle
   > 已在 `<dsh>/node_modules/@deepseek-ai/` 在册，且出现在该 Profile 的 `dsh.profile.bundles`
   > 里（由 dsh 自己的插件页开启，**不在** profile 依赖中）。
   >
   > 规则（三条理由：dsh 会拒卸载；profile 里再装一份会**遮蔽**自带的那一份且版本可能不同；
   > 一份能力两套开关必然互相打脸）：
   > · **不安装、不钉版本、不写挂载行、不卸载**——动作面整体退出；
   > · 界面只做三件事：说清"现在归 dsh 管、开关在哪"、列出**随 dsh 自带的包名**、
   >   在没有遗留副本时**不出现任何按钮**；
   > · **遗留副本例外**：本 Profile 自己还持有那些包（dock 早期装的）时，给**唯一**一个动作
   >   「清理旧副本」（走既有的破坏性确认链）——不清理它会持续遮蔽 dsh 自带的那一份。
   >
   > 落地：**判据取"这次安装实测"而不是写死的旗标**——某能力的**每个包**都能在
   > `<engines>/dsh-runtime/node_modules/<包>` 找到 ⇒ 这个安装自带它
   > （`official_catalog.rs::installation_shipped`，纯函数 + 单测）。理由：同一台机器上
   > dev 档引擎 0.1.6-alpha.1 **不带**、正式档 0.1.6-alpha.2 **带**，旗标必然在其中一边说谎；
   > 实测判据还会**自动跟上** dsh 后续把更多实验能力内置的节奏（无需改代码）。
   > 视图加 `shippedByDsh` / `legacyCopy`；自带的走**独立详情面**（不套"互斥变体"模型——
   > dsh 侧是两个**各自可开关**的官方插件，宿主层与 Web 层并不互斥）；
   > `legacyCopy` 的判据与 `planRemove` 的能力面对齐——bundle 贡献的行我们删不掉，**不能**
   > 算成可清理的痕迹（否则「清理旧副本」是个点了没反应的假按钮）。
   > 探测不到安装目录 = 空表 ⇒ 一切照旧由 dock 策展（**宁可多管，也不谎称"dsh 已内置"**）。
   > **每次 dsh 升级的复核点**：安装包 `@deepseek-ai/dsh` 的 `dependencies` 里有没有新的
   > `@deepseek-ai/dsh-experimental-*`（＝ `OPTIONAL_BUNDLES` 增项）。今天仍是"要装才有"的
   > 发布包：`auto-review` / `browser-use`（三档）/ `computer-use`（两档）。

   > **修订（2026-09-20，ADR-0028 §6.2 维护者裁定）**：dsh 已把它接管为 optional
   > bundle 的能力，**本壳不再摆任何面板呈现**——能力目录按 `shippedByDsh` 过滤后展示
   > （`lib/pluginCatalog.ts::dockCuratedCaps`），独立详情面/自带徽标/「清理旧副本」整条
   > 链移除。用户开关它的地方只有 **dsh 自己的插件页**；启用后其层写进本档
   > `dsh.profile.bundles`，在插件列表与模板层同列展示（内置 + 层 N + 版本随 dsh）。
   > 契约字段 `shippedByDsh` / `legacyCopy` 后端仍下发（判据不变），本壳不再消费。

   > **修订（2026-09-22，维护者「如果 dsh 官方内置了或者作为可选安装，dock 就不展示
   > 操作入口，只做官方的补充」——ADR-0020 §7.3 升级复核点首次触发）**。上游事实
   > （dsh 0.1.7-rc.1 本机正式档引擎实测）：① Agent Teams 收拢为**单个** optional
   > bundle —— `@deepseek-ai/dsh-experimental-agent-team-profile` 自述 "Agent Teams
   > collaboration, tools, and Web UI in one experimental bundle"，独立的
   > `agent-team-web-profile` **退役**（npm 终版 `0.1.6-alpha.2`，0.1.7 起无此包）；
   > 上游 `OPTIONAL_BUNDLES` = `agent-team-profile` ＋ 新增的 `voice-input-bundle`。
   > ② 目录引用退役包 ⇒ "每个包都在册"的全量判据在 0.1.7 上恒不成立 ⇒ agent-team
   > 重新被判为 dock 策展、面板重新摆出操作入口（且默认首档会去装一个不存在的包版本）
   > ——正是本次修订要关掉的形态。落地三件事：
   > · **目录对齐上游**：agent-team 从「Web 档 / 自建档」两变体收成**单变体**
   >   （`agent-team-profile`）；旧引擎（≤0.1.6-alpha.1，安装不自带）上它只等价于
   >   宿主层，是上游事实而非壳的裁剪；
   > · **判据加固**：`installation_shipped` → `installation_provided`，从"运行时在册"
   >   单信号扩为 **安装清单声明 ∨ 在册**（读 dsh 自己的 package.json `dependencies`，
   >   上游 plugin-manager `installation.dependencies` 同口径）——半途装机、符号链接
   >   农场等磁盘布局抖动不再把"官方提供"误判成"没有"；
   > · **回归护栏续命**：`resolve_capabilities_with` 开注入口，"子集档"（自建档 ⊂ Web 档）
   >   在上游收拢后真目录里已无此形态，其判定逻辑的回归测试迁到**合成目录**。
   > 判据的保守方向不变：探测不到 ⇒ 一切照旧由 dock 策展（宁可多管，不谎称自带）。

6. **前置条件与价值主张进目录元数据**（`summary` / `prerequisites`，人类语言）。
   > **2026-09-17 补**：**不得含任何 markdown 标记**——面板没有 markdown 渲染器，
   > 反引号与 `**` 都会原样显示（D6 只挡住了反引号，`unlocks_zh` 里的 `**权限最高**`
   > 一直漏到用户眼前）。门禁：Rust `user_facing_copy_has_no_markdown_markup`
   > ＋ 前端 `dictCopyHygiene.test.ts`（扫两本字典的求值叶子）。
7. `list_official_plugins` **改名并改形**为 `list_experimental_capabilities`（返回
   `CapabilityView[]`）。无外部消费者，仅本仓库前端；连带 `ipc.rs` 闸门 fixture 与登记册。
8. **区分「互斥变体」与「子集变体」**（2026-09-16 真机暴露，D8）：变体状态里新增
   `subsumed_by`——本变体的包若是另一个**已就位**变体的真子集，则本档显式为「已包含」，
   **不提供独立开关与移除**（它的包就是超集档的基础层，单独关掉会把超集档拆坏），
   且**不参与冲突判定**（冲突只数"没被包含"的档）。真互斥族仍必须报冲突——
   正反两条都有回归测试（`subset_variant_is_subsumed_not_conflicting` /
   `genuinely_exclusive_variants_still_conflict`）。
9. **删掉「registry `latest` 错配提示」这条死路径**（2026-09-16 独立评审核出）：
   `latest` 只能联网取，而该命令**禁网**（唯一网络面 = `updates.rs`），故 `latest_by_package`
   在生产路径**恒为空** ⇒ `version_skew_notice` 永远不触发。§2.4 的实质要求
   （确认框显示将要安装的**确切版本**）由确认框直接列出 spec 满足。留下恒不可达的分支
   只会制造"看着有、其实没有"的假象——与本仓库最忌讳的那类失败同源。

### 7.3 后果

- 目录元数据维护面扩大：每能力新增 `summary` / `prerequisites` / `variants` 三项，随 dsh 升级复核。
- **停用 ≠ 卸载**这条双重语义必须让用户看见：卡片状态区分「已启用 / 已停用（仍占磁盘）/
  未启用」，移除入口写明"卸载包并删行"。
- 破坏性 payload 变更：`CatalogRow`/`CatalogStep` 退役，`CapabilityView`/`VariantView`/`StepView` 接手。
- 不新增网络面、不改 `docs/contract.md`、不升 `MANIFEST_FORMAT`（改动全在 `$DSH_HOME` 管理面内）。

### 7.4 D7 的闭合：分类只能在**写之前、装之后**判定

激活方式是读目标包 `package.json` 的 `/dsh/bundle/patch` 得到的（§2.8 唯一依据），
而安装前该文件不存在——"安装前判定"只是"没装"的同义词。故：

- `apply_official_patch_row` **写行前当场重判**：声明了 `dsh.bundle` → **拒绝写行**，
  返回 `RowWriteOutcome{ autoActivated: true }`（如实告知"由 CLI 激活，壳未写行"，
  而不是静默什么都不做）；
- 目录里的 `activation` 字段降级为**展示用**（详情折叠里的实现说明），不再驱动写行决策。

### 7.5 上游锚点（本次修订新增，2026-09-15 实查 @ `0d1f5000`，入台账）

| 事实 | 位置 |
|:---|:---|
| 帧格行级 `disabled` 的语义：**卸载插件但不删除其 Cordis 配置项**，改回后重新加载 | `docs/cordis-tutorial/06-composition-and-hmr.zh.md:16,19` |
| 字段定义「Prevents this entry and descendants from running.」+ 父级继承 | `vendor/loader/src/config/entry.ts:18-19,72-82,133-136` |
| 真实先例：CLI 自己用 `{ id, disabled: true }` 关遥测行 | `apps/cli/src/profile-boot.ts:171-173` |
| `agent-team-profile` 的层 patch **除插入 2 行外还停用 4 条旧 subagent 行** → 层副作用无法用行级开关回滚 | `packages/experimental/agent-team-profile/cordis.patch.yml:4-14,16-29` |
| `dsh plugin remove` **同时把该包从有序层列表 `dsh.profile.bundles` 移除** | `agent-team-profile/README.zh.md:37`；`agent-team-web-profile/README.zh.md:37` |
| `@deepseek-ai/dsh-browser-use` / `-computer-use` **无 `dsh` 字段** → 两者都是普通行（非层） | `packages/browser-use/browser-use/package.json`、`packages/computer-use/computer-use/package.json`（实查 `dsh: null`） |
| 浏览器后端 3 选 1 / 桌面提供方 2 选 1：共享服务**拒绝任何第二次注册，含同名实例** | `docs/subsystems/browser-use.zh.md:9,17`；`docs/subsystems/computer-use.zh.md:9,16` |
| 上游插件清单 GUI **只读**（`EnablementKind` 无写接口）→ 写路径只能自建 | `packages/client/ui-settings-plugin-inventory/.../PluginInventorySettingsTab.tsx:20-27,184` |

**不在策展集内（有意）**：`inspector`（需 `--patch` 手工 overlay + 先构建，不能经
`dsh plugin add` 成为 profile 层）、`ptc-runtime-python`（需自备 CPython ≥3.10，且与
workflow 互斥、无随附 profile 启用）、`webworker-*` / `browser-use-runtime`（库或构建工具，
不可挂载）。

## 8. 第三次修订（2026-09-16）：挂载行**载荷契约**、宿主前置硬门与启动可见性

### 8.1 事故

用户装完 3 个实验包（`browser-use` / `computer-use` / `chrome-devtools-mcp`）后重启，
**工作台再也起不来**：启动页停在 step3「等待服务响应超时」，`dsh-shell.log` **0 字节**，
诊断台只能显示「详情见日志」——而日志是空的（用户与开发者同时瞎掉）。

隔离复现（`cp -Rc` 克隆 dev home，未动现场）后拿到真实死因，退出码 1、34 s：

```text
Error: dsh: plugin tree failed to load: failed to apply loader entry include (cordis:include): loader entries failed to apply
Error: failed to apply loader entry dsh-dock--…-computer-use-cua-driver-mcp (@deepseek-ai/dsh-experimental-computer-use-cua-driver-mcp): mcp-client(cua-driver-mcp): initial connection or tool synchronization failed
Error: spawn cua-driver ENOENT
Error: failed to apply loader entry dsh-dock--…-browser-use-chrome-devtools-mcp (…): Cannot read properties of undefined (reading 'mode')
TypeError: Cannot read properties of undefined (reading 'mode') at validateBrowserMcpConfig (…/browser-use-runtime/lib/types/mcp.js:26)
```

### 8.2 缺陷

| # | 缺陷 | 根因 |
|:---|:---|:---|
| D8 | 挂载行**只写 `{id, name}`** → 上游 MCP provider 的 `Config` 是**必填**（浏览器族 `mode`），`apply` 直接抛 TypeError | §3.1.4 的「挂载行不写 `config`」口径对**这类包**不成立：`config` 虽是整体替换语义，但**必需字段**必须由壳写 |
| D9 | `cua-driver-mcp` 未检前置即允许安装：缺 `cua-driver` 可执行文件时 `spawn … ENOENT` → 整棵树失败 | 上游原文即"仅在 Cua Driver 已安装时选用"；壳只把它写成提示语，**没有门** |
| D10 | `BOOT_STALL` = 20 s，而 dsh 报此类错误要 **34 s** → 壳在 dsh 开口前判「卡死」并 SIGKILL，日志全空 | 停滞判定把"还没开口"当成了"已经死了" |

### 8.3 决策

1. **行载荷单源**：`official_catalog::required_row_config(package)` 是挂载行 `config` 的
   **唯一来源**（现 3 条：两个浏览器 MCP 档 `{mode: launch, headless: true}`；cua-driver
   MCP `{command: cua-driver, args: [mcp]}`），`plugins::ensure_catalog_insert_row`
   只负责写对。依据 = 上游各包 README 的「Minimal configuration」原文。
2. **既存坏行可就地补齐**：`config` 缺字段 → 补，值不同 → 改；用户在该行手写的其它键
   不动、不重建第二行、幂等；命中即为 `changed: true`——这正是「修复」能把起不来的
   profile 修好的机制。
3. **宿主前置是硬门**（前后端各一道）：`required_command(package)` 声明前置可执行文件，
   命令层经 **dsh 子进程同源 PATH**（`resolve::command_available` + `dsh_child_path`）探测；
   - `list_experimental_capabilities` 下发 `VariantView.prerequisite_missing`（后端文案）；
   - 前端**禁用开关**并把原因露在卡面上（不折叠在详情里）；
   - `apply_official_patch_row` **同样拒绝写行**（前端是呈现，不是闸门）。
4. **静默 ≠ 已死**：`wait_for_ready` 新增 `stall_grace`（`BOOT_STALL_GRACE` = 25 s）——
   停滞到点先宽限，期间进程若自行退出即判 `Exited(code)` 并取回真实错误栈；宽限用尽才
   `Stalled`。20 + 25 = 45 s 覆盖实测 34 s，硬上限仍是 `BOOT_TIMEOUT`（90 s）。
5. **错误卡点名 + 就地修**：`boot_failure` 新增 `PluginRowFailed { row_id, package, cause }`
   （解析上游那句 `failed to apply loader entry <id> (<pkg>): <cause>`，**逐处扫描**——同段
   日志里 `include` 那条不是行 id）；建议文案点名行 id；行归壳所有（`dsh-dock-` 前缀）时
   下发 `actions=["quarantine_plugin_row"]` + `quarantine={profile,rowId}`，前端一键
   「移除该行并重启」（`remove_official_patch_row` → `terminalAction("retry")`）。
   **不给 `retry`**：同一行必然再失败，摆一个必然失败的按钮等于教用户白点一次。

### 8.4 后果

- **正面**：目录里每个可安装变体现在都"装得上且起得来"（真机等价验证：克隆体写入带
  `config` 的 chrome-devtools 行 → 就绪 URL 正常；去掉 `config` → 复现退出码 1）；
  起不来时用户第一次能看见**真实死因**并一键回到可用状态；该类失败不再落 `unknown` 兜底。
- **负面 / 新增债务**：
  - 行载荷表随上游 README 漂移（三条，锚点入台账），dsh 升级须复核；
  - 前置探测按 PATH 判定，装在与 dsh 子进程 PATH 不同的位置时会被判「缺」（保守方向：
    宁可挡住，也不产生起不来的 profile）；`resolve_toolchain` 失败时同样按缺处理；
  - **自动隔离兜底未做**（启动失败即自动移除 + 单次自重试），本轮给的是**用户可见的
    一键**；若同类事故再现则重开。

### 8.5 复审条件

- 上游为 MCP provider 补默认 `mode`、或让 `Config` 变可选 → 行载荷表可减；
- `cua-driver` 改为随包分发（如同 native 档自包含）→ 前置门可撤；
- 启动等待口径再调（如 dsh 改为「失败即快退」）→ `stall_grace` 复评。

---

## 9. 面板落点迁出（2026-09-20，指针）

本 ADR §7 定的**呈现单位、开关语义、状态口径与硬门全部不变**；变的是面板**落在哪个
视图**：能力面板自**插件中心**（`PluginHub` 的 `official` 子页）迁入 **Profile 详情页**
第 5 个 tab（与「插件列表」并列），插件中心收敛为市场 / 已安装两子页，
「外挂插件」更名「插件列表」。理由与实现见 **[ADR-0028](0028-capability-panel-moves-into-profile.md)**
（档位自持下拉 → 受控 prop，消除"面板档位与选中档不一致"的错写入口）。
本节只作指针，**禁双源**：面板交互的一切判据以 ADR-0028 与其闸门
（`frontend/src/__tests__/experimentalCapabilitiesGate.test.ts`）为准。

§2.8 的「同一能力操作入口单源」在 **ADR-0028 §6（第二批，清单打标合并）** 有直接推论：
「插件列表」里的实验性行**不持开关/卸载**（卸载必须连带清挂载行，编排只在能力面板），
只给「去开关」跳转——合表之后两个视图不是"各一套入口"，而是"清单看事实、面板管动作"。
