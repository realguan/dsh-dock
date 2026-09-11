# BootSelector 文案收口（task-25）

> 日期：2026-09-11 ｜ 执行：`frontend` ｜ 任务：task-25（fix）
> 证据强度：**实测**（逐行扫描 + 可达性判据 + 门禁红→绿 + en 漏译基线回归 + sha256 校验复原）
> 基线：39 files / 289 passed → 交付后 **40 files / 298 passed**
> 范围：`pages/BootSelector.tsx` + `content/{zh-CN,en-US}.ts` + `__tests__/`（未动 Rust / 依赖）

## 1. 系统性排查结果（4 处，非任务列的 2 处）

扫描口径：剥离注释后**逐行保留原始行号**扫 CJK，覆盖 JSX 文本节点 / 字符串字面量 /
模板串 / `??` 回退字面量；属性面另单独核对（`title` / `placeholder` / `aria-label` / `alt`）。

| # | 行（改前） | 代码 | 可达性判据 | 分类 | 处理 |
| :-- | :--- | :--- | :--- | :--- | :--- |
| 1 | `:330` | `p.name === "web" ? "官方开箱即用" : t.selector.customDesc` | **可达**：条件为 `pluginCount === 0` 且 `p.name === "web"`（`pluginCount = summary?.dependencies.length ?? 0`）⇒ 未物化模板 web，或已物化但零第三方依赖。**en 用户可见中文** | **真 i18n 缺陷** | → `t.selector.officialReadyToUse` |
| 2 | `:159` | `` detail: `${msg}（可返回重选）` `` | **可达**：`handleLaunch` 的 `catch`（`api.chooseProfile` 抛错）→ `setLocalError({detail})` → `ErrorCard` 渲染 `payload.detail`（`ErrorCard.tsx:108/110`）。**en 用户可见中文**（任务未列，本次扫出） | **真 i18n 缺陷** | → `t.error.reselectHint`（后缀键，见 §3） |
| 3 | `:123` | `title: name === "web" ? "默认工作台" : name` | **不可达 / 潜在陷阱**：`items.web` 恒存在 ⇒ `t.selector.items[name] ?? …` 的 `??` 对 `"web"` 不触发；对非 web 名三元取 `name` 分支（无中文）⇒ **两分支都不吐中文**。但删掉 `items.web` 即向 en 吐中文 | **潜在陷阱** | 见 §4（消除而非翻译） |
| 4 | `:151`（改后 `:160`） | `` logger.warn("[selector]", "保存默认工作台偏好失败", …) `` | 可达，但**面向开发者、非 UI 文案** | **不应入字典** | **不改**（理由见 §5） |

**不应入字典的其它项**（扫过、确认无需处理）：`"web"`（profile 标识符）、`"DSH Dock"`（品牌名）、
`{count}`（占位符）、数字键位提示 `1-9`（在字典 `quickKeysHint` 内）、`"CUSTOM"`/`locale`（数据值）。
属性面：本文件 `title={p.desc}` 取自字典，无硬编码 `placeholder`/`aria-label`/`alt`。

## 2. 新增键对照（2 个，zh/en 对称）

| 键 | zh-CN | en-US |
| :--- | :--- | :--- |
| `selector.officialReadyToUse` | 官方开箱即用 | Official · ready to use |
| `error.reselectHint` | （可返回重选） | ␣(you can go back and reselect) |

**`reselectHint` 的括号体系**：拼接形式为 `` `${msg}${hint}` ``，故 zh 用**全角括号**（无需空格）、
en 用**半角括号 + 前置空格**（否则会粘成 `error(you can…`）。门禁显式断言这一差异。

## 3. 「官方开箱即用」与「（可返回重选）」的可达性复核（任务要求引代码条件）

