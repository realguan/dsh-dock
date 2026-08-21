# dsh-desktop-shell

ADR-0004 的「产品壳」：一个极小的 **Tauri v2 桌面壳**，把装配好的 dsh 工作台以一个
**独立、可安装、跨平台**的桌面版 DeepSeek Harness 呈现给最终用户。

> 位置关系（见 [dsh-launcher ADR-0004](https://github.com/realguan/dsh-plugin-hub/tree/main/dsh-launcher/docs/adr/0004-standalone-desktop-package.md)）：
> 启动器是装配车间，工作台是装配工位；本仓库是「样机的可复刻外壳」——同一份壳，
> 装入不同快照即得到不同桌面产品。

## 设计原则

- **壳是通用机制，产品是数据**：壳不感知任何具体产品身份。运行时只读
  `resources/product.manifest.json`（要 spawn 哪个 node / 哪份 dsh / boot 哪个 profile）；
  产品名称 / 标识符 / 图标是**构建期身份**，由 `scripts/render-product.sh` 注入。
- **同生命周期**：壳退 = dsh 停。应用退出时对 dsh 优雅停止（SIGTERM → SIGKILL 兜底）。
- **内嵌 WebView 呈现**：主窗口加载 `http://127.0.0.1:<port>/` 的 dsh Web UI。
  可加载性由 [探针](https://github.com/realguan/dsh-plugin-hub/tree/main/dsh-launcher/docs/adr/0004-standalone-desktop-package.md)
  实证：无 CSP/XFO、全应用同源、`/api` 非鉴权。

## 契约与装配

- [docs/contract.md](docs/contract.md) —— `product.manifest.json` 与快照目录布局（两侧共同遵守的接口）。
- [scripts/render-product.sh](scripts/render-product.sh) —— 把快照 + 构建期身份注入壳工程。

## 开发 / 构建

```bash
# 壳自带前端是免构建静态页（ui/），无需 node/npm。

# 单元测试（URL 解析、manifest 校验等）
cd src-tauri && cargo test

# 本地运行（用默认 sample manifest；真实快照需先 render）
cargo run

# 当前平台出安装包
cargo tauri build

# 注入一个真实产品后出包：
../scripts/render-product.sh \
  --node <platform-node> --dsh-runtime <node-modules-root> --dsh-home <virtual-home> \
  --profile default --name "我的 DeepSeek Harness" --id com.me.dshdesktop
cd src-tauri && cargo tauri build
```

三平台安装包由 [.github/workflows/build.yml](.github/workflows/build.yml) 的 CI matrix
自动产出（macOS / Windows / Ubuntu）。

## 品牌与图标

- 桌面客户端全部图标（安装包图标 / Dock / 任务栏 / 加载页 logo）一律使用 **dsh 官方标**，
  源自 dsh-web-frontend 的 `favicon.svg`（深色模式官方渲染即白标，本壳忠实采用）。
- 图标是**生成产物**，不手绘：改标请改 `assets/icon-master.svg`（官方 path 合成 + 深色圆角底），
  再跑 `scripts/regen-icons.sh` 重生成全部平台产物（`rsvg-convert` → `cargo tauri icon`）。
- 官方原始 SVG 落于 `ui/assets/dsh-logo.svg`（溯源副本）。

## 结构

```
dsh-desktop-shell/
├── ui/                    # 壳自带加载页（静态，无框架无构建器）
├── src-tauri/
│   ├── src/
│   │   ├── main.rs        # 入口
│   │   ├── lib.rs         # run()：读契约 → spawn dsh → 导航 WebView → 退出优雅停止
│   │   ├── manifest.rs    # product.manifest.json 契约（format=1）
│   │   └── shell.rs       # spawn/URL 解析/优雅停止（移植自 dsh-launcher process_guard）
│   ├── capabilities/default.json   # remote: http://127.0.0.1:*（回环 Web UI）
│   ├── resources/         # product.manifest.json + dsh-snapshot/（自包含快照）
│   └── tauri.conf.json    # 默认身份；render-product.sh 在此注入产品身份
├── scripts/render-product.sh
├── sample/product.manifest.json
└── docs/contract.md
```
