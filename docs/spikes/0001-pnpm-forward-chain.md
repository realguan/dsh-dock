# Spike A：GUI 无 shell rc 下 pnpm 转发链可用性验证

- **日期**：2026-08-27
- **执行人**：guan（AI 代理协作）
- **状态**：✅ 已完成——转发链可用，无需注入或壳侧复刻
- **输入**：roadmap 4.3 前置 spike（`docs/roadmap.md` 第 142 行「验证 GUI 环境（无 shell rc）下经 `dsh plugin` 转发链 pnpm 是否可用、失败时如何给可行动报错」）
- **输出**：本结论文档 → 喂给 ADR-0009（Profile 管理器技术方案）

---

## 1. 问题定义

Profile 创建的主路径是 spawn `dsh plugin --profile <新名> add <bundle>`——dsh 内部把除 `--profile <名>` 外的参数**原样转发**给 profile 目录内的 pnpm。而 GUI 子进程不加载 shell rc，pnpm 可能不在 PATH——这是 ADR-0005 踩过的坑的**同源风险**（GUI 子进程环境 vs 用户登录 shell 环境不同）。

要回答的问题：
1. dsh-dock spawn dsh 时注入的 PATH 里，pnpm 是否可定位？
2. 转发链（initProfile → spawnSync pnpm → reconcile）在无 rc 环境下是否完整走通？
3. pnpm 不在 PATH 时，是否已有可行动报错文案？

---

## 2. dsh 源码锚定

### 2.1 转发实现（`@deepseek-ai/dsh/lib/plugin-9h8shc4d.js`）

`runPlugin(profile, args)` 三件事：

1. **init-if-needed**：若 `join(dir, "package.json")` 不存在 → `initProfile(dir, PROFILE_TEMPLATES[profile] ?? DEFAULT_PROFILE_BUNDLES)`；输出 `dsh: initialized profile <name> at <dir>`。
2. **转发**：`spawnSync("pnpm", args.map(anchorPathSpec), { cwd: dir, stdio: "inherit", shell: process.platform === "win32" })`。**POSIX 上 shell: false**——不借 shell，直接查进程继承的 PATH。
3. **reconcile**：pnpm exit 0 → `reconcilePlugins(before, dir)` 回写 `package.json` 的 `dsh.profile.bundles`；失败输出 `dsh: pnpm failed in profile directory <dir>`。

### 2.2 出错分支（同文件）

```js
if (result.error !== void 0) {
    if (result.error.code === "ENOENT") {
        process.stderr.write(`${NAME}: pnpm not found on PATH — install pnpm to manage profile plugins\n`);
        return 127;
    }
    throw result.error;
}
```

**dsh 自带可行动报错**：`pnpm not found on PATH — install pnpm to manage profile plugins`，退出码 127。

### 2.3 initProfile 三件套（`dsh-app-boot/lib/index.js`）

初始化写三个文件（`initProfile` @ 13418）：
- `package.json`：`{ name: "dsh-profile-<basename(dir)>", private: true, dependencies: {}, dsh: { profile: { bundles: [...] } } }`
- `cordis.patch.yml`：`[]`
- `pnpm-workspace.yaml`：`packages: [.]` + `nodeLinker: hoisted` + `autoInstallPeers: false`

### 2.4 profile 名为合法校验（`resolveProfileDir` @ 11826）

