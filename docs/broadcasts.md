# 📢 广播记录（公共频道知会档案）

> 协作知会的**仓库内正式载体**。依据 [`CONTRIBUTING.md`](./CONTRIBUTING.md) §0——
> 「所有必须共享的信息，唯一合法载体是仓库落盘内容」：聊天频道用于即时协调
> （占用抢先后到者得），但消息沉底即失忆、AI 冷启动读不到；知会类事件必须在此
> 落档。检索、审计、纠纷回溯一律以本档为准。
> 本文件是普通区 append-only 文档：只追加不改写历史条目（同 ADR 惯例），
> 不走宪法级修改流程。

## 一、登记范围（触发即记，当天落档）

| 类型 | 触发点 | 依据 |
|:---|:---|:---|
| 宪法级文件改动 | `AGENTS.md` / `docs/contract.md` / `CONTRIBUTING.md` / `node-map/` 的改动预告与合入归档 | AGENTS §10；CONTRIBUTING §3 |
| 快车道直推 master | 直合完成即记（单人小改通道同样适用） | CONTRIBUTING §2 流程图 I→M |
| PR 合并完成 | 合并人 squash 后记录 reviewer 与结论 | CONTRIBUTING §2 流程图 M |
| 发版事项 | 打 tag、冻结期起止、Release notes 征集与三平台验收 | CONTRIBUTING §8 |
| 占用声明/释放 | 频道声明后补一行即可（时效判定仍以频道时间戳为准） | CONTRIBUTING §3 |

## 二、条目格式

倒序追加（最新在上），一条一个三级标题：

```
### YYYY-MM-DD <类型> · <一句话主题> —— <发起人>
- 变更：<commit hash / 分支 / 文件清单>，两三行摘要
- 影响：<需要他人做什么动作；无需动作写「仅周知」>
- 凭据：测试结果 / diff 规模 / 频道消息时间点
```

漏记不补改旧条目——另发一条「补记」并注明原委。

## 三、记录

### 2026-08-28 文档修正 · README/CONTRIBUTING 面向前端迁移与 Profile 管理器现状更新 —— guan（AI 协作）

- 变更：本 commit——`README.md`（「它做什么」补 Profile 管理器与 pnpm 环境
  保障两条、WSL 条目 settings.json 字段表述更正、开发命令全面修正：前端
  React SPA 的 npm 闸门 + **`cargo run` 须先 `cd src-tauri`**、品牌资源路径
  ui/assets → frontend/public 与 Emblem、结构树重画含 profiles.rs/ipc.rs 等
  新模块）；`docs/CONTRIBUTING.md`（「壳前端免构建（ui/ 静态页）」过时表述
  更正为 React SPA + npm 三闸门）。
- 影响：仅周知。根因是 2026-08-27 前端迁移与 4.3 各刀落地后文档未跟上——
  本机实测 `cargo run` 在仓库根直接失败（Cargo.toml 在 src-tauri/ 下），
  README 旧命令会误导所有新上手者。
