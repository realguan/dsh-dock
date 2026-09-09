# ADR-0013：pnpm 构建脚本默认批准（profile 级 `dangerouslyAllowAllBuilds`），审批门逻辑退役

- **日期**：2026-09-09
- **状态**：**已接受**（2026-09-09 维护者裁定：默认批准、不交给用户逐包裁决、审批链全删）
- **提出人**：guan（AI 起草，依据用户实测反馈与当日 bug 定位）
- **相关方**：`src-tauri/src/build_policy.rs`（新增）、`profiles.rs`（创建链）、
  `plugins.rs`（插件操作前补齐）；退役面：`build_approvals.rs`、`set_profile_build_approvals`
  IPC、`frontend/src/components/profiles/BuildApprovalDialog.tsx`、`lib/buildApprovals.ts`、
  队列 `blocked_gate` 态
- **关联**：ADR-0009 第六次修订（写入例外 #5，**随本 ADR 退役**）、ADR-0011（队列内联审批裁定，
  同批退役）、ADR-0010（引擎档 pnpm 版本是壳资产）、复现点 12；AGENTS §6 例外册 / §7 IPC 登记

---

## 1. 背景与问题

pnpm 12 默认拦截依赖的安装脚本，壳在 ADR-0009 第六次修订里把它产品化：解析 dsh 转发链输出
→ 逐包裁决 → 受控改写 `allowBuilds` → 重试。**两种门槛形态**（复现点 12 + 2026-09-09 实测）：

- npm 来源：`ERR_PNPM_IGNORED_BUILDS`，pnpm 写 `allowBuilds: {包名: "set this to true or false"}`
  占位模板并退出 1；
- git/tarball 来源：`ERR_PNPM_GIT_DEP_PREPARE_NOT_ALLOWED`，pnpm **不写模板**，只在 help 示例里
  给出 exact key（`名@解析后来源URL#commit`，被终端按宽度折行）。

三条事实汇成本次决策：

1. **解析器是单样本归纳**。两种错误码、终端折行、exact key 重组，全按 pnpm 12.3.1 的输出
   形态写死（`build_approvals.rs`，558 行含测试）。ADR-0011 已把「任何新增来源/错误形态必须
   端到端跑通一轮」写进规程——因为单样本归纳是本类缺陷的共同根因。
2. **2026-09-09 实测失效（当日 bug）**：`run_dsh_forward` 的输出改为从**追加式**日志读全量
   （commit 9d341f8），解析器因此在历史文本里先撞上旧的 git 门槛标记 → 装 `dsh-ssh`（真实门槛
   = `cpu-features` / `ssh2`）却弹出 **dsh-pet 的旧 exact key**；批准写的是那个旧键，对真门槛
   无效 → 重试再撞 → 死循环（plugin-op.log 三次同型失败；test profile 的 `cpu-features`/`ssh2`
   至今仍是占位串）。**门槛解析的正确性依赖「喂给它的输出恰好是本次运行」这一隐含前提**，
   而该前提随无关改动（日志策略）静默失效。
3. **用户裁定**：这条链维护成本高于收益，改为默认批准。

## 2. 约束与硬指标

1. **红线 1 不动 dsh 源码**：只能改 profile 文件层（写入例外登记制）。
2. **零新依赖**、**纯函数可测**（AGENTS §5）。
3. **增量**（AGENTS §8.2）：本次只动构建脚本策略，不捎带其他模块重构。
4. **不写坏用户文件**：只动一个顶层键，其余逐字节保留；结构不可理解时拒绝写入。
5. **跨平台一致**：macOS / Windows / WSL 同一套（键在 profile 文件里，与平台无关）。
6. **以引擎档 pnpm 为准**（ADR-0010）：本机实测版本 12.3.1。

## 3. 备选方案及评估

### 方案 A：profile 级 `dangerouslyAllowAllBuilds: true` —— ✅ 最终采纳

- 思路：向 profile 的 `pnpm-workspace.yaml` 写入 pnpm 自带的开关，pnpm 直接跳过审批门；
  壳删除解析/裁决/重试整条链。
- **实测矩阵**（2026-09-09，引擎档 pnpm 12.3.1，`/tmp` 双向复现）：

  | 场景 | 无旗标 | 有旗标 |
  |:---|:---|:---|
  | npm 依赖带 install 脚本（`ssh2@1.17.0` → `cpu-features@0.0.10`） | `ERR_PNPM_IGNORED_BUILDS` 退出 1 | 安装成功，无门槛 |
  | git 托管包带 `prepare`（本地 `git+file://` 构造） | `ERR_PNPM_GIT_DEP_PREPARE_NOT_ALLOWED` | 安装成功，**且 `prepare` 真的执行**（产物文件已生成） |
  | 旗标 + 既有 `allowBuilds: {键: false}` / 占位串 | — | 不报 `OVERRIDING` 错，直接通过；`false` 也被压过（git 案例实证脚本仍执行） |

- 优点：**两种门槛形态一次覆盖**；存量 profile 的占位/旧裁决无需清理；解析器、对话框、队列态、
  IPC 全部可删（约 700+ 行 + 64 处引用）；上述当日 bug 随解析器一起消失。
- 代价/风险：见 §5 负面后果（供应链门禁关闭）。
- 对照约束：①只写 profile 文件；②纯函数 + 原子写；③本次只动这一件事；④只动一个顶层键、
  无变化零写入；⑤键与平台无关；⑥实测即 12.3.1。