拒绝：空名、含 `/` 或 `\`、`.`、`..`、字面量 `node_modules`。**壳侧必须与 dsh 逐字一致复用**（roadmap §1 事实边界陷阱条）。

---

## 3. 实机验证（2026-08-27，macOS arm64）

### 3.1 环境

- dsh v0.1.1-rc.2（`/Users/guan/.npm-global/lib/node_modules/@deepseek-ai/dsh`，本机全局安装）
- node v24.18.0（fnm 管理，`~/.local/share/fnm/node-versions/.../bin`）
- pnpm 10.24.0（`~/Library/pnpm/pnpm`，shell wrapper）
- 模拟 GUI PATH：`node_bin` 首位 + `effective_path` 固定目录集合（`~/.npm-global/bin` `~/.local/bin` `~/Library/pnpm` `~/.local/share/pnpm` `/opt/homebrew/bin` `/usr/local/bin` `/usr/bin` `/bin`），**不 source 任何 shell rc**
- `DSH_HOME` 指向全新临时目录（隔离，不污染真实 `~/.dsh`）

### 3.2 测试 1：转发链完整走通（新 profile 名触发 init）

```
dsh plugin --profile spike-a-1787842371 add @deepseek-ai/dsh-base
```

**结果（stdout 关键行）**：
```
dsh: initialized profile spike-a-1787842371 at <tmp>/dshhome/profiles/spike-a-1787842371
Progress: resolved 1, reused 0, downloaded 0, added 0
```

**结论**：
- initProfile 成功写出三件套（`package.json` / `cordis.patch.yml` / `pnpm-workspace.yaml`，内容与 §2.3 逐字一致——已 `cat` 校验）。
- **pnpm 被成功找到并执行**（`Progress: resolved ...`），无 `pnpm not found`。
- `reused 0 / downloaded 0 / added 0`：pnpm 全部命中本地 store——本机 store 已有 `@deepseek-ai/dsh-base` 完整依赖，**该场景纯本机完成，零网络**。
- 网络注意点：pnpm 在 resolved 阶段还会挂一个到 `registry.npmmirror.com`（本机 `~/.npmrc` 配置）的 audit 请求，本次出现 `ECONNRESET` 重试（网络问题，**与 PATH/转发链无关**）。这启示 4.4 插件管理：pnpm 失败模式要区分「转发链失败」vs「网络失败（镜像不可达）」——文案应反映后者。

### 3.3 测试 2：pnpm 不在 PATH 的失败分支

保留 node 可见，但把 `~/Library/pnpm` 从 PATH 移除 → 模拟「有 node、无 pnpm」的 GUI 子进程。

```
dsh plugin --profile spike-nopnpm-<pid> add @deepseek-ai/dsh-base
```

**结果**：
```
dsh: initialized profile spike-nopnpm-<pid> at <tmp>/dshhome/profiles/spike-nopnpm-<pid>
dsh: pnpm not found on PATH — install pnpm to manage profile plugins
exit=127
```

**结论**：
- initProfile 仍先执行（写三件套）——**即 dsh 允许「创建 profile 成功但插件安装失败」的中间态**，壳侧需容忍此状态（profile 目录存在 + 空依赖）。
- 失败文案精确命中 dsh 自带文案，exit 127。**壳无需自造文案**，可直接展示 dsh stderr；若需更细的可行动建议（如 macOS 上装 pnpm 的具体命令），可再包装一层。

### 3.4 测试 3：dsh-dock PATH 注入链（静态核查，无需实机重跑）

`shell.rs:80-84` 已有既有实现：
```rust
.env("PATH", crate::resolve::path_with_bin(node_bin, &crate::resolve::effective_path()))
```

`effective_path()`（`resolve.rs:301`）= 合并多路（去重保序）：登录 shell PATH（`login_shell_path`，zsh→bash `-lc` 2s 超时）→ 固定目录（含 `~/Library/pnpm`、`~/.npm-global/bin` 等）→ fnm/nvm bin → 当前 PATH。

`path_with_bin`（`resolve.rs:321`）把选中的 node 目录放首位——**这同时保证 dsh 自己、以及 pnpm wrapper（`~/Library/pnpm/pnpm` 是 shell script 需要 node）都能找到 node**。

**结论**：dsh-dock 已有环境感知链，spawn dsh 时注入的 PATH 天然包含 pnpm 常见安装位（`~/Library/pnpm`、`~/.npm-global/bin`），转发链成立的前提已有保障。测试 1/2 模拟的就是这个注入后的 PATH。

---

## 4. 结论与对 4.3/4.4 的设计指引

### 4.1 结论

| 问题 | 结论 |
|:---|:---|
| 转发链在无 rc 环境可用吗？ | **可用**。dsh-dock 已有 PATH 补全注入（`effective_path`），pnpm 可定位；实测完整走通 init → pnpm → reconcile。 |
| 需要 global-bin-dir 式注入吗？ | **不需要**。ADR-0005 的风险是 `pnpm add -g`（全局安装）路径；转发链是 `pnpm add`（profile 目录内安装），不涉及 `global-bin-dir`。二者同源（GUI 环境）但机制不同。 |
| 壳侧复刻 initProfile 需要吗？ | **主版本不需要**。转发链已验证成立；壳侧复刻只在「pnpm 不可用且 profile 为空」时兜底——且 dsh 已把 init 与 pnpm 分开（init 先执行、pnpm 失败不回滚），所以「创建成功但未装插件」的中间态天然存在。 |
| 失败文案谁出？ | **dsh 自带**（`pnpm not found on PATH — install pnpm to manage profile plugins`，exit 127）。壳侧可包装更细的可行动建议（按平台给安装命令），但基础文案无需自造。 |

### 4.2 对 4.3 创建路径的影响

- 主路径用 `dsh plugin --profile <名> add <bundle>`，**不依赖 shell rc**，无需额外注入。
- 创建流程的预期中间态：initProfile 先执行（三件套落盘）→ pnpm 安装插件 → reconcile 回写 bundles。若 pnpm 失败（网络/PATH），profile 目录已存在但依赖未安装——UI 应显示「profile 已创建，插件未安装」状态，而不是「创建失败」。
- 失败模式要分两类：① pnpm 不在 PATH（dsh 文案 + 壳侧补充安装建议）② 网络/镜像失败（提示检查网络或镜像配置）。
- 命名校验：与 `resolveProfileDir` 逐字一致（空名 / `/` `\` / `.` / `..` / `node_modules`）。

### 4.3 对 4.4 插件管理的影响

- `add` / `remove` / `update` 全部复用同一条 `runPlugin` 转发链——**Spike A 的结论直接覆盖 4.4 的主要操作路径**。
- 进度显示：`stdio: inherit` 意味着 pnpm 输出直接进 dsh 子进程 stdout/stderr（壳侧日志文件）——壳要展示进度需解析 dsh 日志的 pnpm 行（`Progress: ...`），或另起封装。
- pnpm 失败模式与 4.3 共用同一套错误处理与文案。

---

## 5. 遗留事项

- [ ] pnpm 网络失败（镜像不可达）的实机一测（本次测试 1 遇到 `ECONNRESET`，但为镜像 audit 非安装主链路）——归 4.4 插件管理的失败模式验证。
- [ ] Windows 平台转发链（`shell: win32` 分支）实机一测——本 spike 为 macOS 实机，Windows 语义不同（spawnSync 走 shell），3.4 静态核查不覆盖。
- [ ] profile 名为 `web`/`headless` 时的模板命中（`PROFILE_TEMPLATES` 只命中这两个名字）——4.3 创建时若用户指定默认模板名，走 dsh 内置模板而非 `dsh-base`。
