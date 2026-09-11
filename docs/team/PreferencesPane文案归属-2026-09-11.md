# PreferencesPane 文案归属修复（task-20）

> 日期：2026-09-11 ｜ 执行：`frontend` ｜ 任务：task-20（fix）
> 证据强度：**实测**（逐处扫描全清单 + 门禁红→绿 + 两轮注入探针复现 + sha256 校验复原）
> 范围：`components/system/PreferencesPane.tsx` + `content/{zh-CN,en-US}.ts` + `__tests__/`
> 基线：36 files / 240 passed → 交付后 **37 files / 250 passed**

## 1. 结论摘要

- 系统性排查该文件，用户可见硬编码文案共 **11 处**（不止任务书列的 3 处）：
  **真 i18n 缺陷 10 处**（其中 3 处为**反向漏译**：zh 用户看到英文）·
  **冗余重复渲染 1 处**（英文卡副标题 = 标题逐字复述）·
  **不应入字典 4 处**（键盘快捷键符号，保留在组件内）。
- 语言选择器视觉重复问题**已核实并消除**：中文卡最终渲染中「简体中文」**恰好出现一次**
  （标题「简体中文」+ 副标题「默认」）；英文卡同类重复（标题/副标题同为 `English (US)`）
  一并**删除副标题行**。
- 新增 **10 个键**，zh/en 对称。

## 2. 逐处分类与改动（11 处文案）

| # | 行（改后） | 原文案 | 分类 | 处理 |
| :-- | :--- | :--- | :--- | :--- |
| 1 | `:107` | `正在加载偏好设置…` | **真 i18n 缺陷**（en 见中文） | `t.console.settingsLoading` |
| 2 | `:172` | `Auto Detect` | **真 i18n 缺陷·反向**（zh 见英文） | `t.console.localeSystemHint`＝「自动检测」/"Auto Detect" |
| 3 | `:191` | `简体中文 (默认)` | **真 i18n 缺陷**（en 见中文）＋**复述标题** | `t.console.localeZhHint`＝「默认」/"Default" |
| 4 | `:210` | `English (US)`（副标题） | **冗余重复渲染**（与其标题逐字同串，两语皆然） | **删除该行**（不新增键，见 §4） |
| 5 | `:276` | `智能熔断保护协议（Circuit Breaker）` | **真 i18n 缺陷** | `t.console.breakerTitle` |
| 6 | `:280` | `监控窗口：` | **真 i18n 缺陷** | `t.console.breakerWindowLabel` |
| 7 | `:281` | `60 秒滑动窗口` | **真 i18n 缺陷** | `t.console.breakerWindowValue` |
| 8 | `:284` | `熔断阈值：` | **真 i18n 缺陷** | `t.console.breakerThresholdLabel` |
| 9 | `:285` | `连续 3 次崩溃` | **真 i18n 缺陷** | `t.console.breakerThresholdValue` |
| 10 | `:288` | `熔断后动作：` | **真 i18n 缺陷** | `t.console.breakerActionLabel` |
| 11 | `:289` | `停机并弹诊断卡` | **真 i18n 缺陷** | `t.console.breakerActionValue` |

**不应入字典（未改，4 处）**：`:374` `⌘ + ,` / `Ctrl + ,`、`:395` `⌘ + ⇧ + P` / `Ctrl + ⇧ + P`
——`⌘`/`⇧` 是跨语言固定符号，`Ctrl` 是键名，二者由 `isMac` 分支选取；字典内已有
`shortcutDefault` / `shortcutShiftP` 承载**标签**文案（且已含同一组符号），按键序列本身无翻译价值。

> 标签行内的冒号随语言不同（zh「监控窗口：」全角 vs en "Window:" 半角），故标点留在字典值内，
> 渲染结构与改动前逐字一致。

## 3. 新增键对照表（`console` 命名空间）

