# 平台对齐开发计划（2026-09-21 · 快车道）

> 来源 = [`docs/team/平台对齐审计-2026-09-21.md`](../team/平台对齐审计-2026-09-21.md)（依据 `AGENTS.md` §0 红线 3）。
> 第一刀已落地：**A1**（Linux 托盘启动即崩）· **A7**（取份交叉闸门）· **A4**（凭据 0600 披露）· **A5**（日志台读错世界）。
> 本计划排剩余项，按**能否在本机验证**分三条道 —— 分道是为了"不许把没验的东西说成验过了"。

## 道 A｜本机可完成且可验证（**本轮 5/5 完成**）

| # | 项 | 动作 | 验收 |
|:--|:--|:--|:--|
| A-1 | **A6** `render-product.sh` 快照档 | Windows 上 `node.exe` 被 `cp` 成无扩展名 `dsh-node` ⇒ CreateProcess 找不到。改按目标平台落 `dsh-node(.exe)` 并同步 manifest `nodeBin` | 脚本文本闸门（三平台名映射 + manifest 同步）+ 本机跑一次 dry 校验 | ✅ `462d6fb`
| A-2 | **§3.9** Windows 包"必须含两份 tgz" | 闸门只查"leg 声明的份 ⊆ 白名单"，不查"leg 取全"。补 `leg → 必需份` 断言 | python 闸门新增用例；负例：把 Windows leg 改成只取一份即红 | ✅ `29026e2`
| A-3 | **§2.10** `mgmt.rs` 死码 | `unsupported_in_wsl` / `require_local` 零调用点，唯一引用是前端单测里逐字复制的文案 | 删除 + 前端 fixture 清理；`cargo test` / vitest 全绿 | ✅ `43027bc` + `0e4c7ff`
| A-4 | **§3.5** 「全局快捷键」措辞 | 实际是页内 `keydown`（ADR-0024 已裁"暂缓"），文案却写"全局" | zh/en 双侧 + RELEASE_NOTES 措辞校正；字典对称测试绿 | ✅ `fcdd4eb`
| A-5 | **§3.7** README 补 Windows 提示 | macOS 有 Gatekeeper 提示、Windows 无 SmartScreen 提示（披露不对称，签名是成本问题另说） | README 增补；零成本 | ✅ `fcdd4eb`

## 道 B｜依赖 Windows 真机（本轮不做，理由与前置写死）

| # | 项 | 为什么现在不做 |
|:--|:--|:--|
| B-1 | **A2** Windows 登记表孤儿清扫第二层 | 修法 = `DuplicateHandle` + `STARTUPINFOEX`/`HANDLE_LIST` 让子进程真正继承锁句柄。本机编不了 Windows 目标、更跑不了；改完**无法证明**，只会把"已知失效"变成"未知是否失效"。**前置** = 恢复 Windows 真机验证（`docs/roadmap.md` §4.16 / `docs/executor.md` 现为搁置） |
| B-2 | **A4 尾** Windows 显式 DACL（`icacls` / `SetNamedSecurityInfoW`） | 同上（且已用披露止损） |
| B-3 | **A6 尾** 快照档 Windows 实机验证 | 脚本改了，但"落成 `dsh-node.exe` 后能起来"只能在 Windows 上证明 |
| B-4 | **§3.1** Windows 毛玻璃（`Effect::Mica`，同一 `EffectsBuilder` API）+ 自绘 chrome | 技术可行性已核（crate 源码），但观感与自绘控件行为必须真机调；且属**产品取舍**，需先裁定做不做 |
| B-5 | **§3.4** 「托盘常驻 / 最小化运行」名副其实（`ExitRequested` + `prevent_exit`） | 改的是**关窗语义**：macOS 无托盘，"隐藏而非退出"与平台惯例冲突 ⇒ 需维护者先定各平台期望行为 |

## 道 C｜需裁定或属新功能（本轮只登记，不动手）

| # | 项 | 待定问题 |
|:--|:--|:--|
| C-1 | ~~**A3** WSL 客体档 5 项下沉~~ ⇒ **已升为最高优先级（2026-09-21 维护者裁定）**：这四项是核心能力，而 WSL 客体模式是红线 3 的适配目标，「需补原语」是欠债自陈不是限制。施工方案见 [`wsl-parity-plan-2026-09-21.md`](wsl-parity-plan-2026-09-21.md)（P0 客体 patch 内核 → P1 能力目录只读 → P2 MCP 探测） |
| C-2 | **A8** macOS Intel 执行级验证 | 三条路各有代价：`macos-15-intel` 原生 leg（runner 可用性未确认）· arm64 runner 装 Rosetta 跑 x86_64 测试· 维持只编译。**且 macOS 侧无 boot 冒烟作业**（GUI 会话依赖） |
| C-3 | **§3.6** Linux 无托盘宿主时 About/更新入口不可达（`host.ts` 恒 `clientUpdate: true`） | 需裁定：加窗口内入口，还是改能力矩阵措辞 |
| C-4 | **§3.2** Windows 平台标记（`data-windows-titlebar`） | 与 B-4 同批；单独做有布局风险 |
| C-5 | **§3.10** 自启 / 系统通知 / 深链接 / 文件选择器 / 窗口状态 | 三平台一致缺失（非不对称）⇒ 属产品功能排期，不属红线 3 |
| C-6 | **§3.3** ARM64 覆盖 | 红线 3 现文只写 x64；README 已显式"仿真运行"。要扩需改红线文本 |

## 执行纪律

- 每条独立提交、带闸门或可复跑验证；**负例实测**后才算完成（本轮 A-2 必带负例）。
- 快车道 = 直推 master + 广播落档；不裸打 tag、不动版本号。
- 诚实边界随条目走：本机验不了的一律标进道 B/C，不写成"已修"。

## 执行记录（2026-09-21 快车道本轮）

- **道 A 全部完成**：A-1 `462d6fb` · A-2 `29026e2` · A-3 `43027bc`+`0e4c7ff` · A-4/A-5 `fcdd4eb`。
  每条都带闸门或可复跑验证，A-1/A-2 另有负例实测（见广播）。
- **道 B 未动**：前置是"恢复 Windows 真机验证"（维护者裁定项），不是工作量问题。
- **道 C 未动**：C-1（A3 WSL 5 项）是独立一轮的活；C-2（Intel 执行验证）三条路各有代价，
  需先选路；C-3/C-6 需裁定；C-4 与 B-4 同批；C-5 属产品功能排期。
- 本轮顺带修正：`render-product.sh` 的 `--out` 此前"用法写了、解析没实现"。
