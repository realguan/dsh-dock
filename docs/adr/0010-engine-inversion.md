# ADR-0010：运行时归属倒置——自有引擎包 + pnpm12 引导，探测层退役

- **日期**：2026-09-03
- **状态**：已接受（2026-09-03 维护者裁定；环境准备阶段全节点 grilling 三轮评审通过，裁定台账见 §7）
- **提出人**：guan（方向裁定）·AI 起草（调研与举证见会话档案）
- **相关方**：resolve.rs / updates.rs / executor.rs / profiles.rs / plugins.rs；contract.md；node-map；AGENTS.md §0/§6/§7
- **关联**：ADR-0004 §7（WSL 补齐链，本决策修订其 pnpm 缺口）· ADR-0005（global-bin-dir）· ADR-0006（镜像链）· ADR-0009（pnpm 硬依赖口径 2）· node-map/README.md

---

## 1. 背景与问题

环境准备层是全仓自研兼容面最大、且只增不减的一层。其根因是一个架构决策：
**「复用用户环境来执行」（system 档）**——为了借用户的 node/dsh，壳被迫回答
「用户的工具到底在哪、能不能信」，由此产生四层靠枚举外部世界维持正确性的代码：

- 登录 shell PATH 探测（resolve.rs `login_shell_path`，zsh/bash -lc + 超时）；
- 约 26 个固定目录硬编码（`fixed_path_dirs`：npm/pnpm 全局位、volta、scoop、homebrew…）；
- 版本管理器目录枚举（`fnm_nvm_bin_dirs` 排序取最新；executor.rs `guest_prep` 客体内再来一遍 glob）；
- Windows pnpm 全局布局考古（`global/3..=10` 逐主版本猜，pnpm 12 落 `global/12` 即漏——已核实）。

以及由此派生的死局与缺陷：

- **TooOld 死局**：用户全局 dsh 版本过低 → 整个应用拒绝启动（resolve.rs `resolve_launch` bail），不自动覆盖演变成「不可启动」；
- **pnpm 版本未 pin**：`npm i -g pnpm` 追最新，major 行为漂移（v10 allow-build、v11 配置键迁移、v12 弃用 env）无闸；
- **WSL 客体链三缺陷**：未装/未查 pnpm（与 ADR-0004 §7 文档「node→pnpm→dsh」不符，客体内 `dsh plugin` 必挂）；node tarball 下载无校验和（contract 要求 SHA-256，本地链有客体链没有）；依赖客体有 curl，无兜底。

维护者裁定（2026-09-03）：**自研逻辑最小化优先**——自己写的逻辑越多越容易出错；
能用成熟工具验证过的行为，就不自研。本 ADR 以此为准绳重审环境准备层。

## 2. 约束与硬指标

- **dsh_home 保持 dsh 默认**（`$DSH_HOME` / `~/.dsh`）——用户世界归属不变（维护者 2026-09-03 裁定）。引擎归壳，数据归用户，两者正交。
- 三件套不得代为初始化（红线 1 / ADR-0009 不变量）；会话目录只读等 dsh 文件系统不变量继续成立。
- 网络面唯一 `updates.rs`（§7）；WSL 客体内的网络动作沿 ADR-0004 模式发生在客体进程内。
- 插件安装/更新/删除与 profile 创建必须 pnpm 可见（dsh 硬编码 `spawnSync("pnpm")` 无回退，锚定 plugin-9h8shc4d.js @ 101）。
- 版本必须可 pin、可回滚；下载完整性校验必须存在（不能裸下载）。
- 安装包体积增幅必须记录在案；新增内置物须修订红线 2——本 ADR 即修宪载体。

## 3. 备选方案及评估

### 方案 A：引擎倒置 + 内置 pnpm12 引导 —— ✅ 提案采纳（状态：草案）

