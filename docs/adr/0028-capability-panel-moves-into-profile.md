# ADR-0028：实验能力面板迁入 Profile 详情页——开关与配置同作用域

- **日期**：2026-09-20
- **状态**：已接受（2026-09-20 维护者会话裁定：迁入 + 插件中心子页下线 + 「外挂插件」更名「插件列表」）
- **提出人**：guan（AI 协作）
- **相关方**：`frontend/src/components/market/ExperimentalCapabilities.tsx`、`frontend/src/components/market/PluginHub.tsx`、`frontend/src/components/profiles/ProfileDetailPane.tsx`、`frontend/src/pages/ProfileManager.tsx`、`frontend/src/lib/pluginCatalog.ts`（第二批）、`frontend/src/__tests__/experimentalCapabilitiesGate.test.ts`、`frontend/src/__tests__/pluginCatalog.test.ts`（第二批）、`frontend/src/__tests__/pluginListMergeGate.test.ts`（第二批）
- **关联**：ADR-0020（官方策展集与能力开关，§9 已回填本 ADR 指针）、ADR-0026（安全模式读写同一文件）、AGENTS §4.3（跨窗真相源）、§8（增量生成）

---

## 1. 背景与问题

实验能力面板自 ADR-0020 §7 起落在**插件中心**（`PluginHub` 的 `official` 子页），
与「外挂插件」分属两个作用域：

1. **Profile 对应靠自持下拉，与选中档无关**。面板的档位是组件内 state，初值取
   `getActiveProfile()`（`ExperimentalCapabilities.tsx:182-200`），而「外挂插件」
   那侧是 ProfileManager 的 `selectedName`。用户在列表选中 A 档、切到插件中心，
   看到的却是运行中的 B 档——开关真会写进 B 的配置。这是唯一会**改错 profile** 的
   入口（2026-09-20 排查「外挂插件与实验能力启停不联动」时坐实）。
2. **两面板状态不联动**。行身份判据缺陷（`official_catalog::row_state_for` 不认
   自命名 bundle 行，2026-09-20 已修）之外，落点造成的**作用域错配**仍在：同一文件
   的两套开关相距一个顶级视图，任何"两边对账"都依赖用户记得自己操作的是哪一档。

## 2. 约束与硬指标

- AGENTS §4.3 红线 3：跨窗口真相源——各窗独立 JS runtime；但**同窗内**应共享单一
  取数/刷新链，不靠切页重挂兜底。
- ADR-0020 §2.8 禁双源：同一能力的状态判定只许一处；同一能力的**操作入口**同理
  （双入口 = 两套"当前档位"心智）。
- 壳不感知产品身份、前端运行时禁新网络请求等通用约束不变。
- 增量生成（§8.1）：本次只做**搬面板 + 下线子页 + 改名**；清单打标合并（底座 /
  第三方 / 实验性三类一行）是下一次意图。

## 3. 备选方案及评估

### 方案 A：迁入 ProfileDetailPane 第 5 个 tab，插件中心子页下线 —— ✅ 最终采纳

- 思路：`ExperimentalCapabilities` 改为受控组件（`profile` prop 取自
  `ProfileDetailPane` 的 `name`），作为「实验能力」tab 与「插件列表」同页；
  `PluginHub` 删掉 `official` 子页，收敛为市场 / 已安装两子页；「外挂插件」tab
  更名「插件列表」。
- 优点：开关与它所改的配置（`profiles/<名>/cordis.patch.yml`）回到同一作用域，
  profile 对应问题从"要小心的边界"变成不存在；两 tab 同页可共享一次
  `get_plugin_rows` 回读链（能力操作完成后 `onChanged` → 父级 `reload`），
  此前讨论过的"面板间失效通知"问题随之消解；插件中心回归"跨档市场"本职。
- 代价/风险：组件失去自持档位能力——不能在面板内临时切档操作别的 profile。
  裁定：这是特性不是损失（换档回左侧列表选，心智单一）。
  **本项在落地自查中兑现为真实副作用**（见 §4.1）：档位从"面板内自持"变成"父级受控"后，
  换档入口搬到了面板管不到的地方——原先靠 `Select disabled={run !== null}` 挡住的
  "在途换档"重新可达。已在同一批次补三道闸门钉死。
