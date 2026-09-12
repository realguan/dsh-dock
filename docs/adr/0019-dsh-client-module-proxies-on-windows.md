# ADR-0019：Windows 模块代理模式下补齐浏览器客户端模块（client modules）

- **日期**：2026-09-12
- **状态**：已接受
- **提出人**：guan（AI 协作）
- **相关方**：`engines.rs`（bootstrap 与 client-proxies 落位）· `dsh-client-proxies.mjs` · ADR-0018（模块代理前置）· ADR-0017
- **关联**：ADR-0018 · Web 工作台报错 `Failed to load plugins: client-modules: HTML did not preload @deepseek-ai/dsh-client-modules/client.js`

---

## 1. 背景与问题

在 ADR-0018 中，Windows 侧通过在 `dsh-boot.mjs` 中设置 `process.pkg ??= { dshDock: true }`，成功命中了 dsh 自带的「模块代理（module proxy）」模式。该模式将 `$DSH_HOME/profiles/node_modules` 下原有的 481+ 个符号链接转换为真实目录与 `entry-N.js` 转发文件，彻底消除了 Windows 普通账户在启动阶段因缺少 `SeCreateSymbolicLinkPrivilege` 权限而报的 `EPERM: symlink` 错误。

然而，在模块代理模式下，当 dsh 后端服务（3080 端口）成功启动后，用户在浏览器中打开 Web 工作台时，前端立即白屏崩溃并抛出严重错误：

```text
Failed to load plugins
client-modules: HTML did not preload @deepseek-ai/dsh-client-modules/client.js
```

### 1.1 根因分析

通过审查 `@deepseek-ai/dsh-app-boot/lib/index.js` 中 `ensureModuleProxy()` 的实现，定位到如下核心逻辑断层：

1. **dsh 生成代理清单时丢弃了前端元数据**：
   `ensureModuleProxy()` 只关注 Node.js 端 ESM 模块解析所需的路径，为每个包生成的代理 `package.json` 形式如下：
   ```json
   {
     "name": "@deepseek-ai/dsh-client-modules",
     "version": "...",
     "private": true,
     "type": "module",
     "exports": { ".": "./entry-0.js" },
     "dsh": {
       "moduleFallback": {
         "targets": {
           ".": "file:///.../dsh-runtime/node_modules/.../client.js"
         }
       }
     }
   }
   ```
   它**只**保留了 `dsh.moduleFallback.targets`，而源包中原本声明的 `dsh.client`（如 `{ "platform": "web" }`）以及针对浏览器的客户端导出 `exports["./client"]` 被**完全丢弃**。

2. **运行时加载路径分叉**：
   - 在 dsh 官方闭源打包可执行文件（packaged executable）中，客户端模块清单是在构建期直接内嵌并从 snapshot 载入的，运行时无需回查磁盘代理目录；
   - 在 DSH Dock 使用的纯 Node.js 安装中，工作台前端插件系统在启动时必须经由文件系统扫描 `$DSH_HOME/profiles/node_modules` 下各包的 `package.json` 来寻找 `dsh.client` 声明并提取客户端 bundle；
   - 由于代理目录中的清单缺失了 `dsh.client` 与 `exports["./client"]`，前端扫描结果为 0，HTML 模板未生成任何 preload 标签，导致前端在加载插件时触发断言崩溃。

---

## 2. 约束与契约不变量

- **绝对不修改 dsh 上游源码**（AGENTS 红线 1）。
- **必须严格维护 dsh 模块代理的幂等性契约**（破坏这三条会导致 dsh 强制删除并重建整个代理树）：
  1. `dsh.moduleFallback.targets` **必须逐字节完全保留**（dsh 的 `moduleFallbackEntryCurrent` 会逐键逐值比对 targets）；
  2. `version` 字段**必须完全保留**；
  3. 每个 `entry-N.js` 原有内容与解析方式不得改动。
- **纯增量与幂等性**：补齐逻辑必须是可加的（additive）和幂等的（idempotent），再次运行时如无变化不执行任何磁盘写入。
- **跨平台与目录可搬迁性**：不得硬编码绝对路径。

---

## 3. 架构方案与决策

**采纳方案：在 `<engines>/bin/` 下落位专用的代理增强脚本 `dsh-client-proxies.mjs`，在 `dsh-boot.mjs` 执行 `runCli()` 前完成预热与增量补齐。**

### 3.1 客户端模块代理补齐器（`dsh-client-proxies.mjs`）