- 凭据：文档改动；所有引用路径已 `ls` 核实存在（frontend/public/mark.svg、
  assets/dsh-logo.svg、scripts/*、node-map/README.md 等）。

### 2026-08-28 完成通知 · pnpm 补齐落地（boot 硬依赖 + 创建时复用，ADR 红线 2 收口）—— guan（AI 协作）

- 变更：本 commit——`updates.rs` 新增 `ensure_pnpm`（PATH 可见即返回；
  缺失经 `npm install -g pnpm` 同步补齐，复用 ADR-0005 npm 全局链 +
  ADR-0006 镜像序；补齐后必须在同一 PATH 重新可见，否则按失败处理）+
  `install_pnpm_via_npm`（双镜像逐试、聚合报错；pnpm 纯 JS 不传
  allow-scripts）+ 3 条测试（命中不装 / 装后可见性强制 / 失败聚合文案，
  假 node/npm-cli fixture）；`executor.rs` LocalExecutor::probe 接线
  （boot 期检查 → 失败阻断 boot 出可行动错误卡；基准 = 注入 dsh 的
  PATH）；`profiles.rs` 创建时防御检测升级为「缺失 → 补齐 → 再失败才报
  错」（复用同一函数）；`find_pnpm` 回归私有（外部消费点已被
  ensure_pnpm 收编）。
- 影响：仅周知。ADR-0009 红线 2 口径 2 至此完整：boot 环境检查保证
  「dsh 全部子命令可用」，补齐失败 = 新增 boot 失败模式（可行动文案：
  检查网络 / npm 镜像 / 手动 `npm install -g pnpm`）。WSL 客体内
  node → pnpm → dsh 链仍归 4.9（ADR-0004 §7）。AGENTS §7 网络面登记
  无变化（boot 期 pnpm 补齐已于 2026-08-28 登记）。4.3 遗留仅剩
  Windows 转发链实机验证（Spike A 遗留）。
- 凭据：`cargo test` 125 绿（+4，含恢复一处被测试插入截断的既有测试
  node_download_urls_mirror_first）/ `fmt --check` / `clippy -D warnings`
  全过；diff 已逐行人肉复核。

### 2026-08-28 完成通知 · 4.3④ defaultProfile 消费接线 + WSL 放开评估收口 —— guan（AI 协作）

- 变更：本 commit——`resolve.rs` 新增纯函数 `consume_default_profile`
  （存储默认值 ∈ webUi 候选才消费，含正反例测试）；`executor.rs`
  LocalExecutor::probe 接线——命中即以该 profile 启动并跳过选择器，
  仅覆盖 dsh_home = 用户 home 的档位（system/download），bundle 快照
  世界不适用；未命中（headless 类无 webUi / 已被手工删除）回退常规
  流程并记日志。ADR-0009 §5 WSL GUEST_BOOT 放开评估收口：**本版不放开，
  维持 `--profile web`，归 4.9**（客体 home 与壳侧 home 不同世界 /
  非 webUi 无 URL 可导航 / 客体内 profile 管理属 4.9 范围）。
- 影响：仅周知。「设为默认启动」语义自此完整：设置 → 持久化 → 下次
  启动自动使用（多 webUi 不再出选择器）；选择器仍在（未设默认时），
  其「选择只影响本次会话」语义不变。验收路径：管理器设默认 → 重启
  应用 → 直接进入该工作台（日志可见 defaultProfile 命中行）。
- 凭据：`cargo test` 121 绿（+1）/ `fmt --check` / `clippy -D warnings`
  全过；diff 已逐行人肉复核。

### 2026-08-28 完成通知 · 4.3 Profile 管理器第五刀（前端管理页）+ AGENTS §4.4 重评 —— guan（AI 协作）

- 变更：本 commit——前端 `pages/ProfileManager.tsx` + `components/profiles/*`
  （列表行两态 / 详情 / 创建 / 复制 / 重命名 / 删除确认五组组件）+
  `stores/profilesStore.ts` + `lib/profiles.ts`（校验镜像等纯逻辑，6 条 Vitest）
  + `lib/tauri.ts`（8 个 profile api，共 20 命令全类型化）+ `types/ipc.ts`
  （五个响应类型锚定 Rust serde 形状）+ `content/zh-CN.ts`（profiles 文案段）
  + `App.tsx`（label=profiles 路由）；后端 `lib.rs`（`open_profiles_window`
  镜像 about 主线程约束 + macOS 菜单 / 非 macOS 托盘入口
  `profiles_manager`）+ `capabilities/default.json`（windows 数组加
  `profiles`——ACL 按窗授权，漏加即整页 IPC 静默拒绝）。
- 影响：**触宪法级**两处，仅周知——① AGENTS §4.4 Vitest 重评条件已触发并
  落盘结论：维持纯逻辑测试，RTL/jsdom 不引入（再评触发 = 需 DOM 断言的
  复杂交互）；② TanStack Query 未接入：管理页是「读一次 + 变更后手动刷新」
  形态，frontend-migration §11 的触发条件裁定延后（rationale 见
  profilesStore.ts 头注释，出现跨窗口订阅诉求再立 micro-ADR）。
  范围声明：4.3 全部六项能力的 UI 至此可用（菜单/托盘 → Profile 管理器）；
  defaultProfile 的 boot 消费接线、pnpm 补齐仍归后续刀。前端文案含
  删除确认三要素（不级联全局数据 / 其他 dsh 实例 / 模板名重新物化，ADR
  §2 要求）与创建 pending 中间态（ADR §3 方案 A 契约）。
- 凭据：`npm run typecheck` / `lint` / `test`（40，+6）/ `build` 全绿；
  `cargo test` 120 绿 / `fmt --check` / `clippy -D warnings` 全过；
  diff 已逐行人肉复核。

### 2026-08-28 完成通知 · 4.3 Profile 管理器第四刀（生命周期 + 默认持久化）—— guan（AI 协作）

- 变更：本 commit——`profiles.rs`（复制/重命名/删除文件层 + 前置校验 + 运行中
  防护文案 + patch `../` 引用扫描警告 + 8 条测试）；`settings.rs`
  （`defaultProfile` 字段，第二最小面例外）；`executor.rs`（`active_profile`
  trait 方法：运行中防护比对源，本地取 launch、WSL 固定 web）；`lib.rs`
  （5 个新 IPC 命令；**连带修复** `switch_mode`/`choose_mode` 改
  load-modify-save——原整体覆盖写法会抹掉 defaultProfile）；三处同步
  （ipc.rs/capabilities）。引用面全按 Spike B §3 执行：复制排除 node_modules
  + name 改写；重命名删 node_modules 让 dsh 自愈；删除不级联 sessions；
  defaultProfile 删除时清除、重命名时同步，失效读取侧兜底 web。
- 影响：**触宪法级**——AGENTS §6 例外册登记 `defaultProfile` 落地、§7 IPC
  登记表新增 5 命令（`copy_profile`/`rename_profile`/`delete_profile`/
  `set_default_profile`/`get_default_profile`），仅周知。范围声明：管理器
  后端能力至此齐备（列出/详情/创建/复制/重命名/删除/切换默认）；前端管理页、
  defaultProfile 的 boot 消费接线（含 WSL GUEST_BOOT 放开多 profile 评估）、
  pnpm 补齐均归后续刀。
- 凭据：`cargo test` 120 绿（112 + 新增 8：复制排除与改写 / 重命名自愈 /
  删除不级联 / 运行中防护文案要素 / 默认候选校验 / settings 旧格式兼容等）；
  `gate_tests` 三处同步一致性绿；`cargo fmt --check` / `clippy -D warnings`
  全过；diff 已逐行人肉复核（经维护者裁定 ①+② 范围：生命周期 + 默认持久化
  一刀完成，pnpm 补齐与前端拆分后续刀）。

### 2026-08-28 完成通知 · 4.3 Profile 管理器第三刀（创建能力）—— guan（AI 协作）

- 变更：本 commit——`src-tauri/src/profiles.rs`（创建段：转发链 spawn 封装 +
  前置校验 + 结果分类 + 4 条纯函数测试）；`updates.rs`（`find_pnpm` 转
  pub(crate) 共用）；`ipc.rs` / `lib.rs` / `capabilities/default.json`（IPC 三处
  同步，`create_profile` 为**首个异步命令**：探测 + 转发链全在 spawn_blocking，
  避免同步命令冻结主线程）；`AGENTS.md` §7；ADR-0009 §5；ledger 复现点 7。
  能力：spawn `dsh plugin --profile <名> add @deepseek-ai/dsh-base` 半官方路径
  创建 profile（三件套由 dsh initProfile 写出，壳零写入）；重名拒绝 + 半初始化
  放行重试（重跑 add 幂等，ADR §4）；pnpm 防御检测缺失即拒 spawn（可行动文案，
  补齐归后续刀）。
- 影响：**触宪法级**——AGENTS §7 IPC 命令登记表新增 `create_profile`，仅周知。
  范围声明：定位仅系统探测（离线档/未装系统 dsh 用户暂不可创建，报可行动错误）；
  复制/重命名/删除/默认持久化/WSL 客体内 profile/pnpm 补齐均归后续刀。
  实机验证（macOS，DSH_HOME=临时目录零污染）：init 先行 / pnpm 经注入 PATH 可
  定位 / 顺带实测 pnpm 网络失败模式（镜像 ECONNRESET -> 已创建未装中间态
  exit 1，分类文案与单测 fixture 逐字吻合）；成功路径 reconcile 沿用 Spike A
  §3.2 同机同版本结论；Windows 转发链（shell: win32 分支）仍为遗留。
- 凭据：`cargo test` 112 绿（108 + 新增 4）；`gate_tests` 三处同步一致性绿；
  `cargo fmt --check` / `clippy -D warnings` 全过；diff 已逐行人肉复核。

### 2026-08-28 完成通知 · 4.3 Profile 管理器第二刀（只读能力）—— guan（AI 协作）

- 变更：本 commit——新增 `src-tauri/src/profiles.rs`；`ipc.rs` / `lib.rs` /
  `capabilities/default.json`（IPC 三处同步）；`AGENTS.md` §7；`docs/adr/0009`
  §5 勾选；`docs/contracts/dsh-behavior-ledger.md`。能力：profile 非法名校验
  （与 dsh `resolveProfileDir` @ 318 逐字一致）、profiles 扫描器（已物化 +
  未物化内置模板名两态合并；排除 `profiles/node_modules` 符号链接农场）、单
  profile 详情（package.json 关键字段 + `cordis.patch.yml` 原文，YAML 不解析——
  serde_yaml 已弃维，依赖选型推迟到启停插件刀）；新 IPC 命令
  `list_profiles` / `get_profile_detail`。纯读：零写入、零 dsh 子进程、零网络。
- 影响：**触宪法级**——AGENTS §7 IPC 命令登记表新增两条（新命令流程规定动作），
  仅周知。范围声明：管理器仅覆盖壳侧本地 home（`user_dsh_home()`）；WSL 客体内
  profile、创建/复制/重命名/删除、`--dump-config` 详情、pnpm 补齐均归后续刀。
  ledger 复现点 6/8 已按行号勘误口径（318/323/353，弃 11826/13418）入册。
- 凭据：`cargo test` 108 绿（基线 98 + 新增 10：校验正反例 / 扫描两态与农场
  排除 / 详情路径遍历拒绝等）；`gate_tests` 验证三处同步一致性；
  `cargo fmt --check` / `clippy -D warnings` 全过；diff 已逐行人肉复核。

### 2026-08-28 宪法级改动 · AGENTS.md 减法：248 → 178 行，删微观管理留边界 —— guan

- 变更：`AGENTS.md` 全文重写（commit `876dbcb` 之后）——删除三类内容：① 通用工程
  常识（跨平台 #[cfg] 分叉、阻塞主线程禁令、组件文件命名、Zustand 选择器细则、
  mock 策略细节等——AI 工程常识默认做对，写清单反而暗示「除此以外随便」）；
  ② 可推导明细（模块职责逐个注释、依赖版本行、构建发布行、§7 专项裁定长文——
  §9 索引已有等价一行结论，消双源）；③ 过度规定（错误页样式、stdout 日志技法——
  归还代码注释与 ADR）。保留全部边界与真坑：两红线（含台账指针）、dsh 文件系统
  不变量、唯一网络面、IPC 三处同步、事件总线模块加载期竞态坑、child_cmd、
  tauri-cli 同代、zip pinned、必测四场景、AI 交互七条、持久化例外册、§11 元规则。
- 影响：**宪法级**——规范哲学转向「只写边界 + 真坑 + 为什么，不写怎么做」，给
  AI 留发挥空间；被删内容均有归宿（代码 / ADR / roadmap / 全局 AI 配置）或属
  可推导。此前批准的全部提交于 `876dbcb`，本笔减法独立可回溯。
- 凭据：纯文档改动，不触运行时；全文 178 行 ≤ 250 预算、单节最大 31 行（§4）；
  真坑与边界关键词 grep 抽查全数在册；修改未提交，待确认后提交。

### 2026-08-28 宪法级改动 · 规则方向性审核落地：复现台账 + 白名单闭环 + 规则触发器 —— guan

- 变更：① 新建 `docs/contracts/dsh-behavior-ledger.md`——dsh 行为复现台账（已落地
  5 项 / 计划 3 项，基线 v0.1.1-rc.2），堵「复现点随 dsh 升级静默漂移、CI 无法发现」
  缺口，dsh 升级 = 逐条复核触发器；`AGENTS.md` §0 红线 1 挂台账指针。② §4.4 白名单
  治理闭环——「未来数据层」改「数据获取层（取数·缓存·同步）」判据，新增依赖 =
  先回写清单（唯一权威）再广播。③ 两条规则补再评估触发器——RTL/jsdom 禁令挂
  「Profile 管理器 UI 落地时重评」、无 dev-dependencies 加「确需专用依赖先 ADR」口子。
  ④ roadmap Next 加工程前置——IPC 三处同步自动化自检 spike（4.3 开工前把人肉纪律
  升级为机器闸门）。
- 影响：**宪法级**（AGENTS.md）+ 新增台账文档。dsh 升级流程多一步「复核台账」；
  前端新增依赖流程变「回写清单 + 广播」；4.3 开工前多一个 spike 闸门。纯文档，
  不触运行时。
- 凭据：复现点逐条 grep 锚定代码实际位置（shell.rs / resolve.rs / updates.rs /
  executor.rs）；AGENTS 全文 248 行 ≤ 250 预算、§4 恰 40 行；修改未提交。

### 2026-08-28 宪法级改动 · 跨文档定位统一 + AGENTS 一致性修正 —— guan

- 变更：三份文档定位统一为「dsh 的桌面管理面板」（2026-08-27 重定义的收尾）——
  roadmap §1 开头「桌面终端 / 极小的壳」改写、README 开头定位段改写（补管理能力句）、
  contract.md 宿主解析节加历史定位注（ADR-0005 语义不变）；AGENTS.md 五处一致性
  修正——§7 例外册补登记 `app:update` 事件（updater 回推，仅 main/about）、§2 docs
  清单补 frontend-migration 与 spikes/、§5 测试命令收敛为 §1 指针（消双源）、§0
  工程准则补 §6 指针、§6 持久化例外改「例外册登记制」（与 roadmap 硬约束 4 对齐，
  第二例外落地前先登记字段名）。
- 影响：**宪法级**（AGENTS.md + contract.md）。定位口径此后以 AGENTS §0 为唯一
  权威；settings.json 第二字段落地前须先在 §6 登记。纯文档，不触运行时。
- 凭据：grep 全仓「桌面终端」仅剩 ADR 历史引用（刻意保留）；本笔与上一条（制度
  建设）同批未提交，待确认后按 CONTRIBUTING 提交。

### 2026-08-28 宪法级改动 · AGENTS.md 首轮回收：394 → 244 行，回归 §11 预算 —— guan

- 变更：`AGENTS.md` 全文按 §11 回收——§2 目录树（58→17 行）：`ls` 可推导的文件级
  明细删除，只留职责与陷阱（勿动 / 勿手改 / 永不入库）；§7 例外册（50→19）：IPC 命令
  收敛为登记行，专项裁定收敛为「一行一裁定 + ADR 编号」，推理细节以 §9 索引 + ADR
  为唯一源（消除双源）；§1 技术栈（35→18）：依赖版本明细还 Cargo.toml / package.json，
  只留坑与裁定锚点；§4 代码规范（69→40）：正反例去重合并；§5 测试明细（25→13）还
  roadmap；§0 / §8 / §9 / §10 措辞收紧。只删可推导明细与已有归宿的长文，裁定结论全保留。
- 影响：**宪法级**——纯回收、无新增裁定；所有删除项均有既有归宿（ADR-0001~0009 /
  roadmap 4.1/4.2 / 代码现状）。协作者若发现某被删细节在 ADR / roadmap 也找不到，
  频道提出即可，git 历史可回溯旧版全文。本笔与上一条（§11 元规则）同批未提交，
  建议合为一笔提交（hunk 交叉无法拆分）。
- 凭据：纯文档改动，不触运行时；全文 244 行 ≤ 250 预算、单节最大 40 行（§4），
  diff 394→244（−150）。另发现 README.md「结构」节仍是 React 迁移前旧版（ui/ 静态页），
  属已知失真，另行开工修正，本笔不动。

### 2026-08-28 宪法级改动 · AGENTS.md 新增 §11 写入边界（元规则）—— guan

- 变更：`AGENTS.md`——① 文件头挂一行指针（「最小必要集，不是知识库」）；② 新增
  §11 元规则：准入判据四条（高频 / 违约即事故 / 不可推导 / 无家可归，核心判据 =
  「没有这条 AI 会做错吗」）、排除清单七类（模块技法→注释、决策推理→ADR、流程
  细则→CONTRIBUTING、契约细节→contract.md、计划指标→roadmap、操作手册→docs
  专项、通知→broadcasts）、形态规则（结论一句 + 日期 + 指针；升格 ADR 后原条目
  必须回收，禁双源）、预算与回收（全文 ≤ 250 行 / 单节 ≤ 40 行）。
- 影响：**宪法级**——此后向 AGENTS.md 写入任何条目须先过 §11.1 判据，不合者
  review 可依据本节驳回；知会落档、broadcasts 登记范围不变。存量超支（~354 行，
  §2 目录树 / §7 例外册为主）按新规则另行开工一笔回收。
- 凭据：纯文档改动，不触运行时；本笔仅新增元规则、不回收存量（一次一意图）；
  修改未提交，待确认后按 CONTRIBUTING 提交。

### 2026-08-27 宪法级改动 · 项目边界重定义：dsh 桌面管理面板 + 两红线 —— guan

- 变更：`AGENTS.md`（§0 定位重写为「dsh 的桌面管理面板」/ §1 技术栈表数据库行 /
  §4.2 禁库改「需 ADR 评估」/ §6 存储与生命周期重写）、`docs/roadmap.md`（硬约束
  2/4 重写）、`docs/adr/0008`（壳保持薄加注「仅约束运行时」）、`docs/adr/0005`
  （转发链 vs 全局安装区分注）、`docs/adr/0006`（管理功能网络面不属唯一网络面注）、
  `docs/executor.md`（defaultMode 例外表述同步）、`docs/adr/0009`（§2 重写为
  3 红线 + 工程准则）。
- 影响：**宪法级**——① 项目定位从「通用产品壳」改为「dsh 的桌面管理面板」；
  ② 红线定为两条：不修改 dsh 源码 + 安装包不内置依赖（优先宿主检测、缺失时
  实时下载，含 pnpm 自动补齐扩展）；③ 「壳保持薄 / 无状态库」降为**仅约束运行时**，
  管理功能按优秀软件工程设计（可引入数据库 / 持久化）；④ 历史 ADR 保持原状仅加注。
  协作者注意：管理功能（4.3+）不再受「无状态库」限制，但 dsh 文件系统不
  变量（0600 凭据 / 三键 / 农场只读）继续必须维护。
- 凭据：纯文档改动，不触运行时；diff 已逐文件核对（AGENTS §0/§1/§4.2/§6 +
  roadmap 硬约束 + 4 个 ADR 加注 + executor.md 同步）。修改未提交，待确认后按
  CONTRIBUTING 提交。

### 2026-08-27 快车道直推 · 完成通知：Now 阶段收口（4.1 工程化基线 + 4.2 updater 测试）—— guan

- 变更：六连提交——`bd94596` fix(clippy) 全部 17 处警告清零（关键：
  shell.rs `floor_char_boundary` 击穿 rust-version=1.77.2 的 MSRV 承诺，
  CI 全用最新 stable 故未暴露）；`dd521d1` style 全仓 fmt 归一
  （9 文件 168+/147- 纯机械，与 clippy 修复分仓提交）+ 落地仅锁 edition 的
  `rustfmt.toml`；`9029ebe` chore(rust)；`10e0957` ci 三平台
  fmt --check / clippy -D warnings 闸门 + ubuntu coverage job
  （cargo-llvm-cov 出 lcov，先出数不定阈值）；`5417987` test(updater)
  六条纯函数测试；docs 提交（见下）。
- **宪法级改动（本次知会）**：① AGENTS §1 Rust 行——移除 `rust-version`
  基线，**Rust 工具链跟随最新 stable**（2026-08-27 维护者裁定：不设 MSRV；
  上限纪律不变：CI @stable 自动跟新）；② AGENTS §1 Lint/Format 段改写为
  已建基线状态；③ AGENTS §5 updater 待补条目改为已覆盖表述；
  ④ AGENTS **新增 §8 第 7 条「驳回不合理的规则」**——AI 判定规范与现实
  冲突/自相矛盾/失效时应停手提请驳回与修订（举证义务在提请方），
  不得以变形实现绕行；顺从≠忠诚，变形合规比违规更危险。
- 影响：CI 首跑新闸门有红的风险已用本地预演对冲（本机 1.98 三道全绿）；
  Next 阶段（4.3 Profile 管理器）进入条件满足，开工前先做两个前置 spike。
- 凭据：本地 rustc 1.98.0 下 fmt --check / clippy --all-targets -D warnings /
  cargo test 95 绿；fmt 与 clippy 两类 diff 分仓提交均经人肉复核。

### 2026-08-27 发版事项 · v0.5.1 三平台验收通过，冻结期解除 —— guan

- 确认：tag run `33049383090` 三平台 job 与 release job 全部 success；Release
  资产 14 个齐全（dmg/exe/msi/AppImage/deb/rpm + 签名 + latest.json）；
  下载 macOS `.app.tar.gz` 实拆——`Info.plist` 版本 0.5.1，主二进制内嵌前端
  bundle 五个事件名（app:update / boot:step / boot:progress / boot:update /
  boot:error）grep 全中（本次缺陷的产物级判据）。上游 dsh 会话工作正常。
- 影响：**v0.5.1 发布完成，冻结期自此解除**——master 恢复正常合流
  （改动仍按 CONTRIBUTING 占用声明纪律）；下一步按路线图 Now 阶段推进。
  已装 0.5.0 的环境因自更新面板同受缺陷影响，需手动换装 0.5.1。
- 凭据：`gh api .../runs/33049383090/jobs` 全 success；本条即对上两条
  「处置见下一条广播」预告的闭环。

### 2026-08-27 发版事项 · v0.5.0 缺陷确认 → 重切 v0.5.1 热修 —— guan

- 变更：fix `f3cef30`（事件总线 import 锚点，见下条补记）+ updater 观测日志
  （run_check 入口与 set_state 每次推进记 tracing）+ 全仓版本升 `0.5.1`
  （tauri.conf.json / Cargo.toml / Cargo.lock / frontend package.json+lock），
  tag `v0.5.1` 当日推送。
- 裁定（为何不沿用上次 force 迁移 tag 的做法）：v0.5.0 三平台产物**已发布且含
  缺陷**——事件监听缺失不止影响关于页：启动时间线 / 错误卡 / 下载进度同链路
  全部不刷新，冷启动表现为启动页冻结后硬跳工作台、失败时错误卡不渲染。
  上次 force 迁移的前提是「原 run 无任何产物」；本次 Release 已存在、可能已有
  下载，force 迁移会留下同名异物的资产，违反可追溯原则。按语义化版本重切
  v0.5.1。注意：**0.5.0 客户端的自更新面板恰好也受此缺陷影响**（自动检查在
  Rust 侧正常执行，但 UI 不回显），已装用户需手动换装 0.5.1。
- 影响：冻结期继续（master 只收 fix）；CI 三平台验收通过前不宣布发布完成。
- 凭据：cargo test 89 绿 + 前端四道门禁绿；实机 AX 观察到修复后徽章
  「检测中 → 最新」翻转（自动首查全链路），手动点击路径同链路 +
  新增日志可事后定位；版本 diff 六文件已人肉复核。

### 2026-08-27 补记 · v0.5.0 实机缺陷：事件总线未进 bundle（关于页检查更新无反应） —— guan

- 变更：fix 待提交——`frontend/src/main.tsx` 增加 `import "./lib/events"` 副作用
  锚点；`docs/frontend-migration.md` §9 新增「事件总线」产物级回归清单条目。
- 根因：`lib/events.ts` 靠模块加载期 `initEventBus()` 自装配（宪法 §4.3 裁定），
  但全仓没有任何运行时 import 它——Vite 树摇将其整体排除出 bundle（v0.5.0
  dist 中 `app:update` 出现 0 次，实锤）。所有窗口的 boot:*/app:update 监听均未
  注册；关于页显示的「已是最新」全部来自进入时的播种 invoke，恰好掩盖断链。
  纯逻辑单测（34 例全绿）测不出「装配丢失」这类集成缺失。
