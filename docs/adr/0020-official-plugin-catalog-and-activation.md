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