- 对照约束：§2 全部满足；单一入口、同作用域。

### 方案 B：保留插件中心子页，仅把下拉同步到 selectedName —— ❌ 否决

- 思路：最小改动，子页不动，`profile` 初值改传 `selectedName`。
- 否决理由：双入口仍在，两面板仍分属两个顶级视图，"对账靠切页重挂"的脆性不变；
  且违反 §2 的入口单源精神（同一能力两处开关 = 两套档位心智）。

### 方案 C：插件中心保留「跨档总览」只读视图 —— ❌ 否决

- 思路：子页降级为只读汇总，操作一律去 Profile 详情页。
- 否决理由：同一能力两套呈现（只读汇总 + 可操作面板）是另一种双源——汇总的状态
  一样要回答"哪个档"，问题没有被搬走，只是换了个名字留下。

## 4. 最终决策

实验能力面板迁入 `ProfileDetailPane`，作为与「插件列表」并列的 tab；插件中心
`official` 子页**下线**（单一入口）；「外挂插件」更名「插件列表」（resolve
与插件中心「已安装」（跨档总览）的同名歧义）。面板为受控组件：
`profile: string`（必传）、`onChanged?: () => void`（写操作完成后父级刷新）。

### 4.1 迁移带出的竞态：在途换档（2026-09-20 落地自查补）

**缺陷形态**：旧档 A 的动作在途 → 用户点左列表切到 B。旧版这条路径被面板内下拉的
`disabled={run !== null}` 挡着；**迁入后换档入口在父级 `ProfileRow` 的 `onSelect`，
不受面板 busy 约束，于是重新可达**。且原有的 `loadSeq` 令牌**拦不住它**——A 的回读
自己会取到更大的 `seq`，"最后一次请求"反而成了旧档的数据。

**三个症状**（都按"两档能力 id 同名"成立）：① 清单串档——左列表显示 B、清单是 A 的事实；
② 失败串档——A 的失败挂在 B 的同名能力上，用户按它算的计划会装进 B；③ 假重启提示——
B 档弹"配置已变更"，把用户骗去重启一个没被改过的 Profile。

**处置**（同一批次，三道闸门 + 效果核验）：

1. `load()` 开头按**最新档位**（`profileRef`）早退：旧档的回读整体作废，不占令牌、不发请求；
2. 动作结果**落账前复核档位**：换了档就整体丢弃（含 `onNotice` / `setDirty` / `failures`）；
3. `finally` 里的 `onChanged` 与回读同挡——它是发起时那一版 `reload`（闭包钉着旧档名），
   迟到调用会把旧档的详情/行表写进父级此刻显示的新档页面；进度显示按 `run.profile === profile`
   归属（两档 id 同名，单看 `capId` 必然把转圈画到新档头上）。

闸门落在 `experimentalCapabilitiesGate.test.ts` §⑩（含"先红后绿"核验：拆掉守卫后四条断言
逐一失配）。**通用教训**：把状态从组件内提到父级受控时，组件内原有的"护栏"（这里是
`disabled`）会随之失效，必须逐条确认护栏是否仍有人兜——护栏跟着状态走，不跟着组件走。

## 5. 后果与后续行动项

### 正面后果

- 错写 profile 的入口关闭；两面板状态天然一致（同档、共享回读）。
- `PluginHub` 收敛为两子页，`onRestart` prop 随子页下线移除
  （重启入口复用 ProfileDetailPane 经 ProfileManager 的既有确认链）。

### 负面后果 / 新增债务

- 面板失去"在面板内临时切档操作别的 profile"的能力——换档须回左侧列表（已裁定为特性）。
- 面板内多了一层"档位是否还在"的判断（§4.1 三道闸门）：跨档动作的归属规则从"组件自洽"
  变为"必须与父级档位对账"，后续任何跨档异步都需照此办理。