- 思路：壳自带「引擎包」，用户环境从执行来源降级为数据。
  - **引擎目录**（壳数据目录下，如 `engines/pnpm-home`）：内置 pnpm12 原生二进制
    （~17–19MB/平台；v12 起不含 node），`PNPM_HOME` 由壳注入指向引擎目录。
    node（`pnpm runtime set node <v>`）、dsh（`pnpm add -g`）、pnpm 自身——全部由
    pnpm 下载、解压、布局、激活（`nodejs_current` 指针），**壳代码只做编排**。
  - **版本治理**：node 版本仍由 node-map 定（ed25519 验签保留；SHA-256 仲裁让位
    pnpm 的 SHASUMS256——信任模型降级，如实记录于后果；SHA 字段保留不消费，为
    方案 B 回退免改版复用）；pnpm 版本 = 随壳 pin（**显式版本号**，不能跟 `latest`
    dist-tag——其仍指 v11；升级走壳发版节奏，map schema 无需升 v2）；dsh = **显式
    升级**（§7 裁定）：boot 比对最新稳定版但排除预发布（0.1.2-alpha.2 事故的预防性
    过滤，依赖完整性预检保留），boot 恒用已装版本启动，升级经用户决定。
  - **探测层退役**：`detect_system_dsh` / `fixed_path_dirs` / `fnm_nvm_bin_dirs` /
    `login_shell_path` / `global/3..=10` / guest nvm-fnm 扫描整层退役；manifest
    resolution 档序简化（contract v3），system/bundle 档语义终结。
  - **子进程环境自构**：spawn dsh 注入 PATH = 引擎 bin + 系统最小集；dsh 内部
    `spawnSync("pnpm")` 恒解析到引擎内 pin 版 pnpm。插件操作改用引擎 node/dsh
    （现状 plugins.rs 要求系统 node+dsh 的限制一并消除）。
  - **WSL**：musl 静态 pnpm 投递进客体（通道选型见行动项），替代 curl tarball
    脚本——客体 pnpm 缺口、无校验和、curl 依赖三个缺陷一次修复；guest_prep 收缩
    为「source rc + 引擎目录前置」。
- 优点：自研面最小（退役下载/解压/校验/激活 + 探测层共约千行）；单一引导器语义
  统一（node/dsh/pnpm/插件一个工具）；死局消失（引擎恒可用，用户全局 dsh 与启动
  解耦，「不动用户环境」从「不覆盖」升级为「根本不碰」）；WSL 三缺陷一并修复。
  pnpm `add -g` 为两段式安装（失败只删新目录、不破坏旧版本；staged + 原子换链，
  Windows junction 例外）——dsh 升级失败的回滚由 pnpm 设计兜底（v12.3.1 源码核实）。
- 代价/风险：红线 2 修订（pnpm 列入内置例外）；安装包 +17–19MB/平台；node 下载
  无断点续传（中断即从头再来），字节进度可解析 pnpm stdout 进度行（约 1 行/秒）
  映射 `boot:progress`；SHASUMS256 与镜像同源（弱于签名 map 的独立仲裁，缓解：
  node-map 仍定版本 + npmmirror/nodejs 双镜像）；耦合 pnpm runtime API 演进——
  pnpm12 为 Rust 重写版（发布较新，`latest` dist-tag 仍指 v11.25.0，**pin 必须用
  显式版本号**）；v11+ 起 pnpm 所装 node **不含 npm**（npm/npx/corepack 不解包）
  ——引擎内一切安装只走 pnpm，npm 缺位为既定事实；缓解：pin 随壳 + 升级测试清单
  + 方案 B 保留为回退路径。
- 对照约束：逐条满足——dsh_home 不变（§2.1）；三件套仍归 dsh 初始化；网络仍仅
  updates.rs 编排（pnpm 为其子进程，同 `npm i -g pnpm` 先例）；插件链 pnpm 恒在
  （引擎内置）；完整性校验存在（pnpm SHASUMS256，模型降级已记录）；体积与修宪
  载体即本 ADR。

### 方案 B：引擎倒置 + 自有下载器（node-first） —— ❌ 否决（保留为回退路径）

- 思路：同样倒置，但保留现有 `download_node`（node-map SHA 仲裁、Range 续传、
  字节进度），node 自带 npm 装 pnpm@pin 进引擎。
- 否决理由：违反 2026-09-03 维护者裁定（自研面最小化）——保留约 300 行自研
  下载/校验/续传代码，且需自维护 tools/node 布局与激活指针，自研面最大。
- **保留记录**：本方案的下载器代码已存在且能力更强（签名哈希独立仲裁、断点续传、
  字节进度，依赖面仅 node 发行包格式——十年未变）。若 §6 复审条件触发
  （pnpm runtime API 再度破坏性变化），从 git 历史恢复本方案成本最低。

