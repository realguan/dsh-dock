# Spike B：复制/重命名 profile 的引用面清点

- **日期**：2026-08-27
- **执行人**：guan（AI 代理协作）
- **状态**：✅ 已完成——真实引用面逐处锚定到 dsh 源码位置
- **输入**：roadmap 4.3 关键行动 ③「复制/重命名/删除：实现前先做一次引用清点 spike」
- **范围**：$DSH_HOME 下 + WSL 客体内（维护者授权：覆盖 WSL 客体内引用）
- **输出**：本清点文档 → 喂给 ADR-0009（Profile 管理器技术方案）

---

## 1. 问题定义

一个 profile 的名字散落在哪些地方？复制/重命名/删除 profile 时，哪些引用面必须同步更新，哪些可以「靠 dsh 自愈」？动手前要把 `$DSH_HOME` 下的真实引用面逐个点清，否则删除/重命名会留悬空引用。

---

## 2. 引用面全清单

以下按「复制/重命名/删除时该怎么做」分组。**关键前提**：dsh 的 profile 语义 = 目录名 `profiles/<名>` 是唯一身份。除目录名外没有全局注册表、没有索引文件、没有显式元数据指向 profile。

### 2.1 目录名本身

| 引用点 | 位置 | 复制/重命名动作 |
|:---|:---|:---|
| `$DSH_HOME/profiles/<名>/` | dsh `resolveProfileDir`（`dsh-app-boot/lib/index.js` @ 11826） | 复制 = 新建 `profiles/<新名>/` 并拷贝内容；重命名 = 把目录改名（同文件系统 rename 原子） |

**这是唯一硬身份**。复制/重命名的核心就是操纵这个目录。

### 2.2 profile 目录内部

| 引用点 | 位置 | 复制/重命名动作 |
|:---|:---|:---|
| `package.json` 的 `name: "dsh-profile-<名>"` | `initProfile` 写入（`dsh-app-boot/lib/index.js` @ 13418）；`readProfileManifest` 消费 | **重命名必须同步改写**为 `dsh-profile-<新名>`。这是 dsh 自身唯一写入 `dsh-profile-` 前缀字段的位置，**无外部消费处**（全 dsh 包树 grep `dsh-profile-` 仅 initProfile 一处写入）。改写为一致性保持（与目录名对应），不做则会漂移。 |
| `package.json` 的 `dependencies` | pnpm 写入 + `reconcilePlugins` 回写 | 不引用 profile 名本身，照搬即可 |
| `dsh.profile.bundles` | `reconcilePlugins` 回写 | 不引用 profile 名，照搬即可 |
| `cordis.patch.yml` | 用户层 patch（`PROFILE_PATCH_FILENAME` = `cordis.patch.yml`） | **可能含路径引用！** 若 patch 里有 `../其他profile/...` 或绝对路径，跨目录相对引用会断。**但 patch 语义不含 profile 自身名**（patch 是模块 id 定位，非名字定位）。仅当 patch 内含相对路径（如 `../foo`）时，复制/重命名后相对引用可能错——**照搬时需人工检查 patch 里的路径引用**。 |
| `pnpm-workspace.yaml` | `initProfile` 写入（`nodeLinker: hoisted` / `packages: [.]`） | 不引用 profile 名，照搬即可 |
| `node_modules/`（pnpm 安装的模块） | pnpm 管理 | **重命名后不能直接搬**——pnpm 的 `node_modules` 内有 `.pnpm` 虚拟 store 和相对路径链接，重命名后链接可能断。**但 dsh 有兜底**（见 §2.3 符号农场）。**最安全做法：重命名后让 dsh 下次启动自愈，或壳侧删 `node_modules` 让 pnpm 重装**。 |

### 2.3 `$DSH_HOME/profiles/node_modules` 符号链接农场（跨 profile 全局）

| 引用点 | 位置 | 复制/重命名动作 |
|:---|:---|:---|
| `$DSH_HOME/profiles/node_modules/`（每个包一个符号链接） | `healProfilesModuleFallback`（`dsh-app-boot/lib/index.js` @ 409） | **不引用任何 profile 名**。链接集 = dsh app 自身依赖闭包（BFS 自 `@deepseek-ai/dsh` 自身的 `package.json` dependencies + peerDependencies 递归），**与 profile 的 bundle 无关**。复制/重命名 profile 不改变这个闭包 → 农场**不受影响**。 |

