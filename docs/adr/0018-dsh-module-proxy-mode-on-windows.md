# ADR-0018：Windows 上以「模块代理」模式启动 dsh（免符号链接特权）

- **日期**：2026-09-12
- **状态**：已接受
- **提出人**：guan（AI 协作，Lead 起草）
- **相关方**：`engines.rs`（shim 生成）· `resolve.rs`（引擎档启动形态）· `shell.rs`（spawn，不改）· ADR-0017（前置）
- **关联**：`docs/known-issues/v120-测试问题.md` 1.1 · ADR-0017 · Windows 真机报错（用户提供，见 §1）

---

## 1. 背景与问题

ADR-0017 已让 dsh **装上**（引擎引导不再因 global hash link 失败；用户真机截图的报错
路径已出现 `engines\dsh-runtime\node_modules\@deepseek-ai\dsh`，证明安装已成功）。
但 dsh **启动时**又失败，且是同一类根因的第二次显形：

```text
DSH 意外退出（代码 1）
EPERM: operation not permitted, symlink
  'C:\...\engines\dsh-runtime\node_modules\@deepseek-ai\dsh'
  -> 'C:\...\.dsh\profiles\node_modules\@deepseek-ai\dsh'
  errno: -4048, code: 'EPERM', syscall: 'symlink'
  at async runCli (.../@deepseek-ai/dsh/lib/bin.js:146:4)
```

**本机复现（macOS，同一 dsh 0.1.5-rc.2）**：以普通 node 方式真 boot 一次，dsh 在
`$DSH_HOME/profiles/node_modules` 下建了 **481 个符号链接**（指向安装树的 `.pnpm` 虚拟 store）。
Windows 普通账户无 `SeCreateSymbolicLinkPrivilege` ⇒ 第一个链接即 EPERM。

**这是 dsh 自身的实现选择，不是我们引入的**（旧路径只是从未走到这一步）：

| 事实 | 证据 |
|:---|:---|
| dsh 用 `isPackagedExecutable()` 选两种链接方式 | `@deepseek-ai/dsh-app-boot/lib/index.js` |
| 判据 = `process.pkg !== undefined` | 同文件 `function isPackagedExecutable()` |
| 普通 node ⇒ `ensureSymlink()`（`fs.symlinkSync`） | 同文件 `:612` `!isPackagedExecutable() ? kind:"symlink" : kind:"proxy"` |
| 打包可执行体 ⇒ `ensureModuleProxy()`（**纯目录 + `entry-N.js` 再导出，零软链**） | 同文件 `ensureModuleProxy()` |
| **该判据全文件仅 1 处调用** | `grep -c "isPackagedExecutable()"` = 2（1 定义 + 1 调用） |
| **无任何 env/config 开关** | 该文件无 `process.env` 读取；无相关配置项 |
| **上游无更新版本** | npm 两源（npmmirror / npmjs）最新均 `0.1.5-rc.2`（2026-09-10） |

**本机实验（决定性）**：置 `process.pkg` 后以同一 dsh 真 boot：

| 模式 | `profiles/node_modules` 软链数 | 目录代理 `entry-*.js` | boot |
|:---|:---|:---|:---|
| 普通 node（现状） | **481** | 0 | ✅ |
| **proxy 模式** | **0** | **1184** | ✅ `dsh web: http://127.0.0.1:3080/?token=…` |

⇒ dsh **本来就有一个完全不建软链的模式**，且该模式在其官方打包发行版中已在用。

## 2. 约束与硬指标

- **不修改 dsh 源码**（AGENTS 红线 1）——本 ADR 不改 dsh 一个字节。
- **不要求管理员/开发者模式**（ADR-0017 同一承诺）。
- **不改 dsh 的默认行为选择之外的任何东西**：只影响「用哪个链接方式」这一个分支。
- **失败要响亮**：若未来 dsh 改了判据导致本方案失效，必须**可诊断**，不得静默退化。
- **不扩大分叉面**：POSIX 侧保持 dsh 默认（那里建软链无特权问题）。

## 3. 备选方案及评估

| 方案 | 做法 | 评估 |
|:---|:---|:---|
| **A. 以 proxy 模式启动**（置 `process.pkg`） | 壳自写 bootstrap：置标志 → import dsh 入口 → 调其导出的 `runCli()` | **✅ 采纳**。零 dsh 改动；一行级语义；实测软链 481→0 且 boot 正常 |
| B. 预建 junction | 壳先把 fallback 条目建成 junction（Windows 无需特权） | **否决**：需**复刻 dsh 的依赖闭包算法**（`resolveModuleFallbackEntries`/`dependencyClosure`）——正是本仓库反复付代价的"重复实现"，且 dsh 一升级就漂移 |
| C. 只报上游，要求开发者模式 | 不改产品 | **否决**：不能兑现「普通账户可用」；但**上游报告仍要做**（见 §5） |
| D. 改用 pkg 打包的 dsh 发行版 | 装官方打包体（`process.pkg` 自然为真） | **暂不采纳**：当前 npm 上没有该形态的独立平台包（`bin` = `lib/bin.js` 纯 JS）；若上游将来发布，应优先改用此路（比置标志更正当） |