### 方案 C：维持现状（system→bundle→download + 探测层），点状修补 —— ❌ 否决

- 思路：仅做 P1 类修补（TooOld 不再 bail、客体补装 pnpm、客体加校验和）。
- 否决理由：四层枚举面的增长驱动力是用户环境多样性，修补不改变「靠枚举外部世界
  维持正确性」的欠账结构；每出新版本管理器/新 pnpm major 都要再加一支适配。

## 4. 最终决策

环境准备层按「引擎倒置」重构：壳内置 pnpm12 原生二进制作为唯一引导器，node / dsh /
pnpm 的下载、布局、激活全部委托 pnpm（`PNPM_HOME` 注入指向壳引擎目录
`<数据目录>/engines/`，其 `bin/` 同入子进程 PATH）；node 版本由 node-map 定、
pnpm 版本随壳显式 pin。升级哲学（node 与 dsh 统一）：**没有任何升级是静默的，也
没有任何升级能阻塞启动**——更新入口提示 → 用户决定 → 标记 → 下次启动生效，拒绝
无硬惩罚。registry 不可达时用已装引擎直接启动：**首启必须联网，之后离线可启动**。
探测层与自有下载器退役；dsh_home 保持 `~/.dsh` 默认不动；WSL 客体同口径投递
musl pnpm（随 Windows 包内置，用户零安装），客体 pnpm 属壳资产、下载源由壳注入
镜像链。裁定准绳：自研逻辑最小化（2026-09-03 维护者裁定）。

## 5. 后果与后续行动项

### 正面后果
- 自研面收缩约千行（下载器 + 探测层退役）；boot 正确性不再依赖枚举外部世界。
- TooOld 死局消失；用户全局 dsh 与应用启动彻底解耦。
- WSL 客体三缺陷（pnpm 缺失、无校验和、curl 依赖）一次修复。
- 版本节奏清晰：node 走 node-map（带外）、pnpm 随壳发版、dsh 追稳定版。

### 负面后果 / 新增债务
- 红线 2 修订：pnpm 列入内置例外（修宪动作随本 ADR 落地）。
- 安装包 +17–19MB/平台。
- node 下载体验降级：无字节进度、无断点续传（慢网络首启体验回退）。
- 下载信任模型降级：SHASUMS256 与镜像同源（缓解：node-map 仍独立定版本 + 双镜像）。
- 新耦合面：pnpm runtime API 的 major 演进（缓解：pin 随壳 + 升级清单 + 方案 B 回退预案）。

### 行动项
- [x] **Spike（先于一切实现）——①②③④ 全部结案（2026-09-04，实证见
  `docs/spikes/0003-pnpm12-engine-bootstrap.md`）**：
  ① ✅ 注入通道核实（v12.3.1 源码 + macOS 实机）：node 镜像 env =
  `PNPM_CONFIG_NODE_DOWNLOAD_MIRRORS`，值为 JSON 对象、**键 = 发布通道
  （release/nightly/rc/test/v8-canary）**，缺键**静默回退**默认源；`pnpm-workspace.yaml`
  / 全局 `config.yaml` 亦可；`.npmrc` 与 `--config.` 旗标静默无效；`--registry=` 全命令
  有效。决定性证据：本地 404 服务器收到 `index.json` 与 `v<ver>/SHASUMS256.txt` 请求
  ——SHASUMS256 强制校验且与镜像同源。✅ 三平台 CI 复验全绿（2026-09-04，spike 0003 §2.7）。
  ② ✅ macOS 非 TTY 实机（GUI 子进程同态）：runtime set exit 0；字节进度行可解析映射
  `boot:progress`；激活需 `pnpm shim add node`（PNPM_HOME/bin 硬链）；npm/npx/
  corepack 缺位实证；**单目录引擎布局成立**（PNPM_HOME 兼作 runtime 项目，bin/global/
  node_modules/package.json 共存）；引擎链全绿（镜像装 node → add -g dsh → 引擎 node
  执行 dsh）。✅ 三平台 CI 复验全绿（2026-09-04，spike 0003 §2.7）。
  ③ ✅ macOS 签名包内 resources 二进制执行许可（2026-09-04 本机，Developer ID +
  hardened runtime，spike 0003 §2.6）：seal 覆盖 Resources 二进制、包内与 quarantine
  模拟下均可执行；exec 级公证行为需真实发布产物复核——挂发版验收清单；
  ④ ✅ WSL 客体投递通道选型（2026-09-04 CI 实测，spike 0003 §2.7）：`\\wsl$` 拷贝与
  wsl.exe stdin base64 两通道 32MB 探针均完整（0.3s / 0.4s）——**选 `\\wsl$` 为主**，
  base64 留兜底。
  ⚠ 实机新发现两处，均已裁定（2026-09-04）：pnpm 二进制解包 32MB → §6「安装包内
  压缩存储」；musl node 下载源硬编码 unofficial-builds、不可镜像注入 → **客体只支持
  glibc 发行版**（Alpine/musl 不在支持范围），不再挂账（spike 0003 §4）。