实现纯 Node.js 模块 `augmentClientProxies(modulesDir)`：
1. 遍历 `$DSH_HOME/profiles/node_modules` 下所有目录；
2. 识别 dsh 托管的代理目录（存在 `dsh.moduleFallback.targets`）；
3. 从 `targets` 的 `file://` 真实路径向上寻获真实的物理包目录及其原始 `package.json`；
4. 提取该包的 `dsh.client`（若 `platform === "web"`）及 `exports["./client"]`；
5. 将真实的浏览器端 bundle（及 `.map` 文件）复制到代理目录下，命名为独立的 `dsh-client-bundle.js`（避免与 dsh 自身生成的 `entry-N.js` 发生命名冲突）；
6. 增量重写代理目录的 `package.json`：
   ```json
   {
     "...": "保持原有所有字段（含 targets 和 version）不变",
     "exports": {
       "...": "原有 exports 保留",
       "./client": "./dsh-client-bundle.js"
     },
     "dsh": {
       "...": "原有 moduleFallback 保留",
       "client": { "platform": "web" }
     }
   }
   ```
7. 采用 `copyIfChanged` 与字节比对，确保二次执行零磁盘写入。

### 3.2 启动引导预热（`dsh-boot.mjs`）

在 bootstrap 脚本中，于执行 `mod.runCli()` 前执行预热：
1. 若参数为版本查询（`-V` / `--version`），直接跳过预热，保证极速响应；
2. 依托 `realpathSync(dshPkg)` 建立 `createRequire`，稳定动态导入上游的 `@deepseek-ai/dsh-app-boot`，调用其 `healProfilesModuleFallback({ installAnchor, home })` 先行在磁盘落位代理结构；
3. 相对引入同目录的 `./dsh-client-proxies.mjs`，执行 `augmentClientProxies` 补齐客户端文件与声明；
4. 捕获非特权异常并给出告警提示，不阻塞 CLI 主进程；若遇到 `EPERM: symlink` 则正常抛出以触发绊线。

### 3.3 引擎布局与就绪刷新（`engines.rs`）

1. 新增 `DSH_CLIENT_PROXIES_NAME: &str = "dsh-client-proxies.mjs"`，通过 `include_str!` 内嵌；
2. 在 `ensure_dsh_runtime_layout` 中统一调用 `write_dsh_client_proxies(data_dir)`；
3. 新安装与存量就绪启动路径（`refresh_dsh_layout_if_project_local`）均能自动落位并校正该脚本，实现无感知平滑升级。

---

## 4. 验证与测试凭据

1. **真实环境复现与验证（红/绿闭环）**：
   - 构造最小复现沙箱，无增强时浏览器打开直接抛错 `Failed to load plugins: client-modules: HTML did not preload @deepseek-ai/dsh-client-modules/client.js`；
   - 经 `augmentClientProxies` 增强后，成功加载全部 52+ 个插件与 client-modules，工作台正常渲染。
2. **端到端真机真实 dsh 启动测试**：
   - `real_dsh_boots_in_proxy_mode` 测试实测结果：
     `真实 dsh boot：软链=0 顶层条目=184 代理 entry-*.js=1184 客户端 bundle=55 进程存活=true`；
   - 验证了 55 个客户端 bundle 完全落位，零符号链接，进程稳定启动。
3. **自动化测试闸门**：
   - Rust 单测通过 `bootstrap_has_three_steps_relative_entry_and_tripwire`、`client_proxies_mjs_contains_contract_invariants`、`write_dsh_client_proxies_is_idempotent`、`ready_launch_path_refresh_is_idempotent`；
   - `cargo test`: 418 passed; 0 failed; 5 ignored；
   - `cargo fmt --check`: clean；
   - `cargo clippy --all-targets -- -D warnings`: clean；
   - Node 语法校验：`node --check src-tauri/src/dsh-client-proxies.mjs` 通过。

---

## 5. 后果与影响

- **正面收益**：Windows 用户在无需管理员/开发者模式权限的情况下，不仅能成功启动 dsh 后端，更能完整加载所有前端插件并正常使用 Web 工作台，本地模式闭环可用。
- **上游建议**：在向 upstream 提交报告时，建议上游的 `ensureModuleProxy` 将原始包中的 `dsh.client` 与 `exports["./client"]` 纳入代理清单中，消除打包模式与纯 Node 模式的前后端契约差异。