### 方案 B：保留门禁，只把「逐包裁决」改为「自动批准」 —— ❌ 否决

- 思路：仍解析被点名包，但不再弹框，直接写 `true` 并重试。
- 否决理由：解析器（两种错误码 + 折行 + exact key）与「输出必须是本次运行」的隐含前提仍在，
  §1.2 那类失效会重演——只是从「弹错包」变成「写错键」；维护成本没有下降。

### 方案 C：引擎全局 pnpm 配置（`pnpm config set dangerouslyAllowAllBuilds true --global`） —— ❌ 否决

- 思路：一处生效所有 profile。
- 否决理由：全局配置只在壳注入的 `PNPM_HOME`/config-dir 下生效，用户从终端启动 dsh 时读的是
  自己的 pnpm 配置 → 同一 profile 两种口径；且新增一个持久化面（AGENTS §6 登记制）却不如
  profile 文件显式（profile 复制/重命名本就会带走它）。

### 方案 D：维持现状（逐包裁决） —— ❌ 否决

- 否决理由：维护成本与失效风险见 §1；用户已明确裁定不采用。

### 方案 E：向 dsh 上游提诉求（dsh 侧提供构建审批开关） —— ❌ 本 ADR 否决（保留为长期诉求）

- 思路：让 dsh 自己处理 pnpm 门槛。
- 否决理由：违反红线 1 的「不自造上游补丁」精神且不在本仓控制面；作为上游诉求保留，不阻塞本次。

## 4. 最终决策

**采用方案 A**：`build_policy.rs` 向 profile 的 `pnpm-workspace.yaml` 幂等写入
`dangerouslyAllowAllBuilds: true`（创建 profile 后写一次；每次插件操作前幂等补齐），
pnpm 不再进入审批门。**退役**：`build_approvals.rs`、`set_profile_build_approvals` IPC、
`PluginOpOutcome.ignored_builds`、前端审批对话框与队列 `blocked_gate` 态、相关文案与测试。
ADR-0009 第六次修订（写入例外 #5）与 ADR-0011 的队列内联审批裁定随本 ADR 回收，不双源。

## 5. 后果与后续行动项

### 正面后果

- 插件安装不再被构建审批门中断（半安装态、批准后重试、死循环三类问题一并消失）。
- 删除解析器/对话框/队列态/IPC，维护面从「猜 pnpm 输出形态」回到「写一个键」。
- 存量 profile 的占位模板与旧裁决无需迁移，旗标压过它们。

### 负面后果 / 新增债务

- **供应链门禁关闭**：任何插件的安装脚本（`postinstall`/`prepare`）都会在**无审阅**的情况下
  执行，包括市场/总览一键装进来的第三方 git 包。pnpm 给该键起名即带 `dangerously`。
- 撤销只能手工（删键 / 改 `false` / 删旗标）——壳不再提供逐包裁决 UI。若将来需要「默认拒绝
  特定包」，需重开本 ADR。
- 键名与语义由 pnpm 掌控，pnpm 大版本变更会让它失效（复审条件已列）。

### 行动项

- [ ] `build_policy.rs`：`ensure_allow_all_builds`（纯函数，只动一个顶层键，其余逐字节保留，
      结构不可理解时拒绝）+ `ensure_profile_build_policy`（原子写、无变化零写入）+ 单测
- [ ] 接线：`create_profile_blocking` 物化后写一次；`mutate_plugin_blocking` 运行前幂等补齐
      （写入失败不阻断操作，只 warn——pnpm 原始报错仍在 detail 里）
- [ ] 退役：`build_approvals.rs`、`set_profile_build_approvals`（ipc.rs / lib.rs / capabilities
      三处）、`PluginOpOutcome.ignored_builds` 与其文案分支
- [ ] 前端：删 `BuildApprovalDialog.tsx` / `lib/buildApprovals.ts` / 队列 `blocked_gate` 态与
      内联审批面板 / `types/ipc.ts` 与 `ipc-shapes.json` 相关形状 / 文案
- [ ] 文档同步：AGENTS §6 例外册 #5 回收、§7 删 IPC 登记、§9 补本 ADR；ADR-0009 第六次修订与
      ADR-0011 队列审批裁定标注退役并指向本 ADR；复现点 12 标注退役
- [ ] 验证：`cargo test` + `cargo fmt --check` + `clippy -D warnings` + 前端
      typecheck/lint/test 全绿；`cargo tdev` 实机装 `dsh-ssh` 不再弹审批且装上

## 6. 复审条件

- **pnpm 大版本**（13+）变更/移除 `dangerouslyAllowAllBuilds` 键名或语义 → 重评（引擎档 pnpm
  是壳资产，ADR-0010，升级时逐条复核本 ADR）。
- dsh 上游自带构建审批处理（复现点 12 的升级复核项命中）→ 改为依赖上游口径。
- 出现「默认拒绝某些包」的真实诉求（如某插件构建脚本被曝恶意）→ 重开本 ADR，届时优先考虑
  profile 级 `allowBuilds` 显式 false 覆盖而非恢复解析器。
- 用户要求恢复逐包审阅 → 重开本 ADR（不得在退役后的代码里悄悄复活解析链）。