- 影响：**v0.5.0 三平台产物若已出包则携带此 bug**——关于页更新状态机不再实时
  推进、启动时间线/错误卡/下载进度不刷新。处置与是否重打 tag 见下一条广播。
- 凭据：前端四道门禁绿 + 新产物 grep 五事件名各 ≥1；修后复验结论随附。

### 2026-08-27 补记 · v0.5.0 发布中断修复（Ubuntu CI 失败 → 重发） —— guan

- 变更：`915657f`（冻结期 fix）—— beforeBuildCommand 钩子 cwd 显式化
  （tauri.conf.json ScriptWithOptions `cwd="../frontend"`），build.yml/AGENTS
  注释同步；`v0.5.0` tag 已 force 迁移指向该修复（原 tag run 全失败、无任何
  产物/Release，无污染可追溯亏损）。
- 根因（CI 实证两连修）：① tauri-cli「自动发现含 package.json 目录」深度遍历
  在 Linux ext4 目录序下可能先命中 `node-map/` → npm ci 找不到 lockfile（本地
  APFS 碰巧命中 frontend/，阶段 A 的验证结论被事实击穿）；② 显式 cwd 相对基准
  实为 **src-tauri**（build.rs `set_current_dir(dirs.tauri)`），首修用的
  `frontend/` 本地复现 No such file 后改为 `../frontend` 复测通过。
