# DSH Dock

dsh 的桌面管理面板（Tauri v2 壳）。壳自带运行引擎，用户的 dsh 世界按原样进入，本词汇表统一环境准备阶段的领域语言。

## Language

**引擎（Engine）**:
壳自管的运行时集合：node + pnpm + dsh 包树，三者由 pnpm 负责下载、布局与激活，壳只做编排。
_Avoid_: 工具链、运行时（泛指时）、bundle

**引擎档（Engine mode）**:
产品 manifest 未声明快照时的默认运行形态——用引擎启动工作台。
_Avoid_: download 档、在线档

**快照档（Snapshot mode）**:
产品 manifest 声明 snapshot 三件套时的运行形态——用内置只读快照启动，离线可用。
_Avoid_: bundle 档、fallback、内置档

**用户世界（World）**:
dsh_home 指向的目录（`$DSH_HOME` 或 `~/.dsh`）：profiles、插件、会话、凭据的归属地。引擎在其上执行，世界归用户所有。
_Avoid_: home 目录（裸用）、dsh 数据目录

**引导（Bootstrap）**:
首次使用（或引擎残缺）时，从零把引擎三件备齐的过程。
_Avoid_: 补齐链、下载档

**就绪判定（Readiness check）**:
每次启动对引擎三件（node / pnpm / dsh）的版本校验；不满足 → 进引导补缺，不作为错误。
_Avoid_: 环境检查、probe（指执行器整体探测时除外）

**引擎目录（Engine dir）**:
壳数据目录下 `engines/`，即注入给子进程的 `PNPM_HOME`，引擎三件的实际落点。
_Avoid_: tools/（旧布局）、私有 prefix
