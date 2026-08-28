# 交接文档:4.3 Profile 创建修复 · add → install(原始版语义)

- **交接日期**:2026-08-28
- **接手对象**:zcode(或任何无本会话上下文的执行者)
- **交接基线**:工作区**含半成品未提交变更**(见 §3),**维护者会先提交再交接**,开工前先 `git log` / `git status` 确认基线入库,勿在脏工作区上开工(尤其 `src-tauri/src/profiles.rs` 有我的半套改动)
- **本文档目的**:让接手者跳过全部决策过程,直接执行实现;所有裁定已落盘,不得重新讨论
- **执行状态(2026-08-28 晚更新)**:✅ **已执行完成**。本会话按 §4 清单实现全部项(profiles.rs / lib.rs / 前端三处 / ledger / ADR-0009 / roadmap / broadcasts),验证清单 §5 全绿(cargo test 126 绿、fmt/clippy 过、前端 typecheck/lint/test 40 绿、实机 install 162ms 零网络 + dump-config 退出码 0)。经交接人核对,当前工作区 `git diff` 与 §4 预期完全一致。本文档留作决策与实现依据的记录;§3 的"半成品"描述为交接时点快照,已过时。

---

## 0. 任务一句话

**修复"新建 profile 很慢/失败"的根本原因**:创建转发链从 `dsh plugin --profile <名> add @deepseek-ai/dsh-base` 改为 `dsh plugin --profile <名> install`,让创建 = 建一个「只有内置插件声明」的原始 profile,零网络、毫秒级、版本语义免疫。

---

## 1. 背景与根因(AI 协作分析,证据已闭环,不必重验)

### 1.1 现象

用户(本机 macOS,guan)发现新建 profile 极慢(分钟级)或直接失败。`~/.dsh/profiles/test` 的创建日志
(`~/Library/Application Support/io.github.realguan.dsh-dock/profile-create.log`,2026-08-28 17:40)显示:

```
dsh: initialized profile test at /Users/guan/.dsh/profiles/test
Progress: resolved 1, reused 0, downloaded 0, added 0
WARN GET https://r.cnpmjs.org/@deepseek-ai/dsh-bash-env error (ECONNRESET). Will retry in 10 seconds...
WARN GET https://r.cnpmjs.org/@deepseek-ai/dsh-tasks-local error (ECONNRESET). Will retry in 10 seconds...
... (37+ 个包)
ERR_PNPM_META_FETCH_FAIL GET https://registry.npmmirror.com/@deepseek-ai/dsh-compact-basic:
  request to https://r.cnpmjs.org/@deepseek-ai/dsh-compact-basic failed,
  reason: Client network socket disconnected before secure TLS connection was established
This error happened while installing the dependencies of @deepseek-ai/dsh-base@0.0.1-rc.1
dsh: pnpm failed in profile directory /Users/guan/.dsh/profiles/test
```

### 1.2 根因(三层)

1. **版本错位(根)**:`create_command_args`(profiles.rs)传裸包名 `@deepseek-ai/dsh-base` → pnpm 按 `latest` dist-tag 解析 → **`0.0.1-rc.1`**(已弃用旧版)。该版本依赖 37+ 个旧命名空间的包(`dsh-bash-env`、`dsh-tasks-local`、`dsh-skill-local`、`dsh-permission`…)在 registry 上**全部 404**(包已删/改名)。
2. **404 再路由(放大器)**:npmmirror 对缺失 scoped 包返回 404 后,pnpm 打到死域名 `r.cnpmjs.org`(本机解析到保留段 `198.18.0.192`,TLS 直接断)。pnpm 按 10s→60s 递增重试 × 37 包,单包最多 2 分钟,累积远超 `CREATE_FORWARD_TIMEOUT`(600s)。
3. **示例对照**:`dsh-base@0.1.1-rc.2`(正确版本)的 77 个依赖在 npmmirror 全部 200。装对版本不存在这个问题。

### 1.3 为什么说它是"根本性"问题