- 影响：仅发布链路，不触运行时行为——macOS/Windows 两 job 原 run 继续走完
  但其构建内容同构（hook 修复对三平台同效），三平台产物仍以重发 run 为准。
  经验已落档：**tauri 钩子 cwd 永远相对 src-tauri 且必须显式**，勿复信自动发现。

### 2026-08-27 发版事项 · v0.5.0 发布开始 —— guan

- 变更：`chore: 版本 0.5.0（…整批提交）`——tauri.conf.json / Cargo.toml /
  Cargo.lock / frontend package.json+(lock) 同步升版；roadmap 适用版本标 v0.5.0。
  tag `v0.5.0` 由本档案登记当日推送，CI 三平台矩阵 + Release 聚合
  （notes 由 GitHub 自动生成）。
- 影响：**冻结期开始**（CONTRIBUTING §8）——Release notes 发出至三平台产物
  验收通过期间，master 只收 fix 不收 feat。本版内容：前端自静态 HTML 全量迁移
  Vite+React+TS+Tailwind v4+shadcn/ui（ADR-0008 全流程，span commit
  aab68c1→本次），壳行为与 12 IPC 命令零变更；Move/关于/启动/选择器四页
  组件化；宪法同步修订（AGENTS §1/§2/§4.2/§4.3/§4.4/§5/§7）。
