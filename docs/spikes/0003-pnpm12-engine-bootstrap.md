# Spike 0003：pnpm12 引擎引导——镜像注入通道与 runtime set 非 TTY 实机验证（macOS）

- **日期**：2026-09-03
- **执行人**：guan（AI 代理协作）
- **状态**：✅ macOS 实机完成（Spike①② 的 macOS 侧闭环；Windows/Linux 复验与 ④ WSL 投递通道仍待实机）
- **输入**：ADR-0010 行动项 Spike①（镜像注入三平台复验）/ Spike②（`pnpm runtime set node` 三平台非 TTY 验证）；案头核查为 v12.3.1 源码级（见 ADR §5）
- **输出**：本结论文档 → 喂给 P2（引擎私有 pnpm）/ P3（引擎编排模块）实现；两处需维护者知会的边界见 §4
- **实证环境**：macOS darwin-arm64 · pnpm 12.3.1（`@pnpm/exe.darwin-arm64` 内二进制）· node-map pin v24.18.0

---

## 1. 问题定义

引擎倒置（ADR-0010）的三个落地疑问，源码推演不能替代实机：

1. node 下载镜像注入通道在生产 shell（非 TTY 子进程）里是否真的被 env 接管？JSON 值的键是什么？
2. `pnpm runtime set node` 在非 TTY 下是否可用、有无可解析进度、node 落在什么布局、npm 是否真的缺位？
3. 引擎目录怎么摆：PNPM_HOME 与 runtime 项目能否同目录共存？内置 pnpm 二进制从哪取材（须走镜像链）？

## 2. 实验记录与证据

### 2.1 pnpm12 取材通道（P3 内置前置）

- npmmirror **无** pnpm 二进制镜像目录（`/-/binary/pnpm/` → `[NOT_FOUND] Binary "pnpm" not found`）。
- 但 pnpm v12 的 npm 包带平台二进制可选依赖 **`@pnpm/exe.<platform>`**：8 个平台齐全
  （linux-x64 / linux-arm64 / darwin-x64 / darwin-arm64 / win32-x64 / win32-arm64 /
  **linux-x64-musl / linux-arm64-musl**），全部是普通 npm tarball，registry 镜像链（npmmirror）直接可达。
  → **内置取材走既有 registry 镜像链成立，无需 GitHub**；WSL 客体的 musl pnpm 同通道。
- 实测解包：`@pnpm/exe.darwin-arm64@12.3.1` tarball 内 `pnpm` 为 Mach-O arm64 可执行，
  `--version` 输出 12.3.1。**解包后体积 32MB**（见 §4 边界 A）。
- `dist-tags.latest` 实测仍指 11.25.0（2026-09-03）——**pin 必须显式版本号**再次坐实。

### 2.2 `runtime set node` 非 TTY 实机（Spike② macOS）

- `PNPM_HOME=<dir>` 且 `<dir>/bin` 入 PATH，`pnpm runtime set node 24.18.0`（stdout 管道非 TTY）：
  **exit 0**，52.08MB 下载 4–8s 完成。
- **字节级进度行可解析**：`Downloading node@runtime:24.18.0: 1.33 MB/52.08 MB`（多行递增）
  → 映射 `boot:progress` 可行（正则 `^Downloading node@runtime:(\S+): ([\d.]+) ([KMG]?B)/([\d.]+) ([KMG]?B)$`）。
- **布局是项目作用域**：runtime set 在 CWD 生成/改写 `package.json`
  （写入 `devEngines.runtime = {name:"node", version:"<v>", onFail:"download"}`）、
  `node_modules/.pnpm/node@runtime+<v>/`、`pnpm-lock.yaml`；PNPM_HOME 下只有 `store/` 与 `bin/`。
- **激活需要 `pnpm shim add node`**：装完 node 不自动进 bin（输出原话 "To make the bare \"node\"
  command project-aware, run \"pnpm shim add node\""）。shim 后 `PNPM_HOME/bin/node`
  是对 `node_modules/.pnpm/node@runtime+<v>/node_modules/node/bin/node` 的**硬链接**，直接可执行。
- **npm 缺位实证**：引擎 node 树内仅 `node` 一个文件——npm / npx / corepack 均不解包（v11+ 口径）。
- 幂等性：`devEngines` + lockfile 使 runtime set 可重入（换版本 = 卸旧装新，实测 20.19.4 → 22.20.0 平滑切换）。

### 2.3 引擎目录单目录方案（P3 布局输入）

`PNPM_HOME = <数据目录>/engines/` **同时作为 runtime 项目**：同目录下 `bin/`（node shim +
全局命令 shim）、`global/`（add -g 落点）、`node_modules/`（runtime 虚拟 store）、
`package.json` + `pnpm-lock.yaml`（devEngines 锁版本）共存，实测互不干扰：

```
engines/
├── bin/            # node shim（硬链）、add -g 的命令 shim、（生产）捆绑的 pnpm 二进制
├── global/         # pnpm add -g 的包树（dsh 落这里）
├── node_modules/   # runtime set 的 node@runtime+<v>
├── package.json    # devEngines.runtime 锁 node 版本
├── pnpm-lock.yaml
└── store/          # 内容寻址 store（可经 PNPM_CONFIG_STORE_DIR 指到别处共享，实测复用免重下）
```

### 2.4 镜像注入通道实锤（Spike① macOS 复验）

