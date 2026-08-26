# DSH Dock

[dsh](https://www.npmjs.com/package/@deepseek-ai/dsh) 的桌面终端：一个极小的
**Tauri v2 壳**，把 dsh 工作台以独立、可安装、跨平台的桌面应用呈现给最终用户。
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
- **执行环境抽象（executor）**：local（本机）/ wsl（WSL2 发行版）同等地位，
  SSH 预留。壳的 boot / 就绪 / 监护与执行环境无关；首次打开可选运行环境并设默认
  （`settings.json` 仅 defaultMode 一个持久化字段），macOS 菜单 / 托盘可随时切换；
  WSL 迭代 v1 零配置（探测即用默认发行版，须 WSL2 方具备 localhost 端口转发），
  PATH 兼容 nvm/fnm（交互登录壳 `bash -lic` + 安装位兜底扫描，0.4.2 起），
  缺 dsh 自动安装（`npm i -g`，需客体内有 node/npm）。
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
# 壳自带前端是免构建静态页（ui/），无需 node/npm。

# 单元测试（解析链、下载续传、签名验证等）
cd src-tauri && cargo test

# 本地运行（默认在线极简 manifest；首次运行需网络）
cargo run

# 当前平台出安装包（默认在线极简档：不内置 Node/dsh，无需任何前置步骤）
cargo tauri build

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
- 页内徽章统一走 `ui/assets/mark.svg`（形状源）+ CSS mask 上色，颜色由主题控制，
  三页（启动序列 / 工作台选择 / 关于）完全一致；官方原始 SVG 溯源副本在
  `ui/assets/dsh-logo.svg`。

## 结构

```
dsh-dock/
├── ui/                    # 壳自带页面（静态，无框架无构建器）
│   ├── index.html         # 启动序列（时间线 + 下载进度 + 错误卡）
│   ├── selector.html      # 工作台选择器（system 档多 webUi profile）
│   ├── about.html         # 关于面板（版本 + 检查/升级）
│   └── assets/            # 样式 + 官方标（mark.svg / dsh-logo.svg）
├── src-tauri/
│   ├── src/
│   │   ├── main.rs        # 入口
│   │   ├── lib.rs         # run()：契约读取 → 宿主解析 → spawn dsh → 导航 WebView
│   │   ├── executor.rs    # 执行环境抽象：local / wsl（ssh 预留），壳只认识 Executor
│   │   ├── manifest.rs    # product.manifest.json 契约（format=1/2）
│   │   ├── resolve.rs     # 宿主解析链（system → bundle → download）
│   │   ├── updates.rs     # 唯一网络面：版本检测 / Node 下载 / dsh 安装 / 签名映射
│   │   └── shell.rs       # spawn / URL 解析 / 优雅停止
│   ├── capabilities/      # remote: http://127.0.0.1:*（回环 Web UI）
│   └── resources/         # product.manifest.json（可选 dsh-snapshot/ 离线产品档）
├── node-map/              # 签名的 Node 版本映射包（发布到 npm）
├── scripts/               # render-product.sh / regen-icons.sh
└── docs/contract.md       # 壳 ↔ 装配方 接口契约
```

## License

[MIT](LICENSE)
