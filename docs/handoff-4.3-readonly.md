# 交接文档:4.3 Profile 管理器 · 第二刀(只读能力)

- **交接日期**:2026-08-28
- **接手对象**:zcode(或任何无本会话上下文的执行者)
- **交接基线**:工作区当前未提交变更 = 上一刀已完成的成果(IPC 闸门 + ADR 文档修订),**维护者会先提交再交接**;开工前先 `git log` 确认基线已入库,勿在脏工作区上开工
- **本文档目的**:让接手者跳过全部决策过程,直接进入实现;所有裁定已落盘,不得重新讨论

---

## 1. 必读(按序,开工前全部读完)

1. **`AGENTS.md`** — 项目宪法,全读。尤其:§0 红线两条、§4.1/4.2 Rust 规范与禁止项、§5 测试、§7 IPC 与网络面、§8 AI 交互约束(增量生成 / 必带测试 / 先读后写 / 收尾三件事)
2. **`docs/adr/0009-profile-manager.md`** — 本功能技术方案(已接受),行动项即任务清单
3. **`docs/spikes/0001-pnpm-forward-chain.md`** — 转发链实测(pnpm 如何被 dsh 内部调用)
4. **`docs/spikes/0002-profile-reference-surface.md`** — profile 引用面全清单(后续删除/重命名刀的依据)
5. **`docs/roadmap.md` §4.3** — 五步关键行动与退出条件
6. **`docs/contracts/dsh-behavior-ledger.md`** — 复现台账(你实现的每个「文件系统层复现 dsh 行为」都要入册)

## 2. 已裁定的决策(全部已落盘,见对应文档;禁止重新提出)

| 决策 | 结论 | 落盘处 |
|:--|:--|:--|
| profile 创建路径 | 走半官方 `dsh plugin --profile <名> add <bundle>` 转发链;壳侧复刻三件套已否决(方案 B/C) | ADR-0009 |
| pnpm 依赖口径 | pnpm = 环境检查 **boot 硬依赖**(口径 2:保证 dsh 全部子命令可用);缺失经 `npm i -g pnpm` 补齐(复用 ADR-0005 npm 链);创建/操作时**保留防御性检测** | ADR-0009 红线 2 |
| WSL 依赖补齐 | 客体内与本地同口径:node(与本地档同源、tarball 落 `~/.dsh-dock/node`)→ pnpm → dsh 全自动 | ADR-0004 §7 |
| 三件套写入边界 | 壳**不得生成/复刻**三件套内容;既有三件套的整目录复制 + `name` 一致化改写允许 | ADR-0009 红线 3、AGENTS §6 |
| defaultProfile 失效回退 | 定死 `web`(模板名恒可首启) | ADR-0009 §4 |
| 内置模板名 | 仅 `web`/`headless`,非模板名 init 用 `@deepseek-ai/dsh-base` | ADR-0009 §4 |

## 3. 上一刀成果(已完成,勿重做):IPC 三处同步机器闸门

- `src-tauri/src/ipc.rs` = IPC 命令**单一事实源**(`COMMANDS` 常量)+ `gate_tests` 一致性测试
- `build.rs` 经 `#[path]` 由常量生成 AppManifest(不再手写);capabilities 引未知权限由 tauri-build 构建期免费拦截
- **新增任何 IPC 命令的固定流程**:① `ipc.rs` COMMANDS 登记 → ② AGENTS §7 登记 → ③ `lib.rs` generate_handler + `capabilities/default.json` allow-* 落地。漏任何一处 `cargo test` 直接红(负验证已做过)
- 基线:cargo test **98 绿** / `cargo fmt --check` / `clippy -D warnings` 全过

## 4. 本次任务:第二刀——只读能力

**意图一句话**:用户能在面板里看到所有 profile(已物化的 + 内置模板名)并查看单个 profile 详情;纯读,零写入,零 dsh 子进程。

### 4.1 命名校验模块