| 键 | zh-CN | en-US |
| :--- | :--- | :--- |
| `settingsLoading` | 正在加载偏好设置… | Loading preferences… |
| `localeSystemHint` | 自动检测 | Auto Detect |
| `localeZhHint` | 默认 | Default |
| `breakerTitle` | 智能熔断保护协议（Circuit Breaker） | Smart Circuit Breaker Protocol |
| `breakerWindowLabel` | 监控窗口： | Window: |
| `breakerWindowValue` | 60 秒滑动窗口 | 60s sliding window |
| `breakerThresholdLabel` | 熔断阈值： | Trip threshold: |
| `breakerThresholdValue` | 连续 3 次崩溃 | 3 crashes in a row |
| `breakerActionLabel` | 熔断后动作： | On trip: |
| `breakerActionValue` | 停机并弹诊断卡 | Stop and show diagnostics |

- 措辞体系（对齐任务口径「语言自名 + 系统项按当前 UI 语言」）：语言项标题沿用自名
  （`localeZh`「简体中文」/ `localeEn` "English (US)"，task-18 已定），系统项及其副标题
  按当前 UI 语言渲染，故 zh 侧 `localeSystemHint`＝「自动检测」而非 "Auto Detect"。

## 4. 视觉重复问题的核实结论（task 要点 3）

**已核实：修复前确有重复，且不止一处。**

| 卡片 | 修复前渲染 | 问题 |
| :--- | :--- | :--- |
| 跟随系统 | `跟随系统语言` / `Auto Detect` | 无重复，但副标题在 zh 语境是英文（反向漏译） |
| 简体中文 | `简体中文` / `简体中文 (默认)` | **「简体中文」出现两遍** |
| English | `English (US)` / `English (US)` | **标题与副标题逐字同串**（两语皆然） |

**修复后渲染**（实测字典值拼接）：

| 卡片 | zh-CN | en-US |
| :--- | :--- | :--- |
| 跟随系统 | `跟随系统语言` / `自动检测` | `System Default` / `Auto Detect` |
| 简体中文 | `简体中文` / `默认` | `简体中文` / `Default` |
| English | `English (US)`（无副标题行） | `English (US)`（无副标题行） |

**英文卡「删行」而非「补键」的理由（与任务书建议的「zh 语境应为『英语（美国）』」不同，请 lead 复核）**：
task-18 已确立并复核「语言自名（endonym）」惯例——语言选项按其**自身语言**书写，
en 侧 `localeZh` 也因此保留「简体中文」。若给英文卡补一个中文译名副标题，等于**重新引入
task-18 刚删掉的那类译名括注**，两处口径相互矛盾；而该副标题的内容原本只是标题的逐字复述，
删除**不丢失任何信息**。故按「删重复行」处理。如 lead 判定应改为「英语（美国）」，
改动点仅一处（新增 `localeEnHint` 并恢复 `<p>`），门禁断言 `localeEnHint` 不存在需同步放开。

## 5. 新增门禁（`__tests__/preferencesPaneI18n.test.ts`，10 例，纯逻辑无 DOM）

1. 10 个新键两侧对称、类型一致、非空；
2. 新键 en 侧不含 CJK；
3. 新键 zh 侧不得残留英文整词（`breakerTitle` 的术语括注「（Circuit Breaker）」白名单化，
   与 lead 已裁定的 zh `profiles.mcp*` 字段名括注同类）；
4. **语言卡片不得复述自己的标题**（行级去重）；
5. **副标题不得等于任何卡片标题**，且英文卡不设副标题键（`localeEnHint` 必须不存在）；
6. **中文卡最终渲染中「简体中文」恰好出现一次**（子串级计数）；
7. 三张卡片标题互不相同；
8. 组件确实消费 10 个新键（`?raw` 源码断言）；
9. 旧硬编码 11 处特征串不得回流（**剥离注释后**断言——源码注释里会用旧文案作说明，
   把注释当渲染文案判红是假阳性；已加「剥离后仍含真实键」反向断言防过度剥离）；
10. 键盘快捷键符号保留在组件内（记录「不应入字典」的裁定）。

## 6. 验证与取证（实测）

### 6.1 闸门（CI 同款口径）