- **源码锚定**（v12.3.1，sparse clone 核对）：`crates/config/src/env_overlay.rs` 模块注释明确
  只读 `PNPM_CONFIG_<KEY>` / `pnpm_config_<key>` 两形态；`npm_config_*` / `NPM_CONFIG_*`
  pnpm 已停读；**裸 `NODE_DOWNLOAD_MIRRORS` 无效**（那只是 schema 字段名）。
- **值的键 = 发布通道**：`crates/engine-runtime-node-resolver/src/get_node_mirror.rs` ——
  映射按 `release` / `nightly` / `rc` / `test` / `v8-canary` 索引，缺键**静默回退**
  `https://nodejs.org/download/<channel>/`。默认官方基座 = `nodejs.org/download/release/`（≠ `/dist/`）。
- **决定性路由证据**：`PNPM_CONFIG_NODE_DOWNLOAD_MIRRORS='{"release":"http://127.0.0.1:8765/"}'`
  + `runtime set node 22.20.0` → 本地 404 服务器收到
  `GET /index.json` 与 `GET /v22.20.0/SHASUMS256.txt`，pnpm 报错
  `Failed to fetch SHASUMS256.txt (http://127.0.0.1:8765/v22.20.0/SHASUMS256.txt) … (status: 404)`。
  → env 接管 + 解析流程 = 拉镜像 `index.json`（版本列表）→ 拉 `SHASUMS256.txt`（强制校验）→ 拉 tarball。
- 案头结论「键名写错静默无效」实测复现：值用 `{"nodejs":…}`（错误的键）时坏镜像不报错、照常从默认源下载成功。
- **生产形态 e2e**：`{"release":"https://npmmirror.com/mirrors/node/"}` → runtime set 22.20.0
  成功（4.2s）；npmmirror 的 node dist 布局与基座逐文件对齐（HEAD 302→200，52MB）。
- **SHASUMS256 强制且与镜像同源**：校验文件从同一镜像基座拉取；lockfile 另带逐平台
  sha256 `integrity`（variations 表，musl 走 `unofficial-builds.nodejs.org`，见 §4 边界 B）。

### 2.5 完整引擎链 e2e（macOS 全绿）

`engines/` 内依序执行（全部非 TTY）：

1. `pnpm runtime set node 22.20.0`（注入 release 镜像 env）→ ✅
2. `pnpm shim add node` → ✅ `engines/bin/node -v` = v22.20.0
3. `pnpm add -g @deepseek-ai/dsh --registry=https://registry.npmmirror.com` → ✅ 0.1.1-rc.2（5.1s）
4. `engines/bin/dsh --version` → **0.1.1-rc.2**（dsh 经 shim 由引擎 node 执行）

→ 引擎 pnpm / node / dsh 三件在单目录引擎内闭环；dsh 内部 `spawnSync("pnpm")` 所需的
`engines/bin` 入 PATH 即可满足（生产由壳注入）。

## 3. 对 ADR-0010 台账的确认与细化

| 节点 | 结果 |
|:---|:---|
| 引擎目录 | ✅ 确认单目录方案（§2.3 布局图）；`<engines>/bin` 同时容纳捆绑 pnpm、node shim、全局 dsh shim |
| 就绪判定 | ✅ 有利性质：devEngines 锁版本 + `onFail: download`，runtime set 可重入 = 幂等补缺 |
| 可观测 | ✅ 进度行可解析映射 `boot:progress`（§2.2 正则） |
| 离线/信任模型 | ✅ SHASUMS256 强制、与镜像同源（与 ADR 记录一致）；lockfile 另有逐平台 integrity |

## 4. 需维护者知会的边界（不阻塞 P2，P3 前需裁定）

- **边界 A（触发 ADR §6 复审条件）**：`@pnpm/exe.darwin-arm64@12.3.1` 解包后 **32MB**，超过
  ADR §6 写定的「>25MB/平台 → 重评内置策略」线（ADR 草案期 17–19MB 是压缩包估算，安装态实为 32MB）。
  备选：接受 32MB / 安装包内压缩存储解压落 engines / 回退方案 B。
  **2026-09-04 裁定：安装包内压缩存储**（bundle 带压缩 blob，首启解压落 `<数据目录>/engines/bin/`；
  磁盘解包 32MB 不变，收益在安装包/分发体积）——结案。
- **边界 B（WSL 客体镜像主权缺口）**：musl node 变体的资产列举在 pnpm 源码中**硬编码**
  `unofficial-builds.nodejs.org`（`get_node_mirror` 映射只覆盖 release/nightly/… 官方通道）——
  WSL 客体内经 pnpm 装 musl node 的下载源**不可镜像注入**。实测本网络可达（HTTP 200），
  暂接受并挂复审条件；若需严格镜像主权，客体 node 可改由壳下载器投递（方案 B 残留能力）。

## 5. 遗留待办

- [ ] Spike①② Windows / Linux 实机复验（同清单：镜像 env、非 TTY runtime set、签名包 resources 执行许可）
- [ ] Spike④ WSL 客体投递通道选型（`\\wsl$` 拷贝 vs wsl.exe stdin base64）
- [x] 边界 A 裁定（2026-09-04：安装包内压缩存储）
- [ ] 边界 B 裁定（客体 musl node 源：暂接受 + 复审条件 vs P3 加壳投递兜底）