- **in-box bundle 本不该走网络**:`@deepseek-ai/dsh-base` 是内置插件,随 dsh 安装。dsh `resolveBundleDir` 双锚点(先 dsh 安装目录 `INSTALL_ANCHOR`,再 profile node_modules)——它**从 dsh 安装目录即可解析**,profile 里根本不需要 pnpm 装它。`dsh plugin add` 的转发语义却硬要把它拖进 dependencies 并联网装一遍。
- **dist-tag 是隐性漂移点**:壳锚定 dsh 源码 `DEFAULT_PROFILE_BUNDLES = ["@deepseek-ai/dsh-base"]`(app-boot @ 334)的**字符串**,但 `latest` tag 指向什么不在 dsh 控制内。上游把 `latest` 停在 0.0.1-rc.1、当前版本走 `next` tag → 壳"逐字复刻"踩空。**ledger 复现点 7 只登记了路径与转发链行为,没登记版本语义**——升级后可能再踩。
- **Spike A 为何掩盖**:`docs/spikes/0001-pnpm-forward-chain.md` §3.2 实测输出 `resolved 1, reused 0, downloaded 0, added 0` —— 零网络全命中本机 store(store 已有旧版完整依赖时旧版也能"装成功"),于是得出"转发链可用"结论,恰好没暴露版本错位这一层。

### 1.4 用户确认的修复语义(2026-08-28 拍板)

"profile 创建一个原始的只有内置插件的版本就行了" —— **方案 B(`install`)**。已实测:

```
$ DSH_HOME=/tmp/dshhome-probe dsh plugin --profile fresh1 install
dsh: initialized profile fresh1 at /tmp/dshhome-probe/profiles/fresh1
Already up to date
Done in 199ms using pnpm v10.24.0     ← 零网络,199ms

$ dsh --profile fresh1 --dump-config   ← 验证可启动,退出码 0
# == @deepseek-ai/dsh-base (timer/hmr/llm/session …)
```

`package.json` 产物:`dependencies: {}`、`dsh.profile.bundles: ["@deepseek-ai/dsh-base"]`(原始版)。

---

## 2. 已裁定的决策(全部落盘,禁止重新提出)

| 决策 | 结论 |
|:--|:--|
| **创建路径** | 固守 ADR-0009 **方案 A(spawn `dsh plugin` 转发链)**,不改为壳侧复刻(ADR 方案 B/C 仍否决);**仅修订执行细则**:`add <bundle>` → `install` |
| **创建语义** | "原始 profile" = dsh initProfile 写三件套(`PROFILE_TEMPLATES[名] ?? DEFAULT_PROFILE_BUNDLES` = `@deepseek-ai/dsh-base`)+ `pnpm install`(空 dependencies → `Already up to date`);**bundles 列表含 dsh-base,本体随 dsh 安装目录解析,零下载** |
| **pnpm 口径** | 维持 ADR-0009 红线 2 口径 2:pnpm = boot 硬依赖;创建时保留防御性检测(`ensure_pnpm`),缺失经 `npm i -g pnpm` 补齐;**不因 install 变纯本地而移除** |
| **版本语义** | `install` 不解析任何 dist-tag,版本漂移免疫——**这是选择 install 而非 add@版本 的根本理由(方案 A"传版本 spec"否决)** |
| **后续外挂插件** | 加插件走同一条 `dsh plugin --profile <名> add <包>` 链(归 4.4 插件管理),本任务不实现 |
| **ADR 措辞** | ADR-0009 用户对话里的"A/B"与 ADR 方案 A/B **不是一回事**:ADR 方案 A = spawn 转发链主路径(保留);ADR 方案 B = 壳侧复刻三件套(仍否决)。本任务 = **方案 A 的执行细则修订**,在 ADR §4 记录 |

---

## 3. 工作区当前状态(⚠️ 半成品,如实交接)

**维护者会先将以下半套改动提交为基线,或由接手者在下一条 commit 里并入。**
**(2026-08-28 交接时点实测 `git status`:M profiles.rs + M lib.rs,均有未提交改动;handoff 文档 新增。)**
**⚠️ 注:`src-tauri/src/lib.rs` 的改动不在本会话 AI 操作记录内(本会话只编辑过 profiles.rs 与新建本文档),疑似维护者同时手动修改——内容恰为任务所需,接手者开工前请确认其来源与正确性。**

`src-tauri/src/profiles.rs` **已改**(`git diff` 可见,标注 2026-08-28):

- `create_command_args`:参数序列 `["plugin","--profile",名,"add","@deepseek-ai/dsh-base"]` → `["plugin","--profile",名,"install"]` ✅
- 删除 `const CREATE_ADD_BUNDLE` ✅
- `CreateProfileOutcome.installed` 注释:"基础插件已装" → "原始 profile 就绪" ✅
- `classify_create_outcome` 成功文案:"已创建,基础插件(@deepseek-ai/dsh-base)安装完成" → "已创建:内置插件声明就绪,可立即启动使用" ✅
- 该函数 doc 注释 + `create_command_args` doc 注释(含版本坑说明)✅

`src-tauri/src/lib.rs` **已改(来源待确认)**:

- `create_profile` 命令 doc(第 484-488 行):"add @deepseek-ai/dsh-base" → "install" + 原始版语义说明 ✅(内容符合 §4.2 预期)

**未改 / 待办(接手者完成)**:

- `profiles.rs` 顶部模块 doc(第 6 行"spawn `dsh plugin --profile <名> add @deepseek-ai/dsh-base` 半官方转发链" → 改 install 语义)
- `creation_blocker` doc 注释(**不需要改逻辑**,但注释里"重跑同名 add"可顺带校正为"重跑同名 install/创建"——非强制,建议改)
- **测试**:`create_args_forward_profile_name_verbatim` 断言仍是 `["plugin","--profile","my profile","add","@deepseek-ai/dsh-base"]`(**会红**);`classify_covers_dsh_forward_chain_outcomes` 的 fixture(成功路径 `Progress: resolved 1...` 与"安装完成"断言)需更新
- **前端**:
  - `frontend/src/content/zh-CN.ts`:`createDefaultHint`("非模板名将以 @deepseek-ai/dsh-base 初始化")、`createBusy`("首次创建需经 pnpm 安装依赖,视网络数十秒到数分钟")、`createDoneReady`("创建完成,基础插件已安装")→ 全部改为 install 语义(秒级、零网络、内置插件声明就绪)
  - `frontend/src/components/profiles/ProfileCreateDialog.tsx` 顶部注释(第 2-3 行"创建可长时阻塞(pnpm 安装)")→ 更新;`createDefaultHint` 展示(第 109 行)依赖 zh-CN 文案
- **文档**:
  - `docs/contracts/dsh-behavior-ledger.md` 复现点 7:创建 = `install`(非 add);**新增 dsh 升级复核列"dist-tag 版本语义"**(ledger 当前只登记路径与转发链行为)
  - `docs/adr/0009-profile-manager.md` §4(方案 A 决策文本):`add <bundle>` → `install` + 修订理由(原始版语义;裸包名 dist-tag 版本坑;in-box bundle 双锚点零下载);§5 行动项"实现 4.3"勾选行同步。**不要改动方案 A/B/C/D/E 的评估结论**(它们讨论的是"spawn 转发链 vs 壳侧复刻",与本次执行细则修订无关)
  - `docs/roadmap.md` 第 30 行(出厂 profile 模板事实边界)与第 143 行(关键行动 ②):`add <bundle>` 表述 → `install` 语义
  - `docs/broadcasts.md`:**追加**一条 2026-08-28 完成通知(格式照 §参考)
  - 可选:`docs/spikes/0001-pnpm-forward-chain.md` 遗留事项(§5 第 3 条"profile 名为 web/headless 时的模板命中")可加一句已收编(install 走模板命中),非强制

---

## 4. 实现清单(按序,每项完成后才动下一项)

### 4.1 `src-tauri/src/profiles.rs`

1. **顶部模块 doc**(第 1-29 行区块):创建段从"spawn `dsh plugin --profile <名> add @deepseek-ai/dsh-base` 半官方转发链"改为"spawn `dsh plugin --profile <名> install`——仅 init + pnpm install,原始 profile 语义;add 路径留待 4.4 插件管理"
2. **`creation_blocker` doc**(第 258-263 行):"重跑同名 add" → "重跑同名创建(install,幂等)"(逻辑不变:半初始化/空依赖放行仍是正确重试语义)
3. **测试**:
   - `create_args_forward_profile_name_verbatim`:期望值改 `["plugin","--profile","my profile","install"]`;第二个断言(web/headless 模板名)改 `["plugin","--profile","web","install"]`;注释改「install 对模板名也触发 init(模板命中)」
   - `classify_covers_dsh_forward_chain_outcomes`:
     - ① 成功 fixture:`"Progress: resolved 1..."` 改为 dsh 实际输出 `"dsh: initialized profile alpha at /tmp/x/profiles/alpha\nAlready up to date\nDone in 207ms using pnpm v10.24.0\n"`;断言 `ok.detail.contains("安装完成")` → `ok.detail.contains("内置插件声明就绪")`(或 "可立即启动")
     - ② ③ ④ ⑤ fixture 不变(pnpm 缺失 / pnpm 失败 / dsh 失败 / 超时 语义仍成立)——但 ② 的断言 `no_pnpm.detail.contains("npm install -g pnpm")` 保留(可行动建议文案仍存在)
   - ⚠️ `CREATE_ADD_BUNDLE` 常量已删,若有测试引用会编译错(前面 grep 未见,但跑一遍确认)
