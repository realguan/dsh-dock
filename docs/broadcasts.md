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