- 凭据：frontend gate 34/34；cargo test 89 passed；本机 release 构建三产物齐；
  已知遗留——Windows/Mode 页实机走查未做（广播 2026-08-27 阶段 C 条目）；
  fmt/clippy 基线待专项（阶段 E 条目）。

### 2026-08-27 快车道直推 · 完成通知：前端迁移阶段 E 落地（d003905 / 1868411） —— guan

- 变更：**宪法级（1868411）**——AGENTS §1/§2/§3/§4.2/§4.3/新增 §4.4/§5/§7、
  docs/roadmap.md（硬约束 2 与不做清单）、docs/CONTRIBUTING.md（路径行）、
  .github/workflows/build.yml（Frontend gates 步骤）；**非宪法（d003905）**——
  Vitest 34 用例（format/bootProgress/bootStep/updatePhase）、`ui/` 目录删除
  （dsh-logo.svg 迁至仓库根 `assets/`）、全仓悬空引用清理。
- 影响：① **宪法已生效**——「禁止引入任何前端构建链」修订为「前端框架仅限
  React 生态（§1/§4.2/§4.4 白名单）」；② 开发者须知——本地构建/调试请从
  **仓库根**调用 `cargo tauri dev/build`（钩子 cwd 发现逻辑），前端开发需
  node ≥20（`cd frontend && npm ci`）；③ **fmt/clippy 基线评估结论**：存量
  35 文件未归一 + clippy 9 警告，需专项 chore 落地（遵守「不引入全仓格式化
  diff」红线），本轮 CI 只接前端四道闸门，roadmap §4.1 [待补充] 保持；④ 迁移
  完成发布契——master 自此无 `ui/`，release 产物壳页面全 React。