### 行动项

- [x] ADR-0028 立档（本文件）。
- [x] 面板迁移 + 子页下线 + 改名（随本 ADR 同一批次落地）。
- [x] 在途换档竞态处置 + 闸门（§4.1；含"先红后绿"核验）。
- [x] 文档收口（随本批）：ADR-0020 §9 指针（禁双源）、`docs/acceptance-2026-09-16.md`
      入口与验收步骤改指新路径（该文档尚有 🖐 待维护者手点项，旧路径会把人带进已下线的子页）、
      两份字典里点名旧 Tab 名的文案（`desktopRuntimeNote` / `hiddenLayersHint`）。
- [x] **清单打标合并**（§6；与面板同批开工的"下一次意图"，2026-09-20 维护者会话
      裁定三项开放问题后落地）：`PluginKind` 派生底座 / 第三方 / 实验性三类、
      底座组合 tab 退役、层序入列表。
- [ ] **另案 ADR**：「关掉」语义不对称（`toggle_off_supported` 对无副作用 bundle
      层误判为"须卸载"）——auto-review 实测层无副作用，判据待从"激活方式"改为
      "层副作用事实"。

---

## 6. 第二批：清单打标合并（2026-09-20 落地，§5 行动项第 5 条）

维护者裁定（三项开放问题，原样落档）：

1. **层序保留**。底座组合 tab 的唯一独有信息是 `dsh.profile.bundles` 的栈序（后层覆盖
   先层）。平铺列表不丢它：合并行带「层 N」标（N = 组合序，title 带总数与覆盖语义），
   层栈成员排在列表最前（按栈序），其余条目保持依赖序在后。
2. **dsh 自带的实验功能算「内置」**。Agent Teams 两层同时是"dsh 已内置"与"实验能力"：
   主标「实验性 · 多智能体协同」，**再补一枚「内置」标**；内置一律不允许卸载（本就不给
   控制面）。实验性**依赖**（用户经面板装的那份）不补内置标。
3. **「实验性」标到具体能力**（「实验性 · 多智能体协同」，不只"实验性"）：用户能立刻
   知道这个包归哪个开关管。归属判据 `capabilityOfPackage`（`lib/pluginCatalog.ts`）按
   **任一变体的任一步**反查——自建档的标识包就是基座本身，只查"主包"会把它漏标成第三方。

三项落地决策（本批新增，均为上一批框架的直接推论）：

- **合并 = 依赖 ∪ 层栈的并集**，排序与打标走纯函数 `lib/pluginCatalog.ts::mergePluginRows`
  （可单测；组件不自行重排）。既是层又是依赖的包只出现一次，归层栈块定位。
- **实验性行不再持有开关/更新/卸载**。这是单一入口（ADR-0020 §2.8）在合表中的落点：
  卸载实验包必须连带清挂载行（否则悬空行让 dsh 启动失败），该编排只在「实验能力」面板。
  插件列表里这类行的唯一动作是**「去开关」跳转**（切到实验能力 tab 并选中该能力）。
  第三方行保留原有开关/更新/卸载；内置行无控制面（随 dsh 自带，开关在 dsh 插件页）。
- **目录数据改受控**：`caps` 由 `ProfileDetailPane` 取数后下发给能力面板（与插件列表
  共用一次回读）。禁双源的实在理由：归属标记只有目录知道，而目录按 profile——两处各取
  一次，切档/写操作后必然对不上。三道回读闸门随负载搬到父级 `loadCaps`
  （`experimentalCapabilitiesGate.test.ts` 的判据同步改指父级，先红后绿核验）。

### 6.1 同日修订：判据锚定 dsh 源码（2026-09-20 真机截图回退第一版）

第一版落地后维护者截图回退：`@openviking/dsh-memory-plugin` 出现 **5 行**（全标「层 4 ·
内置 · 未安装」）、`auto-review` 顶「层 3 · 内置」。根因是**没读 dsh 源码就定了判据**。
上游事实（`deepseek-harness`：`packages/boot/app-boot/src/profile.ts`、`profile-plugins.ts`、
`packages/boot/plugin-manager/src/index.ts::listBundles`）：