```text
$ cd frontend && pnpm run typecheck && pnpm run lint && pnpm run test
$ tsc -b --pretty false
Found 0 warnings and 0 errors.
Finished in 32ms on 135 files with 96 rules using 8 threads.
 Test Files  37 passed (37)
      Tests  250 passed (250)
```

### 6.2 复现先行 A：门禁对改前组件**实测红**

组件在本任务前**无他人未提交改动**（`git diff` 全为本次改动），故可安全用 `git show HEAD:` 还原该单文件。

```text
× 组件确实消费新键（?raw 源码断言）
    AssertionError: 组件未消费 t.console.settingsLoading
× 旧硬编码文案不得回流（仅本文件的 11 处特征串）
    AssertionError: 回流了旧硬编码文案 正在加载偏好设置…
current-sha256:  59019e594f632920d13a47930d95406d65ae95fee8b3551990d0ad4ef17e36d6
restored-sha256: 59019e594f632920d13a47930d95406d65ae95fee8b3551990d0ad4ef17e36d6
RESTORE_OK (sha256 identical)
```

### 6.3 复现先行 B：注入「天真迁移 + 反向漏译」探针，重复检测器**实测红**

本门禁的核心是把「入字典时顺手把副标题原样搬进来」这种**半成品**卡住。注入
`localeZhHint: "简体中文 (默认)"`（天真迁移）与 `localeSystemHint: "Auto Detect"`
（zh 侧英文）后：

```text
× 新增键的 zh 侧不得残留英文整句（技术术语括注除外）
    AssertionError: zh-CN 侧出现英文整词 ⇒ 反向漏译: expected [ 'localeSystemHint' ] to deeply equal []
× 中文卡最终渲染中「简体中文」恰好出现一次
    AssertionError: 渲染串：简体中文 简体中文 (默认): expected 2 to be 1
restored-sha256: f63008891e717a08cdc7b4ec0f2f06dde4b3137c7ca57f734fa8f2735d34f190  (= current)
RESTORE_OK (sha256 identical)
```

> 注：行级去重断言（第 4 项）对 `"简体中文 (默认)"` 这种**近似**重复不敏感（它不等于标题），
> 真正兜住它的是子串计数断言（第 6 项）——两者互补，缺一会漏。

### 6.4 回归：en-US 漏译基线未退化

```text
en-US leaves: 651 | CJK: 1 -> console.localeZh=简体中文 (Chinese Simplified)
```

仍恒为 task-18 白名单化的唯一例外（语言自名），本轮新增的 10 个键未引入新漏译
（叶子数 641 → 651，正好 +10）。

## 7. 范围与遗留（**未动**，供下一轮排期）

- 严格限于本组件 + 两字典 + `__tests__/`；未触碰其它组件，未改 Rust / IPC / 依赖，无 git 写操作。
- **只读盘点**（启发式扫描，非逐处确认）：`frontend/src/**/*.tsx`（除 `content/`、`__tests__/`）
  仍约有 **124 处**中文候选，集中在 19 个文件，Top 5：

  | 文件 | 候选处 |
  | :--- | :--- |
  | `components/profiles/PluginOverview.tsx` | 19 |
  | `components/profiles/SessionManager.tsx` | 17 |
  | `components/profiles/McpManager.tsx` | 16 |
  | `components/profiles/ProfileDetailPane.tsx` | 15 |
  | `components/system/DiagnosticsPane.tsx` | 13 |

  该数字是**候选上界**（含少量非用户可见串），需按组件逐个人工分类后才能定案——
  建议继续按「一组件一任务」的粒度立项（本轮已验证该粒度可行）。
- `marketI18n.test.ts`（task-16 交付）的「旧文案不回流」断言未剥离注释；
  若将来有人在该文件注释中引用被收口的旧字面量，会假阳性。本轮未改该文件
  （避免 churn 已验收产物），技术已在本文件的 `stripComments` 中实现，可按需回填。

---

# 2026-09-11 补充（task-22）：`localeZhHint` 事实失真