**关键修正（本次 spike 最重要的发现）**：roadmap 原先的说法「复制/重命名 profile 后的新依赖闭包由 dsh 下次启动自愈」**基于一个错误前提**——农场链接集的构成与 profile 无关。正确的表述是：**农场链接只随 dsh app 自身安装的包变化**；profile 复制/重命名后：
- 若新 profile 引用的插件包在 dsh app 闭包内（`dsh-base`及其依赖）→ 农场已覆盖，**立即可用**；
- 若新 profile 引用的插件包**不在** dsh app 闭包内（如第三方 `.cordis.yml` overlay 引入的包）→ **农场不提供该包的链接**，须由 pnpm 安装（profile 自己的 `node_modules`）解决，农场不参与。

农场幂等语义（源码同处注释）：正确链接保持、移动的安装重新指正；**stale 链接（指向已消失包）会保留直到该包名被复用**——悬空链接对模块解析不可见。

### 2.4 `$DSH_HOME` 下的其他文件

| 引用点 | 位置 | 复制/重命名动作 |
|:---|:---|:---|
| `$DSH_HOME/settings.json` | `dsh-settings-file` 模块 | **不引用 profile 名**（设置按命名空间组织，profile 选择是运行时状态） |
| `$DSH_HOME/.credentials.yaml` | `dsh-credentials-local` 模块 | 不引用 profile 名（全局凭据） |
| `$DSH_HOME/sessions/` | `dsh-session-persistence-jsonl` | **可能引用 profile 名**（会话记录可能含 boot 时的 profile）。dsh 明示「删除 profile 不删除会话等全局数据」——故删除 profile 时**不得级联删除 sessions**，但查询 session 时须容忍「其 profile 已不存在」的悬空（显示为「profile 已删除」） |
| `$DSH_HOME/.agent-presets/` | agent-presets 模块 | 与 profile 创建无关（roadmap §1 已核实），不引用 profile 名 |

### 2.5 WSL 客体内引用（本 spike 扩围）

| 引用点 | 位置 | 复制/重命名动作 |
|:---|:---|:---|
| `executor.rs` `GUEST_BOOT` 脚本（`dsh --profile web --port 0 --no-open`） | `executor.rs:447` | **写死 `web`**，迭代 v1 固定 boot web profile（executor.rs:595 注释「迭代 v1：WSL 内固定 boot web profile」）。复制/重命名其他 profile 不影响它；但**默认启动 profile 切换（4.3④）与 WSL 分支合并评估**时，此处须同步放开 |
| 客体内 `$HOME/.dsh/profiles/`（若 WSL 客体内有独立 `$DSH_HOME`） | 客体内 dsh 默认 home | **壳侧不可直接操纵**（客体内文件系统跨边界，壳只做 WSL 命令；重命名/删除客体内 profile 需经 WSL 命令或仅提示用户在客体内操作） |

### 2.6 壳自身对 profile 的引用（本仓库代码）

| 引用点 | 位置 | 复制/重命名动作 |
|:---|:---|:---|
| `resolve.rs` `list_web_ui_profiles(home)`（扫描 `profiles/*/package.json` 的 `dsh.profile.bundles` 是否有 webUi） | `resolve.rs:763` | 纯读，无状态引用；复制/重命名后下次扫描自然反映 |
| `shell.rs` `spawn_dsh`（`launch.profile` 用于 `--profile` 参数） | `shell.rs:70` | 运行时引用，非持久引用 |
| `settings.json` 的 `defaultMode` | `settings.rs` | 当前仅 `defaultMode`；4.3④ 将新增 `defaultProfile` 字段（第二最小面例外）——**该字段一旦引入就变成一个新的持久引用面**：删除 profile 时须引用检查（若 defaultProfile 指向被删 profile → 回退 `web` 或清除） |
| 前端选择器（`choose_profile` 的 `NeedsProfile` 分支） | `lib.rs` | 运行时引用，非持久 |

---

## 3. 结论：对复制/重命名/删除的设计指引

### 3.1 重命名 profile

| 动作 | 必做 | 可选/不做 |
|:---|:---|:---|
| `profiles/<旧名>/` → `profiles/<新名>/`（目录 rename） | ✅ 必须（同文件系统内原子） | — |
| `package.json` 的 `name` 字段改写为 `dsh-profile-<新名>` | ✅ 必须（一致性与 dsh 约定） | — |
| `node_modules/` 处理 | 见下方「node_modules 处理」 | — |
| `cordis.patch.yml` 内部相对路径引用 | 需人工检查 | 若有 `../` 引用需同步改写 |
| `profiles/node_modules` 农场 | **不做**（与 profile 名无关，见 §2.3） | — |
| 会话引用 | **不做**（会话容忍悬空 profile 引用） | — |