- `dsh.profile.bundles` = **激活的层列表**（组合序）。init 写模板层；之后由 reconcile
  维护，且**保留栈内重复**（源码注释原样："template entries retain their order and
  duplicates"）。第一版把每个栈元素渲染成一行 → 一个包最多 N+1 行。
- dsh 自己的插件页对每个包给三个事实：`installed`（在 dependencies）、`enabled`
  （在 bundles）、**`removable = installed && !installation.dependencies`**。
  「内置/不可卸载」的权威判据是**安装目录提供**，不是"在 bundles 里"——用户经 npm
  装进 profile 的层一样进 bundles，可更新可卸载。
- dsh 自己呈现层也用 `[...new Set(...)]` **按包名去重**。
- `OPTIONAL_BUNDLES` = 安装自带、默认关、在 dsh 插件页开关、永不可卸（Agent Teams
  两层）；auto-review 是"发布在 npm 上的实验包"，**不是** optional bundle
  （设计笔记 `2026-09-15-shipped-optional-bundles.md` 原话）。

修订后的判据（`lib/pluginCatalog.ts`，全部由现有 IPC 算出，**零契约改动**）：

- **一个包一行**：栈先去重保序（重复条目折叠到首个位置），层序 = 去重后名次；
  dependency 条目优先作展示位（版本/简介是 node_modules 实读）——修掉「未安装」错显
  （安装提供的层改显字典里既有的「版本随 dsh」）。
- **`dshProvided`（内置、不可卸载）= 有 bundle 条目 ∧ 无 dependency 条目**。依据是
  模块解析序（bundle 名先从安装目录解析，再到 profile node_modules）：在层栈里却不是
  本 profile 依赖的包只能由安装提供——dsh `removable` 规则的反面。覆盖模板层 /
  OPTIONAL_BUNDLES / 桌面包（Rust 已按 desktop-packages spec 判为 bundle 类）。
- `installed` = 有 dependency 条目（用户装的，含**装进 profile 的层**——带「层 N」标、
  控制面同普通第三方）。
- 裁定 2 随之精确化：Agent Teams 两层激活时 = 实验性 + 追加「内置」（`dshProvided`）；
  用户经面板装的 auto-review = 纯实验性，**不带**内置标。

已知边界（如实记账，另案）：若有人绕过 reconcile 手工编辑 package.json，栈里可能留下
"既非安装提供、也未安装"的失效条目，本判据会把它显示为「内置」（保守方向：不给卸载入口，
比让用户卸一个组合树正在用的层安全）。精确判据需后端在 `PluginEntry` 补 provided 旗标
（契约改动）。

已知代价（如实记账）：

- 每次选中 profile 多一次 `list_experimental_capabilities`（内部一次 `dsh --dump-config`，
  与行表各一次）。换来的是不再有第二条目录链、开能力 tab 零等待。若后续测得切档卡顿，
  收敛方向是后端把"包 → 能力"映射并入 `list_profile_plugins`（一次 dump-config 两用），
  那是契约改动，另案处理。
- 「内置」与「实验性」两个筛选计数**可重叠**（dsh 自带实验层同时计入两者），三者之和
  可大于总数——facet 不是分区，这是裁定 2 的直接后果。


### 6.2 第三轮修订：安装自带的能力（OPTIONAL_BUNDLES）不进能力面板（2026-09-20 维护者裁定）

触发点：维护者在 dev 档发现「Agent Teams 在插件列表里看不到」。排查确认的事实链：
dev 引擎已升级到 0.1.6-alpha.2，`engines/dsh-runtime/node_modules` 里**有**那两个
optional bundle ⇒ 能力面板按 `shippedByDsh` 把它显示成「dsh 已内置」（无开关、只有
说明页），但该 profile 的 `dsh.profile.bundles` 里没有它们 ⇒ 插件列表无行可列。
维护者裁定：**「如果在 OPTIONAL_BUNDLES，那就不需要在实验功能里面显示了，因为已经被
官方的插件管理托管了。我们只需要在启动这个插件后，在插件列表能正常展示就可以了。」**