- 凭据：frontend typecheck/lint/vitest 全绿（34/34）；`cargo test` 89 passed；
  `cargo tauri build --no-sign` 出齐三产物；diff 逐行人肉复核（lib.rs 仅两处
  注释；fmt 越界改动已回退——本次 session 自身纪律记录）。

### 2026-08-27 占用声明 · 前端迁移阶段 E 开工（宪法级变更预告） —— guan

- 变更：占用 `AGENTS.md`（§1/§2/§3/§4.2/§4.3+新增§4.4/§5/§7）、
  `docs/roadmap.md`（硬约束 2 与不做清单）、`.github/workflows/build.yml`
  （node 质量闸门 + fmt/clippy 评估）、`ui/`（删除）、`frontend/`（Vitest 测试）。
  依据：ADR-0008 行动项 + docs/frontend-migration.md §6/§7/§10 阶段 E 清单。
- 影响：**宪法级预告**——AGENTS §4.2「禁止引入任何前端构建链」将修订为
  「前端框架仅限 React 生态（Vite+React+TS+Tailwind+shadcn/ui）」，§4.3 全面
  重写为 React 组件规范，新增 §4.4 三条红线（依赖白名单 / 前端禁止网络请求 /
  跨窗口真相源）。`ui/` 目录删除、`dsh-logo.svg` 迁至仓库根 `assets/`。
  执行顺序：测试 → 删 ui → 宪法/CI（单独 commit）→ 全量验证 → 完成通知。