- [x] P1（2026-09-03 落地，独立先行）：TooOld 不再 bail（旧实现直接终止启动 = 死局）
  ——记录后落后续档（bundle/download）继续 boot；档序耗尽才带版本信息报错。复现
  测试 ×2：`resolve.rs::tests::resolve_too_old_system_falls_to_next_tier` /
  `resolve_too_old_exhausted_reports_actionable_error`。
- [x] P2（2026-09-03 落地）：pnpm 补齐落 `<数据目录>/engines/npm`（`--prefix` +
  `NPM_CONFIG_PREFIX` 双保险，替代用户全局；bundle 档只读 resources 亦因此可补齐）+
  显式 pin `pnpm@12.3.1`（spike 0003 §2.1）；子进程环境自构第一步 = `dsh_child_path`
  （引擎 bin → node bin → 用户 PATH），ensure_pnpm 可见性检查与 shell::spawn_dsh
  同源，dsh 内部 spawnSync("pnpm") 恒可达。完整「PATH = 引擎 bin + 系统最小集」
  （剥离用户 PATH）随 P3 引擎模式落地。
- [ ] P3（本 ADR 主体）：引擎编排模块 ✓ → boot 接线 ✓ → contract v3 ✓ →
  检测层退役（待）→ AGENTS §0/§6/§7 同步（待）→ `docs/broadcasts.md` 落档 ✓。
  （**engines.rs 编排模块 + updates 引导入口 2026-09-04 先行落地**——布局/就绪
  判定/镜像注入/进度解析/四步幂等引导纯新增零接线，190 测试全绿；boot 接线、
  探测层退役、contract v3 待续。）
  （**插件操作改引擎档 2026-09-04 落地**——`plugins.rs` 工具链解析引擎优先
  （engines/bin node+dsh 双全才选引擎，半就绪不混搭）、系统探测回退；dsh 启动器
  直接执行（pnpm 全局 shim，child_cmd 吸收 .cmd 差异），不再深挖全局树取 bin.js；
  顺带闭合系统档隐患：spawn PATH 与 ensure_pnpm 可见性基准同源 `dsh_child_path`。
  创建链暂留系统档（`run_dsh_plugin` 保持原行为成为薄封装），随 P3-b 一并切。）
  （**boot 接线 + contract v3 2026-09-04 落地（P3-b）**——manifest `MANIFEST_FORMAT=3`
  （TierKind::Engine；加载规范化 tiers∈{[Engine],[Bundle]}，v1/v2 兼容迁移）、
  resolve_launch 引擎档臂（ensure_engine_bootstrapped → 启动器形态 LaunchSpec）、
  bootstrap 版本惰性闭包（node 已装解析失败→离线降级用已装版；dsh 缺失才查
  dist-tags→就绪引擎离线零网络）、LaunchSpec 执行形态 DshEntry（引擎档=dsh
  启动器直接执行）、no-open 探测泛化（启动器形态同缓存机制）、打包内置 pnpm
  （scripts/fetch-pnpm-bundle.sh + build.yml 步骤 + resources/pnpm/.gitignore）、
  render-product.sh 升 v3 snapshot。197 测试全绿。探测层退役/WSL 客体投递/
  升级呈现随后续刀。）