## 4. 决策

**采纳方案 A：Windows 上由壳自写的 bootstrap 启动 dsh，使 dsh 走「模块代理」分支。**

### 4.1 形态

```
<engines>/bin/
├── node                     # 既有
├── pnpm                     # 既有
├── dsh                      # 【不变】POSIX shim（dsh 默认符号链接模式）
├── dsh.cmd                  # 【改】Windows：指向 bootstrap，而非直接指入口
└── dsh-boot.mjs             # 【新】壳自写 bootstrap（纯文本）
```

bootstrap 职责（三步，全部是**选择 dsh 自带的模式**，不含任何业务逻辑）：

1. `process.pkg ??= { dshDock: true }` —— 使 `isPackagedExecutable()` 为真，选中 proxy 分支；
2. `await import(<dsh-runtime>/…/@deepseek-ai/dsh/lib/bin.js)` —— 入口相对 bootstrap 自身解析；
3. `await mod.runCli()` —— **必须显式调用**：被 import 时 `import.meta.main` 为假，
   dsh 的 `if (import.meta.main) await runCli()` 不会执行（本机实验已踩，见 §1）。

### 4.2 为什么只改 Windows

POSIX 建软链不需要特权，dsh 默认模式在那里工作正常。**改 POSIX 只会白churn 481 个链接
而不带来任何收益**，且偏离上游默认。分叉面严格限制在「Windows 需要绕开特权」这一件事上。

### 4.3 迁移安全性（已核实，无需兼容代码）

`ensureModuleProxy` 对**已存在的软链**是 `unlinkSync(link)` 后重建 —— **会自行替换，不抛错**；
只有「真实目录且非 dsh 托管代理」才抛。⇒ 从 symlink 模式切到 proxy 模式**对存量安装安全**
（含曾在开发者模式下成功运行过的 Windows 用户）。

## 5. 后果

**收益**：Windows 普通账户下 dsh 启动不再需要符号链接特权 ⇒ **本地模式首次真正可用**
（ADR-0017 负责"装得上"，本 ADR 负责"跑得起来"）。

**代价 / 接受的取舍（如实登记）**：
1. **依赖 dsh 的内部判据 `process.pkg`**。今日该判据只有 1 处调用、语义单一，但**属未公开内部约定**，
   上游 `0.1.5-rc.2` 之后可能变更。缓解见 §6。
2. **proxy 模式与 symlink 模式的条目集合不完全相同**：proxy 只为「可解析出入口」的包建代理
   （既无 `exports` 也无 `main` 的包会被跳过）。这是 dsh 打包形态下**同样成立**的行为，判定为可接受。
3. **仍属对上游缺陷的绕行，不是修复**。须同时向 dsh 报缺陷（§5.1）。

### 5.1 上游报告（必做）

缺陷描述：dsh 在 Windows 普通账户（未开开发者模式、非管理员）下无法启动 —— 
`profiles/node_modules` 的模块 fallback 用 `fs.symlinkSync`，而该平台该权限不可得。
建议上游：把 `isPackagedExecutable()` 扩为「打包体 **或** Windows 无符号链接特权」，
或在该平台改用 junction / 目录代理。

## 6. 验证方式

- **本机可验（已做一次，实施后须重做）**：置标志真 boot → 断言 `profiles/node_modules` **零软链**、
  dsh 能起、`entry-*.js` 代理存在。**这是本 ADR 的核心判据**（实现无关，直接验结果）。
- **单元/契约级**：bootstrap 内容断言（三步齐备、入口相对解析）；Windows shim 指向 bootstrap；
  POSIX shim **不得**被改动（分叉面守门）。
- **失败响亮（绊线）**：bootstrap 捕获 `code === 'EPERM' && syscall === 'symlink'` 并**改写为
  可行动错误**（指明「dsh 的 fallback 模式判据可能已变，见 ADR-0018」），使未来 dsh 内部变动
  **不表现为一条看不懂的 EPERM**。
- **Windows 真机判据**：`%USERPROFILE%\.dsh\profiles\node_modules` 下**无符号链接**、
  本地模式能进工作台。

## 7. 关联决策

- **前置**：ADR-0017（dsh 改 project 内安装）—— 本 ADR 是其必然续篇：解决同一根因的第二处显形。
- **不变**：ADR-0010（引擎=壳资产）、ADR-0009 口径 2（pnpm 硬依赖）。
- **再评触发**：① dsh 上游发布「Windows 免特权」修复；② dsh 发布独立平台打包体（应改用方案 D）；
  ③ dsh 变更 `process.pkg` 判据（绊线会报）。