- 凭据：阶段 A-D 均已通过闸门与实机验证（见前四条约）；本预告为宪法修改
  前置知会，修改范围与方案 §7 清单一一对应。

### 2026-08-27 快车道直推 · 完成通知：前端迁移阶段 D 落地（1008cd6） —— guan

- 变更：`pages/BootIndex.tsx` 整页、`components/boot/{BootStep,BootTimeline}.tsx`
  新增、ErrorCard diag 形态、`lib/events.ts` 总线装配时机、`pages/BootMode.tsx`
  握手时序、方案文档 §3.3（总线裁定同步）。**四页至此全部迁入 React**。
- 影响：一处时序裁定周知——事件总线要求在**页面任何播种 invoke 之前**注册；
  实现为模块加载期装配（详情见方案 §3.3 与 lib/events.ts 注释）。stage B 中
  BootMode「先 invoke 再导航」的写法本轮已更正为旧握手（携参回启动页由
  BootIndex 落地）。
- 凭据：typecheck/lint/build 全绿；release 冷启动事件链全通（日志钉板）；
  BootIndex 静态帧经 dev 预览核对；下载条/错误卡的实机触发依赖特定失败路径，
  逐行对照旧码迁移（已复核）。阶段 E 前 master 中间态照旧：壳页面功能已全，
  待删 `ui/` 与宪法修订。

### 2026-08-27 快车道直推 · 完成通知：前端迁移阶段 C 落地（d780855） —— guan

- 变更：`frontend/src/pages/{BootMode,BootSelector}.tsx` 整页、
  `components/boot/`（DownloadProgress/ErrorCard/VersionChip/PulseBar 落地，
  阶段 D 复用）、`hooks/usePlatform.ts`、文案层 mode/selector 扩展。
- 影响：仅周知 + 一项待办迁移——**Mode 页（运行环境选择）是 Windows-only
  表面**，React 版实机目视验证待 Windows 环境（非 Windows 访问按裁定防御性
  回启动页，本机已验证该兜底路径编译正确）。其余同前：master 中间态渐次回填。
- 凭据：typecheck/lint/build 全绿；release 实机 fixture 双工作台触发选择器
  （双卡片 + DEFAULT/CUSTOM 徽标 + 版本芯片渲染正确，截图存档）；点击官方卡
  打通 `choose_profile` IPC 全链路（shell.log 钉板：dsh 启动 profile=web →
  1.3s 就绪 → 导航工作台）。测试进程与 fixture 均已清理。

### 2026-08-27 快车道直推 · 完成通知：前端迁移阶段 B 落地（5ce1296） —— guan

- 变更：`frontend/src/components/about/`（新建四组件）、`pages/About.tsx` 整页、
  `content/zh-CN.ts` 文案扩展、`index.css` token 改名、`App.tsx` 预览钩子、
  方案文档 §3.5 同步。关于窗口自旧 `ui/about.html` 完整迁入 React。
- 影响：仅周知 + 一处 token 命名裁定——`--color-muted/--color-accent` 与
  shadcn 语义层重名导致工具类被覆盖（实机截图发现文字近白），域 token 更名
  **dim / brand**；后续页面（阶段 C/D）直接用新名。master 中间态照旧：壳骨架页
  渐次回填，阶段 E 收口删 `ui/`。
- 凭据：typecheck/lint/build 全绿（gzip 137KB，<500KB 复审线）；release 实机
  截图验证整链路（自动首查→upToDate、三维度真实数据、工作台地址注入）；
  配色修复以构建产物 CSS 钉板（`.text-dim{color:var(--color-dim)}` 解析唯一，
  内联映射机制反向解释原 bug）；本地 release 产物已重建包含修复。

### 2026-08-27 补记 · 阶段 A 目视验证完成（步骤 20/21 清账） —— guan

- 变更：`docs/frontend-migration.md` §3.1 一处标注（钉板句从「待复核」改为
  「已复核通过」）。无代码改动。
- 影响：仅周知——release 产物内以临时第二 webUi profile 触发
  `/selector` 直达，**SPA fallback 实机命中**（主窗口渲染 BootSelector 页，
  步骤 21 钉板完成）；about 窗口经菜单打开，label 路由渲染 About 页（步骤 20）。
  临时 fixture（仅含 package.json 的 `~/.dsh/profiles/probe-dual-webui`）
  已删除，真实环境零残留。
- 凭据：实机截图两帧（主窗口 /selector 内容 + 关于窗口内容）；进程已退出。

### 2026-08-27 快车道直推 · 完成通知：前端迁移阶段 A 落地（a9c1656） —— guan

- 变更：`frontend/`（新建 41 文件）、`src-tauri/tauri.conf.json`、
  `src-tauri/src/lib.rs`、`.github/workflows/build.yml`。Vite+React+TS+Tailwind v4
  + shadcn/ui 七组件脚手架；窗口 label 路由；四页占位骨架；Rust 主/about 窗口
  改载 SPA 根、selector/index 跳转改 pathname；platform_script 扩 `{os,wsl}`。