4. **不留** `CREATE_ADD_BUNDLE` / "add" 残留字面量(除 4.4 相关注释外,见 §4.4)

### 4.2 `src-tauri/src/lib.rs`

- `create_profile` doc(第 484-488 行):"spawn `dsh plugin --profile <名> add @deepseek-ai/dsh-base` 半官方转发链" → "spawn `dsh plugin --profile <名> install`";相应"阻塞动作(系统探测 + 转发链最长 10 分钟)"注释保留(超时仍 600s 兜底,但实际秒级)

### 4.3 前端

- `frontend/src/content/zh-CN.ts`(profiles 段):
  - `createDefaultHint`:`"非模板名将以 @deepseek-ai/dsh-base 初始化"` → `"非模板名将初始化为基础工作台(内置插件)"`(或类似——名字合法且非模板时展示)
  - `createBusy`:`"创建中…首次创建需经 pnpm 安装依赖,视网络数十秒到数分钟"` → `"创建中…仅初始化内置插件,通常秒级"`(或类似——明确不再有长网络等待)
  - `createDoneReady`:`"创建完成,基础插件已安装"` → `"创建完成,基础工作台就绪"`(或类似)
  - `createNameHelp` / `createTemplateHint` / `createDonePending` / `createDoneFailed` / `createAgain`:**不变**(模板名语义、pending/failed 状态、重试语义不受影响)
- `frontend/src/components/profiles/ProfileCreateDialog.tsx` 顶部注释(第 2-3 行):"创建可长时阻塞(pnpm 安装)" → "创建为纯 init+install,秒级";busy 态文案从 zh-CN 取,无需改逻辑
- `frontend/src/lib/profiles.ts` / `frontend/src/types/ipc.ts` / `frontend/src/lib/tauri.ts`:字段 `installed`/`materialized` **不变**(后端契约未变);`summarizeCreateOutcome` 逻辑**不变**。仅确认 `installed=true` 语义更新("基础插件已装"→"原始 profile 就绪")不破坏展示层(实为更准确)

### 4.4 边界确认(不要动)

- `PROFILE_TEMPLATES` / `TEMPLATE_BUNDLES` / `web`/`headless` 模板 bundle 列表:**不改**(install 与 add 走同一 initProfile,模板命中逻辑不变)
- `creation_blocker` 逻辑(非法名 / 文件占用 / 半初始化放行重试 / 完备重名拒绝):**不改**
- `run_dsh_plugin` / `create_profile_blocking` / `ensure_pnpm` 防御检测链:**不改**(install 纯本地,但 pnpm 仍是 dsh spawnSync 的硬前提,防御检测保留)
- `CreateProfileOutcome` 字段(`profile`/`materialized`/`installed`/`detail`):**不改**(IPC 兼容)
- `copy_profile` / `rename_profile` / `delete_profile` / `defaultProfile`:**完全不相干,勿动**

---

## 5. 验证清单(合入前全绿)

### 5.1 Rust

```bash
cd src-tauri && cargo test            # 期望:112 绿 + 新增/修改后全绿(基线 108+4;install 改动预计不变数或 -1/+1)
cd src-tauri && cargo fmt --check     # 必须过
cd src-tauri && cargo clippy -D warnings
```

### 5.2 前端

```bash
cd frontend && npm install           # 如未装
cd frontend && npm run typecheck
cd frontend && npm run lint
cd frontend && npm run test           # 期望:profiles.test.ts 的 summarizeCreateOutcome 不变(纯逻辑),仅 zh-CN 文案测试若有需同步
```

### 5.3 实机验证(**必须做**,临时 DSH_HOME 零污染)

```bash
export PATH="$HOME/.local/state/fnm_multishells/29539_1787909128920/bin:$HOME/.npm-global/bin:$HOME/Library/pnpm:$PATH"
mkdir -p /tmp/dshhome-verify && cd /tmp
DSH_HOME=/tmp/dshhome-verify dsh plugin --profile verify1 install
# 期望:init 行 + "Already up to date" + "Done in ~200ms" + exit 0,无任何 WARN/404
ls /tmp/dshhome-verify/profiles/verify1/                       # 三件套 + pnpm-lock.yaml(install 空依赖也会写 lockfile,实测确认)
cat /tmp/dshhome-verify/profiles/verify1/package.json          # dependencies:{}, bundles:[dsh-base]
DSH_HOME=/tmp/dshhome-verify dsh --profile verify1 --dump-config >/dev/null 2>&1; echo $?   # 期望 0(可启动)
rm -rf /tmp/dshhome-verify
```