- 与 dsh `resolveProfileDir` **逐字一致**:拒绝 空名 / 含 `/` / 含 `\` / `.` / `..` / 字面量 `node_modules`
- 源码锚定(2026-08-28 核对,dsh v0.1.1-rc.2):`dsh-app-boot/lib/index.js @ 318`
- ⚠️ **行号勘误**:早期文档写的 `@ 11826` / `@ 13418` 是错的(该文件仅 1216 行,混入了早期 bundle 行号)。正确行号:`resolveProfileDir @ 318`、`PROFILE_TEMPLATES @ 323`、`DEFAULT_PROFILE_BUNDLES @ 334`、`initProfile @ 353`、`healProfilesModuleFallback @ 409`、`runPlugin @ lib/plugin-9h8shc4d.js:101`、pnpm 报错文案 `@ 115`
- 校验函数带日期注释,入 ledger §二(复现点 8 已预登记)

### 4.2 profiles 扫描器

- 扫描 `$DSH_HOME/profiles/` 目录,读各子目录 `package.json`:解析 `dsh.profile.bundles`、`dependencies`
- 返回结构须区分两种状态:**已物化 profile**(目录存在)vs **内置模板名可首启**(`web`/`headless`,首次启动才物化)——选择器/列表两者合并展示
- ⚠️ **禁止复用 `list_web_ui_profiles`**(`resolve.rs:763`):那是 webUi 选择器原型,无条件注入 `"web"` 并跳过同名目录,语义不同
- home 路径解析复用壳既有逻辑,不能假设 `$DSH_HOME` 环境变量必然等于实际 home(resolve 优先级:显式配置 > 环境变量 > `~/.dsh`)
- 纯函数 + 内联 fixture,不读真实用户目录

### 4.3 IPC 命令

- `list_profiles`(返回上述扫描结果)+ `get_profile_detail`(返回单个 profile 的 package.json 关键字段 + `cordis.patch.yml` **原文**)
- 严格走 §3 的三步流程;做完跑 `cargo test gate_tests` 确认绿
- ⚠️ **YAML 依赖注意**:`serde_yaml` 上游已归档弃维(2024)。本刀详情页建议**只展示 patch 原文不解析**,把 YAML 依赖的引入(选型 serde_norway 等)推迟到启停插件的刀;若必须引入,先按 AGENTS §5 说明理由

### 4.4 范围外(本刀不做,别顺手做)

- 创建/复制/重命名/删除(后续刀);`--dump-config` 详情(spawn dsh 有启动期副作用,需先实机确认是否早退/纯读);WSL 客体内 profile(**范围声明:管理器仅覆盖壳侧 home**);pnpm 补齐实现(单独一刀,落 updates.rs)

## 5. 硬边界(踩线即打回)

1. **不修改 dsh 源码**(不 fork / 不上游 patch);文件系统层复现其行为须:锚定源码位置 + 日期注释 + 入 ledger
2. **dsh 文件系统不变量**(AGENTS §6):三件套只读(本刀本来就读);`.credentials.yaml` / 会话目录 / `profiles/node_modules` 农场一律不碰
3. **Windows 子进程必须经 `crate::child_cmd`**,禁裸 `Command::new`;**网络只在 `updates.rs`**(本刀无网络)
4. **不引 dev-dependencies**;测试全部纯函数 + 内联 fixture
5. **一次会话只做这一个意图**;发现超范围问题记下来报告,不顺手修
6. 裁定性代码注释带日期;关键逻辑中文注释、命名英文

## 6. 验证与收尾(AGENTS §8.5)

1. `cargo test` 全绿(基线 98,本刀新增测试只增不减)——新命令加完后 `gate_tests` 必须仍绿
2. `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings`(参考:build.rs 侧 `#[path]` 模块曾有 dead_code 坑,clippy 会拦)
3. 人肉读 `git diff` 确认无越界 → 按 `docs/CONTRIBUTING.md` 提交 → 频道广播 + 落档 `docs/broadcasts.md`
4. 收尾时更新 ADR-0009 §5 行动项勾选状态、ledger §二 复现点 6/8 入册

## 7. 参考实现风格

- 上一刀 `src-tauri/src/ipc.rs`:模块头注释说明职责与消费关系 / 测试模块 `#[cfg(test)] mod xxx_tests` / 断言失败信息中文且可行动(点名缺失项 + 后果 + 修复方向)
- `settings.rs`:纯函数 + 内联 fixture + 临时目录测试的既有风格
