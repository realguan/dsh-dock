# 产品壳契约 v1（dsh-dock / DSH Dock）

本文是**装配方 ↔ 产品壳**之间的接口定义。
两侧共同遵守，任何一侧改动须先修订本文件并升 `format`。

## 核心原则

- **壳是通用机制，产品是数据**：壳与任何具体产品解耦；一份壳源码服务所有桌面版产物。
- **运行时契约 = `product.manifest.json`**（resources 内，壳启动时读取）。
- **构建期身份 = `tauri.conf.json`**（productName / identifier / 窗口标题 / 图标），由
  `render-product.sh` 在打包时按产品注入，编译进二进制——不在运行契约里。

## 快照目录布局（resources 根）

```
resources/
├── product.manifest.json        # 运行时契约（必填）
└── dsh-snapshot/                # 自包含运行快照（与平台无关的载荷）
    ├── node/                    # 目标平台 Node 运行时（node / node.exe / 单二进制）
    │   └── bin/<node-bin>
    ├── dsh/                     # 自包含运行时依赖树根（node_modules，pnpm 布局）
    │   └── @deepseek-ai/dsh/lib/bin.js        # 含 .pnpm/ 与各包相对符号链接
    └── home/                    # 虚拟 $DSH_HOME
        └── profiles/<profile>/  # 装配好的工作台 profile（配置 + 插件 + 依赖）
```

> 平台差异只出现在 `node/` 一列：同一份载荷，三平台各自注入自己的 Node 二进制后，
> 分别执行 `tauri build`。这正是 ADR-0004「快照平台无关、装配分平台」的落地形态。
>
> `dsh/` 的物化方式 = **整树原样复制**启动器版本库 `runtimes/<v>/node_modules`
> （pnpm 隔离布局的符号链接是树内相对路径，整体复制后依然有效；等价于 `pnpm deploy` 产物），
> 不重链接、不触网、不复用宿主 store——自包含是硬指标。
>
> **发布态嵌套（2026-08-21 e2e 实测）**：Tauri v2 打包器保留相对 `src-tauri/` 的路径前缀，
> 上述 `resources/*` 在安装包内实际落在 `<资源根>/resources/` 下（如
> `.app/Contents/Resources/resources/product.manifest.json`）。壳的 `resolve_resources_dir`
> 优先探测嵌套布局、兼容平铺布局；生产验证以 `cargo tauri build` 产物为准。

## product.manifest.json（v1）

| 字段 | 类型 | 必填 | 说明 |
| :--- | :--- | :---: | :--- |
| `format` | number | ✅ | 契约版本，必须为 `1`。不匹配 → 壳拒绝启动（错误页提示重新打包） |
| `productName` | string | ✅ | 人类可读产品名（展示/日志用） |
| `snapshot.nodeBin` | string | ✅ | 相对 resources 根的 Node 可执行文件 |
| `snapshot.dshBinJs` | string | ✅ | 相对 resources 根的 dsh 入口（`dsh/@deepseek-ai/dsh/lib/bin.js`） |
| `snapshot.dshHome` | string | ✅ | 相对 resources 根的虚拟 `$DSH_HOME` |
| `snapshot.profile` | string | ✅ | 要 boot 的 profile 名 |

```json
{
  "format": 1,
  "productName": "DSH Dock",
  "snapshot": {
    "nodeBin": "dsh-snapshot/node/bin/dsh-node",
    "dshBinJs": "dsh-snapshot/dsh/@deepseek-ai/dsh/lib/bin.js",
    "dshHome": "dsh-snapshot/home",
    "profile": "default"
  }
}
```

## 运行语义（壳）

1. 读取 `product.manifest.json`（版本不兼容 → 错误页就地呈现，绝不静默降级）。
2. 校验零部件齐全后 spawn：
   `node <dshBinJs> --profile <profile> --port 0`，环境 `DSH_HOME=<dshHome>`。
3. `--port 0` 由 OS 分配端口，壳从 dsh 打在日志的地址行解析实际 URL
   （只认 `http://` / `https://`；dsh 就绪上限 20s）。
4. 主窗口 WebView 导航到 `http://127.0.0.1:<port>/`。
   适配依据（ADR-0004 探针定论）：页面无 CSP/XFO、全应用同源、`/api` 非鉴权、
   浏览器端用 SSE 收事件——WebView 直接可用，无需凭据与额外 host 配置。
