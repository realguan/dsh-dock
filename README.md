# DSH Dock

[dsh](https://www.npmjs.com/package/@deepseek-ai/dsh) 的桌面管理面板：
**Tauri v2 壳**，把 dsh 工作台以独立、可安装、跨平台的桌面应用呈现，并在不修改
dsh 源码的前提下提供 dsh 的全局管理能力——Profile 全生命周期管理已落地，
插件 / 设置 / 会话 / 诊断按 [docs/roadmap.md](docs/roadmap.md) 演进。
macOS / Windows / Linux 三平台安装包由 [CI](.github/workflows/build.yml) 自动产出。

## 它做什么

- **在线极简档**：安装包不内置 Node 或 dsh（壳本体仅数 MB）。首次启动必须联网，
  缺少 Node / pnpm / dsh 时自动补齐——Node 官方发行包（SHA-256 固定校验）落到
  应用私有缓存，dsh 经 pnpm 优先、npm 回退装进全局；全程国内镜像优先、官方源兜底。
- **宿主解析链**：已装官方 dsh 的机器直接复用（过版本区间 / engines / 平台三重闸，
  借执行器、不借配置）→ 可选内置快照兜底 → 实时下载补齐。详见
  [docs/contract.md](docs/contract.md)。
- **签名的 Node 版本映射**：Node 版本来自 npm 上的签名映射包
  （[node-map/](node-map/)，npm 包 `@dsh-dock/node-map`），ed25519 验签、六平台校验和齐全才采纳，任何失败
  回退壳内置基线——**升级 Node 不需要发新壳**。
- **下载体验**：字节级实时进度（`boot:progress` 事件）+ HTTP Range 断点续传
  （跨镜像可续，最终整包 SHA-256 仲裁）+ 连接/读双超时（慢网络不被整体超时掐断）。
- **单实例**：二次启动唤起已有窗口，不会出现双进程、双下载或并发安装。
- **同生命周期**：壳退 = dsh 停（SIGTERM → SIGKILL 兜底）；dsh 崩溃就地错误卡，
  带可行动动作（重试 / 升级）。
- **Profile 管理器**：可视化管理 dsh 的全部工作台配置档案——列出（已物化 +
  内置模板名「可首启」两态）、详情（插件组合 / 依赖 / patch 原文）、创建
  （走 `dsh plugin` 半官方转发链，三件套由 dsh 自己写，壳零复刻；「已创建未装
  插件」中间态明确呈现）、复制 / 重命名 / 删除（引用面按
  [ADR-0009](docs/adr/0009-profile-manager.md) 执行：排除 node_modules、
  `name` 一致化改写、不级联会话数据）、设默认启动（下次启动跳过选择器直接
  进入）。菜单 / 托盘常驻入口；删除 / 重命名有运行中防护，删除确认明示
  不级联全局数据。
- **pnpm 环境保障**：pnpm 与 Node / dsh 同列 boot 硬依赖（dsh 的 `plugin`
  子命令硬编码依赖它）；缺失时 boot 期自动经 `npm i -g pnpm` 补齐（镜像序
  与 dsh 安装同链），失败阻断启动并给可行动建议；创建 profile 时复用同一
  补齐函数。
- **执行环境抽象（executor）**：local（本机）/ wsl（WSL2 发行版）同等地位，
  SSH 预留。壳的 boot / 就绪 / 监护与执行环境无关；**WSL 只存在于 Windows**——
  Windows 首次打开可选运行环境并设默认（壳侧持久化仅 `settings.json` 的
  `defaultMode` 与默认启动 profile `defaultProfile` 两个字段），托盘可随时
  切换；非 Windows 机器零 WSL 感知（首次直接本机启动、无选择页、无 WSL
  菜单/按钮）。WSL 迭代 v1 零配置（探测即用默认发行版，须 WSL2 方具备
  localhost 端口转发），PATH 兼容 nvm/fnm（交互登录壳 `bash -lic` +
  安装位兜底扫描，0.4.2 起），缺 dsh 自动安装（`npm i -g`，需客体内有
  node/npm）。
- **内嵌 WebView 呈现**：主窗口加载 `http://127.0.0.1:<port>/` 的 dsh Web UI
  （`--port 0` 由 OS 分配，从日志解析实际地址，无端口冲突）。

## 壳与产品的关系

壳是通用机制，产品是数据：壳不感知任何具体产品身份。运行时只读
`resources/product.manifest.json`（spawn 哪个 node / 哪份 dsh / boot 哪个 profile）；
产品名称 / 标识符 / 图标是构建期身份，由打包方经 `scripts/render-product.sh` 注入。
接口两侧共同遵守的契约见 [docs/contract.md](docs/contract.md)。

`render-product.sh` 是纯打包期工具：把离线快照与产品身份物化进 `src-tauri/resources/`
后再 `cargo tauri build`。应用运行时从不执行它；默认的在线极简档构建完全不经过
这个脚本。

## 开发

```bash
# 前端是 React SPA（frontend/，React 19 + TS strict + Tailwind v4 + Zustand）
cd frontend && npm ci
npm run typecheck   # tsc 全量类型检查
npm run lint        # eslint
npm run test        # vitest（纯逻辑）
npm run build       # 生产构建

# Rust 单元测试与闸门（宿主解析链、下载续传、签名验证、profile 生命周期等）
cd src-tauri && cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

**本地运行**（默认在线极简 manifest；首次运行需联网）：

> ⚠️ debug 构建的前端从 `devUrl`（localhost:1420，vite dev server）加载——
> **直接 `cargo run` 而 vite 未运行 = 壳窗口白屏**（主窗口看不出异常，因为
> boot 完就导航进 dsh 工作台了；独立壳窗口如 Profile 管理器会全白）。
> `Cargo.toml` 在 `src-tauri/` 下，cargo 命令须先进入该目录。二选一：

```bash
# a) 一条命令（tauri-cli 自动先起 vite 再编译运行，热重载）
cargo tauri dev

# b) 两终端
cd frontend && npm run dev    # 终端 1：vite dev server @1420
cd src-tauri && cargo run     # 终端 2
```

```bash
# 出安装包无此问题（release 构建内嵌 frontend/dist 产物）；
# tauri-cli 需与 crate 同代 2.11.x
cd src-tauri && cargo tauri build

# 出内置离线档（可选）：先注入自包含快照 + 产品身份再打包；仅装配方/CI 使用，
# 应用运行时不依赖此脚本
scripts/render-product.sh \
  --node <platform-node> --dsh-runtime <node-modules-root> --dsh-home <virtual-home> \
  --profile default --name "我的 DSH Dock" --id com.me.dshdock
cd src-tauri && cargo tauri build
```

## 升级 Node 运行时（不发新壳）

映射包的更新与签名流程见 [node-map/README.md](node-map/README.md)：
改 `map.json` → `node scripts/sign.mjs` → `npm publish`。密钥轮换流程同页说明。

## 品牌与图标

- 桌面客户端全部图标（安装包 / Dock / 任务栏 / 启动页徽章）一律使用 **dsh 官方标**
  （鲸鱼标，源自 dsh-web-frontend 的 `favicon.svg`，深色模式官方渲染即白标）。
- 图标是**生成产物**，不手绘：改 `assets/icon-master.svg` 后跑
  `scripts/regen-icons.sh` 整体重生成（`rsvg-convert` → `cargo tauri icon`）。
- 页内徽章统一走 `Emblem` 组件（CSS mask + `frontend/public/mark.svg`，形状源
  `assets/dsh-logo.svg`）上色，颜色由主题控制，启动序列 / 运行环境选择 /
  工作台选择 / 关于 / Profile 管理器 全部一致。

## 结构

```
dsh-dock/
├── AGENTS.md              # AI 编码宪法（所有 AI 工具的共享上下文）
├── frontend/              # React SPA（React 19 + TS strict + Tailwind v4）
│   └── src/
│       ├── pages/         # BootIndex / BootMode / BootSelector / About / ProfileManager
│       ├── components/    # boot / about / profiles / layout / ui（shadcn）
│       ├── stores/        # Zustand：boot / 客户端更新 / profiles
│       ├── lib/           # tauri.ts（IPC 唯一入口）/ events.ts（事件总线）/ resource.ts
│       ├── content/       # zh-CN.ts（文案集中）
│       └── types/         # IPC 载荷类型（锚定 Rust serde 形状）
├── src-tauri/
│   ├── src/
│   │   ├── main.rs        # 入口
│   │   ├── lib.rs         # run()：窗口/菜单/托盘 + boot 编排 + IPC 命令
│   │   ├── ipc.rs         # IPC 命令单一事实源（三处同步机器闸门）
│   │   ├── executor.rs    # 执行环境抽象：local / wsl（ssh 预留），壳只认识 Executor
│   │   ├── manifest.rs    # product.manifest.json 契约（format=1/2）
│   │   ├── resolve.rs     # 宿主解析链（system → bundle → download）
│   │   ├── updates.rs     # 唯一网络面：版本检测 / Node 下载 / dsh 与 pnpm 安装 / 签名映射
│   │   ├── profiles.rs    # Profile 管理器：扫描 / 详情 / 创建转发链 / 生命周期 / 默认值
│   │   ├── settings.rs    # settings.json（defaultMode / defaultProfile，原子写）
│   │   ├── updater.rs     # 桌面客户端自更新状态机
│   │   └── shell.rs       # spawn / URL 解析 / 优雅停止 / wait_for_ready
│   ├── capabilities/      # ACL：按窗口授权（main / about / profiles）+ 回环 remote
│   └── resources/         # product.manifest.json（可选 dsh-snapshot/ 离线产品档）
├── node-map/              # 签名的 Node 版本映射包（发布到 npm）
├── scripts/               # render-product.sh / regen-icons.sh
└── docs/
    ├── contract.md        # 壳 ↔ 装配方 接口契约
    ├── CONTRIBUTING.md    # 协作指南（分支 / review / 占用声明 / 发布）
    ├── roadmap.md         # 产品路线图（4.3 Profile 管理器等）
    ├── contracts/         # 公共模块契约规范与 dsh 行为复现台账
    ├── adr/               # 架构决策记录（TEMPLATE.md 起家）
    └── …                  # executor.md（WSL 行为明细）/ macos-signing.md / broadcasts.md
```

## License

[MIT](LICENSE)