- **#1**：`:339`（改后）所在三元链 = `pluginCount > 0 ? pluginsCount : p.name === "web" ? officialReadyToUse : customDesc`。
  `pluginCount` 来自 `summary?.dependencies.length ?? 0`——未物化 profile 无 summary ⇒ 0；
  物化但未装第三方依赖 ⇒ 0。故「web 且零依赖」是**正常运行路径**，不是边缘情形。
- **#2**：`handleLaunch` 在 `await api.chooseProfile(name)` 抛错时写 `detail`；
  `ErrorCard`（经 `BootStep`/错误卡渲染链）会把 `payload.detail` 直接显示（`ErrorCard.tsx:108-110`）。
  启动失败的场景（本仓库最常见用户可见错误之一）必然命中 ⇒ 可达。

## 4. `:123` 的处理：**消除**而非翻译（请 lead 复核此判断）

改为 `title: name`（未知名回退直接给 profile 名，语言中立），并删除 `name === "web"` 三元。理由：

1. **等价性可证**：对任何**真正走到 `??` 回退**的 `name`，必有 `name !== "web"`（`items.web` 恒存在），
   故旧代码取值恒为 `name`，与新写法**逐字一致**——不改变任何可达路径的行为。
2. **除隐患更彻底**：把该字面量搬进字典，只是让「假想未来」里显示英文，**死分支依然留着**，
   而下一次有人改字典仍要重新判断它的用途；删除后**没有任何中文可被吐给 en 用户**。
3. **与 task-23 的处置一致**：lead 已批准「死字段留着等于给将来预留再耦合入口，删之」；
   此处是同一判据（不可达分支 + 中文回退）。且 `title: meta.title || name`（下一行）本就确立了
   「无标题即回退 profile 名」的写法，删除后更一致。
4. **代价**：若将来真删掉 `items.web`，卡片标题会显示 `web`（与右侧 name 芯片重复）而非「默认工作台」。
   这是**诚实且可本地化**的降级；如 lead 认为应保留本地化标题，补一个字典键即可（一行），
   门禁中「非注释行不得出现裸中文」仍会通过。

> 若你判应做成字典键（而非删除），我改回 `title: name === "web" ? t.selector.webFallbackTitle : name`
> 并补键即可 —— **请指示**。当前实现已按「删除」交付。

## 5. logger 中文消息为何不入字典（白名单化的理由）

`logger.warn("[selector]", "保存默认工作台偏好失败", …)` 面向**维护者**，不进 UI、不参与本地化：

- AGENTS §4.3 对日志的要求是**格式**（`[模块] 描述 {ctx}`）+ 禁密钥/PII，**未要求**文案入字典；
- 全仓实测 **7 处**同型中文 logger 消息（`App.tsx:78/96`、`clientUpdateStore.ts:59`、
  `ErrorCard.tsx:75`、`BootStep.tsx:37`、`QuickDshSwitcher.tsx:65` 等）——是既有统一惯例，
  单独把这一处改英文会造成口径分裂；
- 故**白名单化并附理由**，门禁断言白名单**非空且有命中**，将来若日志整体改英文会提示撤销白名单，
  而任何**新增 UI 中文**仍会被抓住。

## 6. 防复发测试（`__tests__/bootSelectorCopy.test.ts`，9 例，纯逻辑）

| # | 断言 |
| :-- | :--- |
| 1 | 该文件非注释行**不得出现裸中文**（logger 行为显式例外，按「行内有 logger 调用」判定而非钉死句子） |
| 2 | 白名单**非空且有命中**（防腐化：日志改英文后该例外应撤销，不能默默放宽）；并反向断言剥离注释后仍含真实结构（防过度剥离假绿） |
| 3 | 两个新键两侧对称、类型一致、非空 |
| 4 | 新键 **en 侧不含 CJK**（组件内缺陷的字典侧防线） |
| 5 | `reselectHint` 括号体系：zh 以 `（` 开头且无前置空格；en 以 ` ( ` 开头 |
| 6 | 组件确实消费两个新键 |
| 7 | 旧内联文案不得回流（`官方开箱即用` / `（可返回重选）` / `默认工作台`）——**在 UI 面上**断言 |
| 8 | **回归**：task-23 的判定解耦未被改动（`resolveIsDefault(name, defaultProfile)` 在、`meta.tag ===` 无、`FACTORY_DEFAULT_PROFILE` 在、徽章仍走 `{t.selector.defaultBadge}`） |
| 9 | **回归**：语义等价性（`items.web` 存在 + `title: name,`） |