> 性质：fix（用户可见**事实失真**，与「2700+」「240+ 预置服务」同类）
> 范围：`PreferencesPane.tsx` + `content/{zh-CN,en-US}.ts` + `__tests__/`
> 证据强度：**实测**（读 store 初始态 + 读 Rust 默认 + 门禁红→绿 + 重引入探针）
> 基线：37 files / 250 passed → 交付后 **37 files / 252 passed**

## 补充 1. 结论

**中文卡不该有副标题——已删除 `localeZhHint` 键（两侧）与该行 `<p>`。**
三个卡片只在**系统卡**保留副标题（`localeSystemHint`，为真且有用）。

## 补充 2. 事实依据（实证，非推断）

| 事实 | 证据 |
| :--- | :--- |
| 前端默认偏好 = 跟随系统 | `i18nStore.ts:38` `preference: "system"`；本门禁直接读 store 初始态断言 `useI18nStore.getState().preference === "system"`（实测通过） |
| settings 无 `locale` 时保持 system | `initFromSettings()`（`i18nStore.ts:63-87`）仅在 `settings.locale` 存在时改写 preference；缺失走 `preference: "system"` + `resolveSystemLocale()` |
| 系统语言非 zh ⇒ en-US | `resolveSystemLocale()`：`navigator.language` 以 `zh` 开头才 zh-CN，**否则 en-US** |
| Rust 侧默认 None | `settings.rs:48` `locale: Option<String>`，注释「None = 跟随操作系统语言」；`:103` / `:153` 断言 `locale == None` |

⇒ 对 `navigator.language = en-US` 的用户，界面显示「简体中文 / **默认**」是在**谎报默认语言**。
⇒ **该主张无法靠改措辞救活**：删掉「默认」后剩下的只有「内置字典」这类**实现细节**，
对用户无价值、不该上界面（lead 口径，我认同）。故直接删键删行，而非换成别的词。

## 补充 3. 改动

| 文件 | 改动 |
| :--- | :--- |
| `content/zh-CN.ts` | 删 `localeZhHint: "默认"`；注释改为记录本次事实裁定（含依据指针） |
| `content/en-US.ts` | 删 `localeZhHint: "Default"`；同上（英文注释） |
| `PreferencesPane.tsx` | 删中文卡 `<p …>{t.console.localeZhHint}</p>`；注释同步 |
| `__tests__/preferencesPaneI18n.test.ts` | 见补充 4（**按新事实重写**，未为保绿而保留错误文案） |

**修复后语言卡片渲染**：

| 卡片 | zh-CN | en-US |
| :--- | :--- | :--- |
| 跟随系统 | `跟随系统语言` / `自动检测` | `System Default` / `Auto Detect` |
| 简体中文 | `简体中文`（无副标题） | `简体中文`（无副标题） |
| English | `English (US)`（无副标题） | `English (US)`（无副标题） |

## 补充 4. 门禁按新事实重写（12 例，未牺牲事实）

- `NEW_KEYS` 去掉 `localeZhHint`（10 → 9 键）；新增 `REMOVED_KEYS = ["localeZhHint", "localeEnHint"]`
  并断言**两侧都不存在**、且**组件不再引用**（防把副标题行挂在已删键上）。
- 删去 task-20 的两条重叠检测断言（`localeZhHint` 相关），替换为按新事实的断言：
  - 「中文卡只渲染标题一行」：`lines === ["简体中文"]`；
  - 「副标题不得等于任何卡片标题；只有系统卡有副标题键」；
  - **新增事实门禁**：「卡片副标题不得声称某个语言是『默认』」——先断言
    `useI18nStore.getState().preference === "system"`，再遍历两侧 `locale*Hint` 值，
    命中 `/默认|[Dd]efault/` 即红。
    > 该门禁初版写成「locale 组全部键不得含『默认』」时**误报**了
    > `localeSystem = "System Default"`——复查后确认那是**系统项的选项名**
    > （意为「用系统的默认」），与事实一致（产品默认确实跟随系统），故把扫描面收窄到
    > 主张性文案（`*Hint`）。**没有为了让门禁变绿而改文案，而是修正了门禁的判定面。**

