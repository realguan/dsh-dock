# ADR-0011：插件管理职责边界与安装来源白名单

- **日期**：2026-09-08
- **状态**：已接受
- **提出人**：guan（经 grilling 会话裁定）
- **相关方**：`src-tauri/src/plugins.rs`、`frontend/src/components/market/*`、
  `frontend/src/components/profiles/ProfileDetailPane.tsx`、`docs/roadmap.md` §4.4
- **关联**：ADR-0009（插件写入例外册，本 ADR 不改其任何写入边界）；问题记录095 #3/#4

---

## 1. 背景与问题

1. **两处插件 UI 职责含混**：控制中心「插件中心」（市场 + 已安装总览）与 Profile
   详情「外挂插件」Tab 存在能力重叠——安装三处都有入口、配置复制两处都有；
   更新/启停/卸载/运行态只在详情。ADR-0009 第五次修订注「总览视图不承载写操作」
   与现状（市场/总览「安装到…」已是 `install_plugin` 写操作）脱节。
2. **市场半数插件安装必败（hotfix 主因）**：线上 registry（awesome-dsh-plugin.com，
   2026-09-08 实拉 3363 条）中 1736 条（51.6%）install spec 为 `github:owner/repo`
   或 `github:…#path:/sub`（130 条为带引号的 tarball 直链 `"https://…tgz"`）。
   壳的 `validate_plugin_spec` 白名单字符集 `@/._^~*-` 不含 `:` `#`，GitHub/tarball
   来源在 `install_plugin` 入口即被拒（用户实测复现：dsh-pet 安装报
   「包名只允许字母数字与 @/._^~*-…」）。pnpm 倘生支持这些形态（spec 原样转发），
   唯一挡路点是壳的校验白名单。
3. **弹窗内容溢出（同批 UI 缺陷）**：`DialogContent` 基类无 `max-h`、无滚动，
   fixed 居中定位下视口不足时弹窗上下两端溢出且不可滚动救援——全站弹窗共用此基类。

## 2. 约束与硬指标

- spec 以单 argv 传给 dsh→pnpm（无 shell 参与），校验职责 = 注入安全（旗标、
  空白/控制字符），**语义合法性归 pnpm/dsh**（不重造包管理器校验）。
- 更新检查只打 npm registry（`npm_packument_versions` 镜像链，§7 唯一网络面）——
  安装白名单放宽**不得**让 `github:…` 形态流进 packument 查询。
- 写入面不变：安装/卸载/更新仍经 `dsh plugin` 转发链（ADR-0009），pnpm 12
  构建审批门（写入例外 #5）覆盖全部来源。
- 恶意反例必测（AGENTS §5）。

## 3. 备选方案及评估

### 方案 A：职责立宪 + 安装专属三形态白名单 —— ✅ 最终采纳

- 思路：总定位「跨 profile 归插件中心，单 profile 归详情」；校验谓词拆二——
  安装走新 `validate_install_spec`（npm spec / `github:owner/repo[#frag]` /
  `https://…tgz` 直链），更新检查与选版本保持 `validate_plugin_spec` 严格
  npm 判别；弹窗基类补 `max-h` + 滚动。
- 优点：市场 1736 条非 npm 来源条目即刻可装；两谓词各自语义单一；安全面不扩
  （argv 单传下 `:` `#` 无注入语义，仍拒前导 `-`、空白、`><` 区间）。
- 代价/风险：git dep 无版本 pin（默认分支快照），`update` 对 git dep 的
  pnpm 语义待实测（行动项 ④）；「分发」对 github 包拼 `pkg@npm版本` 必败
  （行动项 ⑤，改从来源 `package.json` dependencies 取 spec）。
- 对照约束：注入安全=字符集+前导 `-` 白名单，恶意反例入测；网络面不变；
  写入面不变。

### 方案 B：不放宽，市场对不支持来源明示「请终端安装」 —— ❌ 否决

- 思路：维持白名单，UI 如实暴露能力缺口。
- 否决理由：51.6% 的发现面不可安装，市场作为「发现+安装」面名存实亡；
  缺口交还终端与壳的定位（roadmap：壳是唯一插件安装管理入口）自相矛盾。

### 方案 C：全量收口插件中心（唯一管理面） —— ❌ 否决

- 思路：插件中心接管更新/启停/卸载（跨 profile 批量），Profile 详情插件 Tab 退化只读。
- 否决理由：与「profile 是插件组合的归属单元」心智冲突；启停/运行态语义
  锚定单 profile（patch 行 id），批量面会稀释运维精确性；grilling 裁定
  Q1 三选一明确落 A。

### 方案 D：分发走文件级复制（cp node_modules 条目 + 抄声明） —— ❌ 否决（2026-09-09 调研）

- 思路：同机 profile 就是磁盘目录，把 A 已装的插件目录与依赖声明直接复制
  到 B，绕过 pnpm 的网络与构建环节。