5. **壳与 dsh 同生命周期**：应用退出 → 优雅停止（SIGTERM → 3s → SIGKILL）；
   dsh 意外崩溃 → 窗口导航回错误页并给出日志摘要。

## 构建流程（谁来调用什么）

```
装配方（外部打包工具或 CI）
   │  render-product.sh --product '<json>' --node <bin> --dsh-bin <js> --dsh-home <dir>
   ▼
桌面积淀到 src-tauri/resources/（product.manifest.json + dsh-snapshot/）
   │  并改写 tauri.conf.json（productName / identifier / 窗口标题 / 图标）
   ▼
cargo tauri build     （per 平台；CI matrix 三 OS）
   ▼
安装包（.dmg / .msi / .deb 等）——内嵌 resources + 编译期身份
```

## 版本与升级

桌面版是**冻结快照**（ADR-0004 D4）：产物不可变；用户升级 = 回装配车间重打一版。
壳自身升级 = 单发新壳二进制（与快照解耦的可选路径，v2 占位，见 ADR-0004 开放问题 4）。

## 契约改动流程

任何字段/布局变更 → 本文件先改 → `format` 升版本 → 壳 `MANIFEST_FORMAT` 同步 →
打包侧同步 → 三步用同一版本号发布，缺一不可。

---

# 运行时策略：终端宿主解析（ADR-0005，2026-08-21）

本产物是 **dsh 的桌面终端**（独立 Tauri 应用，与 web/tui/headless 前端并列），dsh 是宿主。
（2026-08-28 注：项目定位已扩展为「dsh 的桌面管理面板」，见 AGENTS.md §0；本节宿主解析语义不变。）
运行时不固定「内置快照」，而是走 **宿主解析链**：

```
宿主 dsh / node 解析
  ① 用户环境复用：官方安装（npm/pnpm 全局，PATH 可探）→ realpath 包树
       → 三重校验闸：版本 ∈ 声明区间 / engines.node 达标 / 平台一致 —— 过闸才复用
  ② 内置兜底：可选 bundle 副本（本产品极简档不携带，保持安装包轻量）
  ③ 实时下载：国内镜像优先，经 registry + pnpm/npm 拉取并安置版本库（缓存 + integrity，网络动作）
```

**两条铁律**
- **在线极简档不承诺离线启动**：新电脑首次运行必须联网；缺少 Node/npm/pnpm/dsh 时按下载链自动补齐。
  bundle 仍是可选的产品能力，不能把它误读成当前安装包的离线保证。
- 使用次序是独立配置，node 与 dsh 同构适用。
- **借执行器，不借配置**：复用宿主 dsh 只借其 bin.js/树，产品仍用自己的虚拟 home 与
  默认连接 profile；npx 缓存形态非复用源（版本漂移）；外部打包工具的私有版本库不视为官方形态。

## manifest v2（2026-08-21 定稿，resolution 策略）

```json
{
  "format": 2,
  "productName": "DSH Dock",
  "terminal": {
    "defaultProfile": "desktop-demo",
    "resolution": {
      "node":     { "tiers": ["system", "bundle", "download"], "requireEngines": true },
      "dsh":      { "tiers": ["system", "bundle", "download"],
                    "versionRange": ">=0.1.0-rc.6 <0.2.0", "requireEngines": true }
    }
  },
  "fallback": {
    "nodeBin": "dsh-snapshot/node/bin/dsh-node",
    "dshBinJs": "dsh-snapshot/dsh/@deepseek-ai/dsh/lib/bin.js",
    "dshHome": "dsh-snapshot/home",
    "profile": "desktop-demo"
  }
}
```

- `terminal.resolution.*.tiers`：解析次序；`system` 缺失/不达标即进下一 tier。
- `fallback`：可选的 bundle 副本（只读种子）；极简在线档不声明该字段。
- `versionRange`：SEMVER 区间（装配时定，宽区间以让复用成立）。
- v1（format=1）兼容：壳按 snapshot 三件套迁移为 bundle-only 解析 + fallback（壳 `MANIFEST_MIN_COMPAT=1`）。
- 极简档语义：不写 `fallback`、resolution 缺省即 `system → download`（终端默认形态）；内置档由装配方在产物中显式声明 `bundle` 档 + `fallback`。