> **注释剥离直接做对**（task 明确要求）：`stripComments` 剥离 `{/* */}` / `/* */` / `//`，
> 并额外把 **logger 行**从「UI 面」剔除——两处断言都基于 `uiCode`，避免「日志里的『默认工作台』
> 被当成回流文案」这类假阳性（本次实现时确实先踩到、随后修正）。

### 6.1 复现先行：门禁在改前**实测红**（6/9）

`BootSelector.tsx` 在本次改动前无他人未提交改动，可安全 `git show HEAD:` 还原该单文件：

```text
× 非注释行不得出现裸中文（logger 消息为显式例外）   组件内仍有裸中文 UI 文案：expected [ …(3) ] to deeply equal []
× 例外白名单非空且确有命中                          expected … to contain 'resolveIsDefault(name, defaultProfile)'
× 组件确实消费新键                                  expected … to contain 't.selector.officialReadyToUse'
× 旧内联文案不得回流                                回流了旧内联文案 官方开箱即用
× 回归：task-23 判定解耦未被改动                     expected … to contain 'const isDefault = resolveIsDefault(na…'
× 回归：语义等价性                                   expected … to match /title:\s*name,/
── Test Files 1 failed | Tests 6 failed | 3 passed (9)
current-sha256:  cd232cde0ced665f61434d47e2b3e8973c72a09cd379a8ba588b82d56e97d75e
restored-sha256: cd232cde0ced665f61434d47e2b3e8973c72a09cd379a8ba588b82d56e97d75e
RESTORE_OK (sha256 identical)
```

第 1 条直接量化了缺陷：改前 UI 面有 **3 处**裸中文（正是 #1/#2/#3）。
（第 5/6 条对 HEAD 也红，因 task-23 的解耦与去死字段尚未提交——两轮增量同在工作树。）

## 7. 与 task-23 / task-24 的边界确认

| 命题 | 核实 |
| :--- | :--- |
| 未动 task-23 的判定解耦 | 门禁第 8/9 条回归断言；`isDefault` 仍走 `resolveIsDefault`，未新增任何字典比较 |
| 未动 task-24 的语言初始化路径 | 本任务未触及 `App.tsx` / `i18nStore.ts` / `ProfileManager.tsx`；`BootSelector.tsx` 亦无初始化逻辑 |
| 只动文案 | 唯一非文案改动 = 删除不可达三元分支（§4 已给等价性证明并请裁定） |

## 8. 验证与取证（实测）

```text
$ cd frontend && pnpm run typecheck && pnpm run lint && pnpm run test
$ tsc -b --pretty false
Found 0 warnings and 0 errors.
Finished in 36ms on 138 files with 96 rules using 8 threads.
 Test Files  40 passed (40)
      Tests  298 passed (298)
```

**en 漏译基线回归**（task-18 门禁口径，本任务改了 en 字典）：叶子 651 → **652**（+1，新增键），
CJK 命中仍恒为 **1**（唯一白名单例外 `console.localeZh` 语言自名）⇒ 未引入新漏译。

新增键实际取值（实测）：
`zh: selector.officialReadyToUse="官方开箱即用" / error.reselectHint="（可返回重选）"`；
`en: "Official · ready to use" / " (you can go back and reselect)"`。

## 9. 遗留（**只报告，未动**）