**node_modules 处理**（选择：删 vs 搬）：
- **删**（推荐）：删除 `node_modules/`，让 dsh 下次启动时 resolveProfile 时由 pnpm/linkFarm 兜底，或提示用户重跑 `dsh plugin --profile <新名>` 重装。**简单、无断链风险**。
- **搬**：`mv node_modules` 在同文件系统内可行，但 pnpm 虚拟 store 的硬链接/相对路径可能指向旧路径——**不推荐**，除非测试验证。
- 实机验证建议：重命名后先删 `node_modules`，跑 `dsh --profile <新名> --dump-config` 确认启动正常。

### 3.2 复制 profile

| 动作 | 必做 | 可选/不做 |
|:---|:---|:---|
| 拷贝 `profiles/<源名>/` 内容到 `profiles/<新名>/` | ✅ 必须（排除 `node_modules/`） | — |
| `package.json` 的 `name` 字段改写为 `dsh-profile-<新名>` | ✅ 必须 | — |
| `node_modules/` | **排除不拷**（让 pnpm 在新目录安装，避免旧链接） | — |
| `dsh.profile.bundles` | 照搬（不引用 profile 名） | — |
| `cordis.patch.yml` | 照搬（intact，不改写） | 人工检查相对路径引用 |
| `profiles/node_modules` 农场 | 不做（全局共享） | — |

### 3.3 删除 profile

| 动作 | 必做 | 可选/不做 |
|:---|:---|:---|
| 删除 `profiles/<名>/` | ✅ 必须 | — |
| 删除前确认（明示「不删除会话等全局数据」） | ✅ 必须（roadmap 4.3③ 明示） | — |
| `settings.json` 的 `defaultProfile` 引用检查 | ✅ 必须（引入该字段后；引用被删 profile → 回退 `web` 或清除） | — |
| 级联删除 sessions | ❌ **禁止**（dsh 明示不级联） | — |
| `profiles/node_modules` 农场的残留链接 | 不做（stale 链接对模块解析不可见，且 dsh 幂等） | — |
| WSL 客体内同名 profile | 不做（客体内隔离，提示用户操作） | — |

---

## 4. 关键事实与源码锚定表（供 ADR 引用）

| 事实 | 锚定 |
|:---|:---|
| profile 目录是唯一硬身份 | `resolveProfileDir`（`app-boot/lib/index.js` @ 11826） |
| initProfile 写的 `name: dsh-profile-<basename(dir)>` | `app-boot/lib/index.js` @ 13418 |
| `dsh-profile-` 前缀字段无外部消费处 | 全 dsh 包树 grep 仅 initProfile 一处写入 |
| 农场链接集 = dsh app 依赖闭包,与 profile 无关 | `healProfilesModuleFallback`（`app-boot/lib/index.js` @ 409）——BFS 自 `installAnchor` 的 deps + peerDeps |
| 农场幂等:stale 链接保留但不可见 | 同函数自注释「stale link to a vanished package stays until its name is reused」 |
| patch 文件 = `cordis.patch.yml`（用户层,id 定位） | `PROFILE_PATCH_FILENAME`（`app-boot/lib/index.js` @ 311） |
| WSL GUEST_BOOT 写死 `web` | `executor.rs:447`(dsh-dock 仓库) |
| 壳已有 profile 扫描 | `list_web_ui_profiles`（`resolve.rs:763`,dsh-dock 仓库） |
| 会话存储默认 `$DSH_HOME/sessions/` | `dsh-session-persistence-jsonl` 模块 |
| home 解析优先级:配置路径 > `$DSH_HOME` > `~/.dsh` | `resolveDshHome`（`dsh-home-paths/lib/index.js` @ 73） |

---

## 5. 遗留事项

- [ ] 重命名后 node_modules 处理方式实机验证（删 vs 搬）——建议 4.3 实现时以「删 + dsh 下次启动自愈」为第一方案,实机验证通过再考虑搬。
- [ ] `cordis.patch.yml` 内相对路径引用在复制/重命名后的断链风险实机验证(样本:含 `../foo` 的 patch)。
- [ ] WSL 客体内 profile 的操纵边界(壳是否经 wsl.exe 提供「客体内重命名」UI)留给 4.9 WSL v2 评估。