- 否决理由：① `nodeLinker: hoisted` 把全部依赖拍平进同一 node_modules 根，
  插件的传递依赖与其他插件混布，复制边界无法划定，漏了传递依赖运行时才炸；
  ② pnpm 记账面（`pnpm-lock.yaml` 完整性哈希、`node_modules/.pnpm/lock.yaml`、
  `.modules.yaml`、`.pnpm-workspace-state-v1.json`）与实际内容失配后，下一次
  插件操作会把「多余」目录对齐删除——白装；手改 lockfile 远超写入例外 #5
  的单键边界；③ 绕过 `dsh plugin add` 即绕过 reconcile 记账。且收益比直觉
  小：全局 store 内容寻址（`~/Library/pnpm/store/v11`）使同机分发内容零
  下载（ndjson 实证 `found_in_store`），真正摩擦是 git 解析触网与 per-profile
  审批门——分别归 dsh 上游透传诉求与队列内联审批（见行动项）。

## 4. 最终决策

**职责边界**：跨 profile 能力（市场发现、安装到…分发、未来跨 profile 更新
检查）归插件中心；单 profile 运维（行内安装、更新、卸载、启停、运行态诊断）
归 Profile 详情。安装来源白名单三形态两面对齐（市场卡片安装与详情手填同规格）：
npm spec（现规则不变）、`github:用户名/仓库名[:#frag]`（frag 字符集
`_./:=&-` 覆盖 `#path:/…`、`#分支`、`#提交`）、`https://…` tarball 直链；
`validate_plugin_spec` 保持严格 npm 判别继续服务更新检查/选版本，安装入口
换用 `validate_install_spec`。registry extract 层剥离成对引号（tarball 条目
数据形态）。弹窗基类补高度约束与滚动（全站）。

## 5. 后果与后续行动项

### 正面后果
- 市场约 1736 条 github/tarball 条目恢复可安装（3363 → 可装覆盖率 100%）。
- 两处 UI 职责一句话可述，后续功能归属有宪可依（问题记录095 #3 收口）。
- 全站弹窗不再随视口/内容增长溢出。

### 负面后果 / 新增债务
- git dep 无版本 pin：装的是默认分支快照，「更新」语义（pnpm 对 git dep 的
  update 行为）未实测——已知不确定性，见行动项 ④。
- 「分发」（安装到…）对 github 来源包必败：现按 `pkg@npm版本` 拼装 spec，
  github 独占包在 npm 不存在——行动项 ⑤ 修正前，github 包的分发按钮会失败。
- tarball 直链同样无版本 pin。

### 行动项

- [x] Rust：`validate_install_spec` 三形态 + 恶意反例测试；`mutate_plugin_blocking`
  换装新谓词（2026-09-08 hotfix）
- [x] 前端：`validatePluginSpec` 镜像三形态；`extractInstallSpec` 剥离成对引号；
  市场弹窗提交前预检（2026-09-08 hotfix）
- [x] `DialogContent` 基类 `max-h-[calc(100dvh-2rem)] overflow-y-auto`（2026-09-08）
- [ ] **实测** git dep 的安装/更新/卸载各一轮（dev 环境，用户复测；更新语义
  存疑——pnpm `update` 对 git dep 是否重新解析默认分支待证。2026-09-09 实测
  安装段：坐实 git 来源走 `ERR_PNPM_GIT_DEP_PREPARE_NOT_ALLOWED` 门槛 + exact
  key 为「名@完整tarball URL」——已随当日 hotfix 修入 build_approvals 解析与
  键校验；更新/卸载两段仍待测）
- [ ] 规程（2026-09-09 教训收编）：任何新增安装来源/错误形态，收口前必须
  端到端跑通一轮（安装 → 门槛 → 审批 → 重试）；解析器 fixture 单测绿不等于
  路径通——单样本归纳是本类缺陷的共同根因
- [ ] 「分发」取 spec 改从来源 profile `package.json` dependencies（小版本）
- [ ] 跨 profile 更新检查 + 批量更新进插件中心（小版本，问题记录095 #4
  下载队列随此设计）。队列形态裁定（2026-09-09 维护者确认）：**审批门以
  队列项内联审核操作呈现，不再用模态对话框**——队列项状态机
  `排队 → 解析/下载 → 待审批（内联展示被点名包的 允许/跳过 开关，阻塞该项）`
  `→ 安装中 → 完成/失败`；批准即写该 profile 的 allowBuilds 并自动重试该项，
  连续分发多 profile 的审批在队列中顺序清账（BuildApprovalDialog 的裁决行
  逻辑复用为内联面板；跳过 = 装上但不跑构建脚本，需明示可能不可用的告警）。
- [ ] 术语与死代码：动词统一「安装到…」、详情导入改「从其他 Profile 安装」、
  Tab 名改「插件」、删 `ProfileDetailDialog.tsx`（随小版本批）

## 6. 复审条件

- registry 出现三形态之外的新 install 形态（如 `#semver:` 片段、`git+https://`）。
- dsh/pnpm 升级改变 `plugin` 转发链对 git dep 的 reconcile 语义。
- pnpm 12 构建审批门对 tarball/git 来源的 `ignored_builds` 行为与 npm 来源不一致。
- dsh 上游若支持 pnpm reporter 透传（`--reporter=ndjson`，2026-09-09 实证引擎
  pnpm 12.3.1 输出 `pnpm:stage/progress/stats` 统一事件流；环境变量注入实测
  无效，reporter 为 CLI 专属选项）——安装进度解析层应从「非 TTY 计数行」
  升级为 ndjson 事件流，届时重开解析设计。`--offline/--prefer-offline` 同属
  此诉求（2026-09-09 分发调研：git 来源分发唯一触网点是解析，离线透传可让
  同机分发完全离线，与 reporter 一并向 dsh 提出）。