- [x] 文档同步（2026-09-03 完成）：ADR-0010 定稿 + AGENTS 红线 2 / §6 例外册 /
  §7 登记 / §9 索引 + contract v3 章节落稿（`format: 3` 随 P3 实现升版）+
  broadcasts 落档 + CONTEXT.md 术语表建立。
- [ ] 插件/创建操作改用引擎 node/dsh（插件操作已落地 2026-09-04；创建链随 P3-b）。
- [ ] 升级清单（随壳 bump pnpm 必过）：runtime set 可用、`pnpm add -g` 可用、
  spawnSync("pnpm") 可达、三平台 boot 冒烟。

## 6. 复审条件

- pnpm runtime 语义再变（下一 major 弃用 `runtime set` / 布局再迁移）→ 重开，
  首选回退方案 B（自有下载器自 git 历史恢复）。
- 发生 node 镜像 SHASUMS256 污染/投毒事件 → 重评信任模型（是否恢复签名哈希仲裁）。
- 安装包体积劣化超预期（pnpm 二进制 >25MB/平台）→ 重评内置策略。
  （2026-09-04 维护者裁定：**安装包内压缩存储**——bundle 携带压缩 blob，首启
  引导期解压落 `<数据目录>/engines/bin/`；安装后磁盘占用仍为解包体积 ~32MB，
  收益在安装包/分发体积。spike 0003 §4 边界 A 结案。）
- dsh 上游将 `spawnSync("pnpm")` 增加回退参数 → pnpm 降级为可选，本 ADR 重开。

## 7. 裁定台账（2026-09-03 全节点评审，grilling 三轮）

| 节点 | 裁定 |
|:---|:---|
| 就绪判定 | 三件齐验版本；不满足 → 幂等补缺，不作为错误 |
| boot 节点 | step0 检查引擎 / step1 准备引擎 / step2 启动工作台；WSL 客体同构 |
| node 升级 | 显式：更新入口提示（文案含不更新负面后果：安全修复缺失 / 未来 dsh 门槛抬升后无法追新）→ 决定 → 标记下次启动生效 |
| dsh 升级 | 同 node 显式口径；boot 比对排除预发布；boot 恒用已装版本启动 |
| 升级呈现 | 更新入口徽标常驻 + 新版本首次出现时窗口内非阻断提示条；拒绝后不再弹窗、无硬惩罚 |
| 升级失败 | 用户决策过的失败 → 更新入口反馈可重试；检测性失败 → 纯日志 |
| 版本保留 | node 当前 + 上一成功版，更老 GC；dsh 单份覆盖（pnpm 两段式安装兜底） |
| 离线语义 | 首启必须联网；之后 registry 不可达 → 已装引擎直接启动 |
| 引擎目录 | `PNPM_HOME = <数据目录>/engines/`；`<engines>/bin` 必须入子进程 PATH |
| 客体投递 | Windows 包 resources 内置两份 pnpm：win-x64（壳/宿主用）+ **linux-x64（glibc，客体用）**；壳自动投递，用户零安装。客体 node/dsh 引导**仅支持 glibc 发行版**（Ubuntu/Debian 等）：2026-09-04 裁定统一不考虑 Alpine 后，原 musl 静态份取消（其存在理由即全发行版兼容）；glibc 底线由 node 官方构建的 2.28 决定（pnpm glibc 构建更宽松）。客体探测识别 musl 系时出可行动提示，随 P3 落地 |
| 客体镜像主权 | 客体 pnpm 属壳资产，下载源壳注入镜像链；「用户镜像主权」只约束用户自身 npm/pnpm 配置（ADR-0004 据此补修订记录） |
| 客体旧目录 | `~/.dsh-dock/node` 弃用不迁移不删除 |
| 存量兼容 | 旧 `tools/node` 弃用不迁移；首次启动走一次完整引导 |
| node-map 治理 | nodeVersion / minShellVersion 继续消费；六平台 SHA 保留不消费（方案 B 回退时免改版复用） |
| 可观测 | 诊断页展示引擎 node/pnpm/dsh 版本 + 档位 + 就绪状态；插件操作改用引擎 node/dsh（移除系统 node/dsh 硬要求） |
| 契约 v3 | resolution/fallback 废止；`runtime.mode: "engine"` 缺省，声明 snapshot 三件套即快照档 |