- 影响：**master 中间态**——自本 commit 起 release 产物壳页面为 React 骨架
  （功能回填顺序 B About → C Mode/Selector → D Index，阶段 E 收口删 `ui/`
  并改宪法）。与占用声明的偏差仅一处：CI node 步骤自阶段 E 提前（frontendDist
  切换后 `cargo tauri build` 硬依赖 npm 构建，不提前则打 tag 即挂）；
  Build installers 工作目录随之移到仓库根。开发者注意：本地构建/调试请从
  **仓库根**调用 `cargo tauri dev/build`（钩子 cwd 发现逻辑要求，见 build.yml 注释）。
- 凭据：typecheck/lint/vite build 全绿（JS gzip 79.9KB）；`cargo test` 89 passed；
  本机 `cargo tauri build --no-sign` 出 dmg/.app/updater tar 三产物；release 实机
  启动 Rust 全链路通过入工作台。步骤 20/21 的目视项（骨架观感、release 内
  /selector 直达 SPA fallback 实测）待人工复核。

### 2026-08-27 占用声明 · 前端迁移阶段 A 开工（ADR-0008 实施开始） —— guan

- 变更：占用 `frontend/`（新建）、`tauri.conf.json`（frontendDist 切换）、
  `.github/workflows/build.yml`（node 步骤合批）、`.gitignore`；阶段 E 收口时改动
  `AGENTS.md`（§1/§2/§3/§4/§5/§7）与 `docs/roadmap.md`（硬约束 2 与不做清单）。
  实施依据：ADR-0008 + `docs/frontend-migration.md`（commit aab68c1、f21df95）。
- 影响：**宪法级变更预告**——AGENTS §4.2「禁止引入前端构建链」将在阶段 E 按既定
  方案修订为「Vite + React 定向许可」；执行顺序 A 脚手架 → B About → C Mode/Selector
  → D Index → E 测试与治理收口，各阶段完成即在频道知会。阶段 A–D 仅新增文件，
  不动现有 `ui/` 与 Rust 行为；master 保持随时可构建。他人如需动上述文件请先在
  频道协调。
- 凭据：Tailwind v4 + shadcn/ui 兼容性 spike 已通过（2026-08-27 临时目录验证：
  shadcn init 显式识别 v4、七组件生成、strict TS 下 vite build 成功 JS gzip 73KB；
  结论与三注意点已回写方案 §1 并勾销 ADR-0008 行动项）。本条与 spike 回写同 commit。

### 2026-08-27 快车道直推 · 完成通知：roadmap 对照 dsh 源码核查修订 —— guan

- 变更：`docs/roadmap.md`（+43/−27；本档案与该改动同 commit 落盘）。对照
  deepseek-harness v0.1.1-rc.2 源码逐条核查路线图事实主张后修订：事实表重做
  （子包实测 227、`dsh plugin` = pnpm 原样转发、patch 按 id 逐字段赋值且 `config`
  整体替换不深合并、出厂 profile 模板仅 web/headless 且无任何 profile 管理官方命令、
  dsh 无插件安装/卸载 UI），新增「事实边界与陷阱」清单（非法名校验、profiles/node_modules
  符号链接农场、`.credentials.yaml` 三条硬约束等）；行动项修正：4.3 创建路径弃
  agent-presets 误用改为半官方 plugin-add 引导、4.4 pnpm 失败模式入错误处理并补全
  运行时状态枚举、4.2 updater 测试表述更新、编号引用与版本头（v0.4.7 起）修正。
- 影响：两项裁定仅周知——① 4.3④ 默认启动 profile 持久化到 `settings.json`，经
  维护者批准作为第二最小面例外（落地实现时同步登记 AGENTS §6 后方可合入）；
  ② Next 阶段（Profile 管理器）开工前须先做两个 spike：GUI 环境（无 shell rc）
  pnpm 经 `dsh plugin` 转发链的可用性验证、复制/重命名 profile 的引用清点。
- 凭据：纯文档改动不触运行时；事实主张均锚定 dsh 源码位置（app-boot/src/profile.ts、
  vendor/include/src/index.ts、apps/cli/src/plugin.ts 等）；残留检查干净
  （旧引用 197 包数/（3.4）/零测试 表述已全部清零）。

### 2026-08-27 建档 —— guan

- 变更：新建本档案；`AGENTS.md`（§2 目录树 / §8.5 收尾三件事 / §10 知会落档条目）
  与 `CONTRIBUTING.md` §0 各挂一处指针。
- 影响：协作者此后按上表登记知会；本次指针挂接触宪法级文件，本条即为对其的知会。
- 凭据：纯文档改动，不触运行时。

### 2026-08-27 补记 · 完成通知：删除前端顶栏「关于」入口 —— guan

- 变更：commit `8075eea`——`ui/index.html` / `ui/mode.html` / `ui/selector.html` /
  `src-tauri/src/lib.rs` / `build.rs` / `capabilities/default.json`。删除三页壳顶栏
  「关于」按钮及 `open_about` IPC 整链，与原生常驻入口重复（Windows 启动页问题报告）。
- 影响：**触宪法级**——`AGENTS.md` §7 IPC 注册表移除 `open_about`、「更新常驻入口」
  条目改写为裁定后状态；关于面板此后只能经菜单（macOS）/ 托盘（非 macOS）打开。
  仅周知，无需动作。
- 凭据：`cargo test` 全绿（89 passed）；diff −36/+6 已逐行人肉复核；本条视为对
  该次宪法修订的知会。