**Windows 平台**:`install` 与 `add` 走同一 `spawnSync("pnpm", …, shell: win32)` 转发路径,风险同源可控;按 Spike A 遗留事项,若条件允许补一次 Windows 实机(本任务不阻塞)。

### 5.4 人肉 git diff 复核(收尾三件事第 2 件)

- 确认 `git diff` 无越界改动(理论上仅:profiles.rs、lib.rs、zh-CN.ts、ProfileCreateDialog.tsx 注释、ledger、ADR-0009、roadmap、broadcasts)
- 确认 `CREATE_ADD_BUNDLE` / "add @deepseek-ai/dsh-base" 无残留(除 4.4 相关说明性注释)
- 确认 ADR-0009 方案 B/C(壳侧复刻)仍明确"否决",未误删

---

## 6. 文档同步清单(合入前完成,按 CONTRIBUTING)

| 文档 | 改动 |
|:--|:--|
| `docs/contracts/dsh-behavior-ledger.md` | 复现点 7:创建 = `install`;加"dist-tag 版本语义"复核列(或复现点 7 备注行) |
| `docs/adr/0009-profile-manager.md` | §4 决策文本:`add <bundle>` → `install` + 修订理由;§5 行动项勾选行同步 |
| `docs/roadmap.md` | 第 30 行 + 第 143 行:`add <bundle>` 表述 → install 语义 |
| `docs/broadcasts.md` | 追加 2026-08-28 完成通知(格式照 §参考) |
| AGENTS.md | **无需改**(§7 IPC 命令表 `create_profile` 已存在,命令名不变;§9 ADR 索引行不变) |

**广播落档参考格式**(照 `docs/broadcasts.md` 既有条目):

```
### 2026-08-28 完成通知 · 4.3 Profile 创建修复(add → install 原始版语义)—— guan(AI 协作)

- 变更:本 commit——`src-tauri/src/profiles.rs`(create_command_args 改 install、
  删 CREATE_ADD_BUNDLE、分类文案与注释);`lib.rs`(create_profile doc);
  `frontend/src/content/zh-CN.ts`(创建文案);`ProfileCreateDialog.tsx`(注释);
  `docs/ledger`/`ADR-0009`/`roadmap`/`broadcasts`。
- 根因:裸包名 add @deepseek-ai/dsh-base 按 latest dist-tag 解析到弃用旧版
  0.0.1-rc.1,其依赖 37+ 旧包名已删 → 404 重试拖慢/失败;install 零网络、
  版本语义免疫(原始 profile = 内置插件声明,bundle 本体随 dsh 安装目录解析)。
- 影响:创建语义从"装基础插件"变为"建原始 profile";`installed=true` 含义
  更新;pnpm 防御检测保留(ADR 红线 2 口径 2 不变)。
- 凭据:cargo test N 绿 / fmt / clippy 全过;前端 typecheck / lint / test 过;
  实机验证(临时 DSH_HOME)install 199ms 零网络 + dump-config 可启动;
  Windows 转发链仍为遗留(与 add 同源)。
```

---

## 7. 疑难提示(接手者易踩坑)

1. **区分两套"A/B"命名**:ADR-0009 方案 A(spawn 转发链,保留)/ 方案 B(壳侧复刻,否决)= 对话里的 A(传版本 spec)/ B(install)。**本任务改的是"方案 A 的执行细则",不是切换方案**。
2. **`dsh --profile <名> --dump-config` 可用作"原始 profile 可启动"的回归验证**——它组合 bundles(双锚点解析)并打印配置,退出码 0 即证明 bundle 从 dsh 安装目录解析成功。
3. **测试 fixture 的 pnpm 输出真实形态**:`install` 成功为 `Already up to date` + `Done in 199ms using pnpm v10.24.0`(无 `Progress: resolved/downloaded` 行那种 add 专属形态)。**不要**把 add 的 fixture 输出照抄进 install 测试。
4. **README.i18n / 其他 i18n**:仓库仅 `zh-CN.ts` 一处文案(见 `frontend/src/content/`),i18n 键名不变,只改值。
5. **`npx pnpm` 不存在**:别用 npx,直接用 `~/Library/pnpm/pnpm` 或 PATH 里的 pnpm。
6. **若不改 docs 而只改代码,cargo test 仍绿**(文档无测试),但按 AGENTS §8 收尾三件事第 3 件与 §0 条约,文档必须同步——禁止"文档留到下一条"。
7. **完成时记得 `git add` 全部改动(含 docs)**,不要只提交代码。