落地（规则单一，父级过滤一次，面板与列表共用）：

- `lib/pluginCatalog.ts::dockCuratedCaps` = 目录排除 `shippedByDsh` 的能力。
  父级（`ProfileDetailPane`）过滤后同时喂能力面板（展示）与插件列表（打标）。
- 能力面板：shipped 能力**不再出现**；`ShippedPane`、自带徽标、开关豁免、
  「清理旧副本」整条链移除（2026-09-17 立的"展示但只说明"判据随裁定翻面，
  闸门从"必须有 ShippedPane"反转为"必须过滤掉"）。字典 6 个 capShipped/capLegacy
  键删除；`shippedByDsh` / `legacyCopy` 契约字段后端仍下发（不改契约），本壳不再消费。
- 插件列表：自带能力的层启用后（在 dsh 自己的插件页开关 → 写进本档
  `dsh.profile.bundles`）与模板层同列——「内置」+「层 N」+「版本随 dsh」，无控制面。
  与 base/web-app 的展示完全一致（区别只在默认选没选中）。
- 遗留副本的出路变化：dock 早期装进 profile 的那份不再有专门的清理入口，它作为普通
  第三方行展示，用户可在列表里直接卸载（包是 bundle 类、无壳写行，卸载即清）。

先红后绿核验：`dockCuratedCaps` 改回 no-op → 6 处断言复红（面板边界闸门 + 5 处行为），
复原全绿。

### 6.3 第四轮修订：能力的「基座 / 标识包」角色分流（2026-09-20 真机截图）

维护者截图指出：`@deepseek-ai/dsh-browser-use`（基座）与
`@deepseek-ai/dsh-experimental-browser-use-playwright-mcp`（provider）两行**并列**挂着同一个
「实验性 · 浏览器操作」——读起来像同一能力出现两次。源码确认（`deepseek-harness`）：

- `@deepseek-ai/dsh-browser-use` 官方描述 **"Exclusive named browser-use provider
  registration"**，README 原话 **"This package adds no model-visible tools or browser
  operations"**——它是能力的**登记插槽**（同一时刻只允许一个 provider 注册，装第二个直接
  抛错），出现在全部 3 个变体的包清单里（dock 目录同形状：每个变体 = [基座, provider]）。
- provider 之间**互斥**（"Loading another provider fails with the registered provider
  name"）；`computer-use` 同形状（基座 + cua-driver-native）。

即：**一个能力 = 基座 + 一个生效后端**，不是两个并列插件。修订（`lib/pluginCatalog.ts`）：

- 新增 `capabilityRole`：包出现在几个变体的步骤里——1 = `primary`（变体独有的标识包，
  这一档真正干活的那个），≥2 = `shared`（多变体共有的基座）。判据与
  `experimentalCapabilities.ts::variantPackageRoles` 的 primary/shared 同源同义。
- 渲染分流：`primary` 行挂完整「实验性 · <能力>」+「去开关」；`shared` 行挂**弱化**的
  「<能力> · 基座」（字典 `tagCapabilityBase`，tooltip 明说"自身不提供工具"）。
- 「去开关」入口唯一：标识包在场时只在标识包行；该能力只装了基座（装一半）时基座行
  才承担——同一能力不给两个跳转按钮。

先红后绿核验：角色判据改回"全是 primary"→ 3 处断言复红（2 行为 + 1 闸门），复原全绿。

### 6.4 第五轮：控制面 / 后果面拆分（2026-09-20 维护者要求"布局和交互着着重构"）

先认账的四个结构性问题（v3 几轮迭代攒出来的）：① 左右栏大量重复（状态徽标、包名、
前置、替换说明在两处各说一遍）；② 详情面十节平铺，最核心的"选后端"埋在第 5 节；
③ 开关作用于"选中的后端"而非"生效的后端"——有 misaligned 中间态，行里得挂小图标
补救，两处开关还各说各话；④ 列表行扛 4 种信号，"扫一眼"变"读一遍"。