实时下载的网络与包管理顺序固定为：

1. 包元数据、dsh 安装：`https://registry.npmmirror.com` → `https://registry.npmjs.org`
2. Node 执行器：`https://cdn.npmmirror.com/binaries/node` → `https://nodejs.org/dist`
3. 包管理器：用户 PATH 中的 `pnpm` → 下载/缓存 Node 自带的 `npm-cli`

Node 下载按目标平台选择官方包格式（macOS/Linux 为 `tar.gz`，Windows 为 `zip`），并使用
内置 SHA-256 校验和；Windows 安装器同时使用在线 WebView2 bootstrapper，保持安装包轻量。

pnpm 的全局目录或安装动作失败时回退 npm；因此 pnpm 是优先路径，不是桌面应用的硬依赖。
对 npm 11 显式放行 dsh 所需的 native/helper install scripts；系统全局目录无写权限时，
自动切换到应用数据目录下的私有 prefix，不要求管理员权限。

## 下载运行时语义（2026-08-24 增补）

- **Node 版本来源**：npm 映射包 `@dsh-dock/node-map`（registry 镜像链拉
  packument → tarball），`map.json` 经壳内 ed25519 公钥验签、六平台 SHA-256
  齐全才采纳；本地缓存上次验签通过的副本；任何失败回退壳内置基线
  （fail-closed）。更新流程见 [node-map/README.md](../node-map/README.md)。
- **下载体验**：进度经 `boot:progress` 事件（`{kind:"node", current, total}`，
  Rust 侧节流 ≥100ms）推给前端；HTTP Range 断点续传，`.part` 落盘跨进程/
  跨镜像可续，最终整包 SHA-256 仲裁（哈希不过即弃 `.part` 换镜像从零重下）。
- **超时纪律**：元数据请求整体限时；大文件下载用「连接 + 单次读」双超时、
  不设整体上限——慢网络下大包合法地超过一分钟，停滞连接由读超时兜底。
- **单实例**：`tauri-plugin-single-instance`（OS 级原语），二次启动唤起主窗口，
  杜绝双进程 / 双下载 / 并发写私有 prefix。

---

# 运行时策略 v3：引擎倒置（ADR-0010，2026-09-03 已接受）

> 本节废止上方 manifest v2 的 `resolution` / `fallback` 语义（v2 段落保留作历史档案）。
> `format: 3` 已随 ADR-0010 P3-b 落地升版（2026-09-04：壳 `MANIFEST_FORMAT=3`，
> render-product.sh 快照档产物同步 v3，兼容读取迁移 v1/v2——fallback→快照档、极简在线档→引擎档）。

## 形态

- **引擎档（缺省）**：manifest 不声明 `snapshot` 三件套 → 引擎启动。运行时 = 壳引擎
  （node + pnpm + dsh；pnpm12 引导器随壳内置），`PNPM_HOME` 指向壳引擎目录
  `<数据目录>/engines/`。用户世界不变：`$DSH_HOME` / `~/.dsh`。
- **快照档**：manifest 声明 `snapshot` 三件套（nodeBin / dshBinJs / dshHome / profile）
  → 内置只读快照启动，离线可用（语义沿 v1 fallback，无 resolution）。

```json
{
  "format": 3,
  "productName": "DSH Dock",
  "runtime": { "mode": "engine" },
  "snapshot": { "nodeBin": "...", "dshBinJs": "...", "dshHome": "...", "profile": "..." }
}
```

`runtime` 可省略（引擎档缺省）；`snapshot` 可省略（引擎档）。

## 语义

- **在线语义**：首启必须联网（引擎引导）；之后 registry 不可达 → 已装引擎直接启动。
- **升级**：node 与 dsh 一律显式（更新入口提示 → 用户决定 → 下次启动生效）；
  node 版本源 = `@dsh-dock/node-map`（验签定版本；SHA 字段保留不消费）；dsh 比对
  排除预发布。
- **WSL 客体**：musl pnpm 随 Windows 包 resources 内置，壳自动投递；客体 pnpm 属
  壳资产，下载源走壳注入镜像链（npmmirror → 官方；ADR-0004 修订：用户镜像主权
  条款只约束用户自身的 npm/pnpm 配置）。