1. **同一字面量的第三处在别的文件（真 i18n 缺陷，超本任务范围）**：
   `components/boot/ErrorCard.tsx:66` `` setActionError(`${t.error.actionFailed}：${msg}（可返回重选）`) ``
   —— **可达**（动作执行失败路径），en 用户见中文。本任务已备好 `t.error.reselectHint`，
   该处改造成本 = 一处字符串拼接，建议单独立项（一行改动 + 复用现有键）。
2. **task-23 之后的残留死代码**（本任务未动，避免改动刚验收的快照）：
   `pages/BootSelector.tsx:134` `tag: t.selector.customTag`（回退 meta 的字段）与字典里的
   `selector.items[*].tag` / `selector.customTag`，在 task-23 删掉 `displayProfiles[].tag` 后
   **已无任何消费者**（实测 grep 仅命中该构造点本身）。删它需要同步改
   `bootSelectorDefault.test.ts` 中钉住 `tag: t.selector.customTag` 的断言，
   **属于修改 task-23 的验收快照**，故留待下一轮专项（或与 ErrorCard 一并做）。
   —— 其风险等级低（不再参与判定），但作为「字典里有看似控制令牌的 `"DEFAULT"`」仍是认知噪声。
3. **其它两处「读取侧兜底 web」措辞待校正（不在本任务写入范围，仅报告）**：
   `types/ipc.ts:205`（`DeleteOutcome.default_cleared` 注释「引用已清除（读取侧兜底 web）」）与
   `stores/profilesStore.ts:12`（`defaultProfile` 注释「null = 未设置（读取侧兜底 web）」）。
   它们描述的是 **None** 场景，与新 §6 措辞（「None 由消费方兜底 `web`」）**不冲突**；
   但 `default_cleared` 那条容易被读成「清除后由 web 兜底」——而按新口径，**失效值不消费**。
   两个文件均不在本任务硬边界内，**未动**，建议随下一轮类型注释收口一并厘清。

## 10. AGENTS §6 措辞校正（2026-09-11 会话中到达）与本任务的对应

任务进行期间 `AGENTS.md` §6 更正了 `defaultProfile` 的表述（双源校正，非口径变更）：

| 版本 | 表述 |
| :--- | :--- |
| 旧（我 task-23 报告曾引用） | 「None/失效值读取侧兜底 `web`」 |
| **新（现行）** | 「None 由消费方兜底 `web`，**失效值不消费**（走常规流程＝出选择器）；删除/重命名同步引用」 |

**结论：实现与新口径完全一致**（`name === (defaultProfile ?? FACTORY_DEFAULT_PROFILE)`）：

- `null` → 命中 `web`（兜底；web 确实会被启动）；
- 失效值 → **不命中任何候选**（不消费；没有"将被启动的默认"）。

即 **task-23 的行为差异 ② 得到了宪法层面的确认**（lead 当时已用 `resolve.rs:659`
`consume_default_profile` 的 `filter` 批准过，现 §6 亦与之一致）。

**随之修正的两处陈旧引用**（均在 task-25 写入范围内，已改）：

- `pages/BootSelector.tsx:31`（`FACTORY_DEFAULT_PROFILE` 文档注释）——改为引新措辞，
  并补一段「两条口径 ↔ 判定结果」的对应关系；
- `__tests__/bootSelectorDefault.test.ts:47`——注释改引新措辞，并**新增一条断言**
  「失效默认值 ⇒ 不消费，无任何卡片标为默认」，把新口径的**后半句**也钉进测试
  （原测试只覆盖了 None 兜底这一半）。

**未修正的一处陈旧引用**：`docs/team/控制流令牌耦合修复-2026-09-11.md:42`
（task-23 报告 §2）仍写「AGENTS §6 已登记的读取侧兜底口径（None / 失效 → `web`）」。
该文件的写入范围属 **task-23**（已完成、且已进 qa-verify 复验快照），**本任务不越界修改**，
故只在此登记，请 lead 决定是追加更正条目还是就地修订。