**方案经 ui-ux-pro-max 规则核验后修订**（`compact-label-overflow` /
`truncation-strategy` / `web-target-size` / `overflow-menu` / `destructive-nav-separation` /
`progressive-disclosure` / `number-tabular` 等）：初稿"左栏行内后端 pills"被三条规则
叠加枪毙——50+ 字符包名在 21–25rem 栏宽里 nowrap 放不下、截断会砍掉区分各档的尾部
（2026-09-17 裁定不许截断）、折行就不是 pill，且可点目标需 ≥24×24 CSSpx（WCAG 2.2 AA）。
出路按规则的 `overflow-menu` 落定。

落地（`ExperimentalCapabilities.tsx` v4）：

- **左栏 = 唯一控制面**：一能力一行（图标 · 名称 · 状态 · **生效后端的包名** · 开关），
  多变体时行尾一个**溢出菜单**「切换后端」（Popover 基座，包名在菜单里完整折行；
  当前生效档禁用 + 徽标说明）。行内显示的自古以来就是**事实**。
- **picked 中间态取消**：`rowTarget` = 当前生效档 → 首个可用档（目录序即推荐序）。
  菜单点选即发起（与开关同一条 `activate` 链，该确认的确认）；misaligned 态死亡。
- **右栏 = 后果面**：身份 → 进度/失败 → **后端对照（只读）** → 会发生什么 + 共同前置
  → 排障细节（默认折叠）→ 移除（危险区，沉底）。右栏不再有开关/单选组/onPick，
  恢复与危险区入口保留且受 busy 互斥。
- 汇总计数改等宽数字（`number-tabular`）；live 播报不新造——父级 toast 已是
  `role="status" aria-live="polite"`，再造一个就是 competing live regions。

闸门：`experimentalCapabilitiesGate.test.ts` ⑧/③/⑤/⑥/⑦ 的判据随重构迁移
（`listRegion` 切片终点改为 `BackendMenu`——菜单是披露面不算行内联；右栏判据
`variant.` → `target.`；动作分支入参仍叫 `variant`），新增 ⑫（控制面/后果面五条：
无 picked、菜单即披露、点选与开关同链、右栏只读、等宽数字）。先红后绿核验：
复活 picked state / 放开生效档可点 → ⑫ 两条逐一复红，复原全绿。

### 6.5 第六轮：「层 N」号标撤除 + 基座/provider 组内层级（2026-09-20 维护者两条裁定）

维护者两条连裁：①「我不希望展示层一 层二 这种类型的标签」；②「基座和 provider 的
从属关系不需要从交互上体现一下吗？」（真机截图：两行平铺，从属关系只靠 tag 文字区分）。

落地（`lib/pluginCatalog.ts` + `ProfileDetailPane.tsx`）：

- **「层 N」标撤除**：`tagLayer` / `tagLayerHint` 字典键删除，行内不再渲染层号；组合序
  **保留在排列里**（层栈成员在前、按去重后栈序——裁定 1 只撤号，不撤序）。闸门翻面：
  从"必须渲染层 N"反转为"行内与字典都不得出现 tagLayer"。
- **从属分组**：同一能力的包在列表里**相邻成组**——**基座在上（anchor，能力本体）**，
  provider 缩进其下（`capabilityChild`，挂弱化「后端」标 + 左侧从属线）；anchor 挂完整
  「实验性 · <能力>」+「去开关」（同一能力一个入口），anchor 为基座时再补「基座」标。
  输入序不翻转从属关系（依赖字典序里 provider 可能排在基座前，组内仍基座居首）。
- **组内 hover 联动**：鼠标进组即高亮整组（基座 + provider 一起亮）——从属关系从交互上
  可感知，不再是两行平铺各说各话。

先红后绿核验：组不拉相邻 / anchor 不优先基座 → 行为用例与闸门复红（含"输入序翻转"
专项用例）；复原全绿。