## 补充 5. 复现先行：被删的假主张一旦回归，门禁**实测红**

注入 `localeZhHint: "默认"`（模拟「为让测试变绿而保留错误文案」）后：

```text
× 副标题不得等于任何卡片标题；只有系统卡有副标题键
× 事实门禁：卡片副标题不得声称某个语言是「默认」
current-sha256:  68402b4085c499f50fbd7c299be1944fe085c5da3d5b652716d409b7b22ae536
restored-sha256: 68402b4085c499f50fbd7c299be1944fe085c5da3d5b652716d409b7b22ae536
RESTORE_OK (sha256 identical)
```

## 补充 6. locale 组同类核查（任务要点 3）

| 键 | zh / en 现行值 | 判定 |
| :--- | :--- | :--- |
| `localeLabel` | 界面语言 / Interface Language | **属实**，不改 |
| `localeDesc` | 选择桌面壳界面的显示语言；更改后立即生效 / Choose display language for the desktop shell UI; updates immediately | **当前窗口内属实**（`setLocale` 同步 `set()` 立即重渲染）；**跨窗口不成立**——见补充 7。不靠改文案掩盖代码缺陷，故不改，另报 |
| `localeSystem` | 跟随系统语言 / System Default | **属实**：产品默认确实是跟随系统；en 值 `System Default` 与 zh 侧同义（选项名，非「本语言是默认」的主张） |
| `localeZh` | 简体中文 / 简体中文 (Chinese Simplified) | **属实**（语言自名 endonym，task-18 已裁定；en 侧括注是给英文读者的说明） |
| `localeEn` | English (US) / English (US) | **属实**（自名） |
| `localeSystemHint` | 自动检测 / Auto Detect | **属实且有用**：preference === "system" 时确实经 `resolveSystemLocale()` 读 `navigator.language`。**边界说明**：无 `languagechange` 监听（`grep` 全仓无命中）⇒ 运行期改 OS 语言不会实时跟随，需重启或重新选择该项；此边界不构成「自动检测」失真（该词描述的是选择/启动时的探测行为），**不改**，仅登记 |

**未改 item 的理由已逐条给出**；无一项是「改写措辞绕过错误」。

## 补充 7. 由此暴露的组件/Store 侧缺陷（**只报告，未动** —— 超出本任务硬边界）

**语言偏好只在控制中心窗口生效**（实测）：`initFromSettings()` 全仓**只有一处调用**——
`pages/ProfileManager.tsx:100`（`profiles` 窗口）。

| 窗口 | 路由 | 是否初始化语言 | 后果 |
| :--- | :--- | :--- | :--- |
| `profiles` | `ProfileManager`（`App.tsx:62`） | ✅ 调 `initFromSettings()` | 正常：按 settings.locale / 系统语言 |
| `main` | `BootIndex` / `BootMode` / `BootSelector`（`App.tsx:66-71`） | ❌ 从未调用 | 永远用 store 初始字典 `t: zhCN`、`activeLocale: "zh-CN"` ⇒ **选了 en-US 的用户，启动窗口仍是中文** |
| `about` | `About`（`App.tsx:59`） | ❌ 从未调用 | 同上 |

旁证：`app:settings-changed` 全仓仅被 `components/layout/QuickDshSwitcher.tsx:42` 消费，
且只用于 `switcherShortcut`，**没有任何窗口用它同步语言**；`setLocale` 的调用点也只有
`PreferencesPane`。

- 影响：跨窗口「改了语言但别的窗口不跟」+ 非 `profiles` 窗口对 en 用户恒显中文（**真 i18n 缺陷**）。
- 归属：修点在 `App.tsx` / `pages/*` / `stores/i18nStore.ts`，**均不在本任务写入范围**，
  按硬边界只报告。建议单独立项（含：主/about 窗口初始化语言 + 是否用 `app:settings-changed` 广播同步）。
- 附带口径：`localeDesc`「更改后立即生效」在**当前窗口**为真，修好上述管道后跨窗口也为真；
  **不建议**改文案，改文案只会把代码缺陷掩盖成「文案已准确」。
