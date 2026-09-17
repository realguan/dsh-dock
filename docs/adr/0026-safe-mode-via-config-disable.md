# ADR-0026：安全模式 = 在配置里停用三方插件（+ 覆写前备份 / 一键恢复）

- **状态**：**已采纳**（2026-09-16 维护者裁定，**取代 ADR-0025 的机制**：临时 `--patch` overlay 退役）
- **日期**：2026-09-16
- **相关**：ADR-0025（旧机制，保留决策史）、ADR-0012（启动失败类型化）、ADR-0020（实验能力开关）、
  ADR-0009（patch 写入例外与 `PatchFile`）、ADR-0015（生命线）
- **上游锚点**：`/Users/guan/git/deepseek-harness` @ `0d1f50007f9bca3f52b06e1c3074fa14d5fb0720`

---

## 1. 背景与问题

ADR-0025 的安全模式用**壳自有目录里的临时 overlay**（`dsh --patch <overlay>`）停用插件行，
好处是零文件改动、原子回退；代价是**两个真相源**：配置层（`cordis.patch.yml`）说"已启用"，
运行态说"已停用"。维护者真机看到的正是自相矛盾的界面（开关全开、徽标却是「已停用」），
只能靠一条横幅去解释——**解释界面本身就是设计缺陷的信号**。

维护者 2026-09-16 的裁定（第二版口径，原话）：

> 「我的理解所谓安全模式应该是在配置文件里面把所有三方插件都设置为 disable 就行了，这样就能
> 正常启动，后面可以有用户自己选择启动哪个插件，我们做好用户提醒这些用户体验就好了。」
> 「可以一键恢复，就是把备份好的配置文件覆盖回去。」

即：**配置就是状态**。用户进应用后按已有开关逐个启用（现有「实验能力」面板的开关本就写这个
文件），恢复 = 把进入前那份备份覆盖回去。

## 2. 一并推翻的旧结论（实测记录）

ADR-0025 §4 曾记「三方 **bundle 层**行停不掉：真机停 33 行 → `exit 1`」。2026-09-16 复测
（克隆体 `~/.dsh-dock-dev`，未动现场）：

| 实验（改的是**配置**：在 `cordis.patch.yml` 里写 `- id: x` + `disabled: true`） | 结果 |
|:--|:--|
| 只停 **9 条三方行**（5 用户 patch 行 + 4 三方 bundle 行：`agent-team` / `tool-agent-team` / `ui-agent-team` / `auto-review`） | **8.2s 正常就绪** ✅ |
| 再**多停一条随包行** `tools` | `exit 1`，`required startup failure: 1 entry did not activate` + `agent-loop: pending (waiting for service: tools)` |
| 同一 9 行改用 `--patch` overlay 停 | 7.9s 正常就绪（机制等价，差异只在"改不改配置"） |
| 对照：原样启动（坏行在位） | `exit 1`，`ERR_MODULE_NOT_FOUND`（37.5s） |

**结论**：三方 bundle 层行**可以**停；当年那次 `exit 1` 的真因是**判据过宽**——段落标签形如
`@deepseek-ai/dsh-base, patched by …/cordis.patch.yml`，按"标签里含 `.yml` 即用户行"会把
**被用户 patch 过的随包行**也算进停用集合，于是 `tools` 这类随包服务行被停，依赖它的
`agent-loop` 悬空。故本 ADR 的判据取**段落主段**（`, patched by` 之前）。

## 3. 决策

**安全模式（进入）**：把 profile 的 `cordis.patch.yml` 里**所有非随包行**写成
`disabled: true`（一次覆写、一次备份），随后**正常启动**。

- 停用判据（纯函数、单源）：`safe_mode::should_disable(section)` =
  `primary_section(section) ∉ {@deepseek-ai/dsh-base, @deepseek-ai/dsh-web-app}`；
  用户 patch 行的主段是**文件路径**、三方 bundle 行是**包名**，两者都停；随包行**永不停**。
- 写入复用既有内核 `plugins.rs::PatchFile` + `apply_disabled_toggle`（未改条目**原文保真**
  含行间注释；幂等：已是停用态则零写入）＋ `fs_backup::backup_before_overwrite_path`
  （覆写前备份，fail-closed）。

**一键恢复（退出）**：按**记账**里记下的那份备份，逐字节覆盖回 `cordis.patch.yml`
（`atomic_replace`，不重新解析——配置文件已写坏也能恢复）。覆盖前把**当前**配置再备份一份
（安全模式期间的改动不会无迹可寻）。

**记账**（壳自有 `<app_data>/safe-mode/<profile>.json`）：`{disabled_rows, patch_backup, applied_at}`。
它是"该恢复回哪一份"的唯一依据——**不按文件名猜最新一份**（用户进入安全模式后又改过插件时，
"最新一份"是安全模式之后的状态）。

**语义边界（写清给后续维护者）**：一键恢复 = **撤销最近一次安全模式写入**。若用户进入安全模式、
在面板上又打开过若干插件、随后再点一次"停用全部三方插件并启动"（第二次进入），记账会指向**第二次
之前**那份备份——恢复回到的是"第二次进入前"的状态，而不是最初那份。更早的备份仍在同目录以
`.bak-<时间戳>` 形式留档，可手动取用；这是"每次进入各自快照"的一致语义，不是缺陷。`restorable` = 那份备份是否还在；不在 ⇒ 前端**不渲染**
恢复按钮，只如实说明（宁可不给按钮，也不给一个点了必然报错的按钮）。

**路径校验**：恢复是破坏性动作，记账里的备份路径必须是**同目录的 `cordis.patch.yml.bak-*`**，
否则拒绝覆盖。

**退役**：`--patch` 注入、overlay 读写、`LaunchSpec.patch_overlay` 全部删除；旧的
`<app_data>/safe-mode/<profile>.yml` 在进入/恢复时顺手清理。独立复核曾指出：`--patch`
一旦被别处误加（2026-09-16 就发生过 `error: unknown option '--patch'`），症状是"按钮点了没反应"
或"dsh 秒退"；现在这条路径**不存在**，回归锚 = `shell::tests::launcher_args_never_pass_patch_…`。

## 4. 后果

**正面的**：

- 单一真相源：开关、徽标、下次启动、错误卡全部读同一份配置；此前那条"解释两个真相源"的横幅与
  面板说明**直接删除**（少一处文案 = 少一处误导）。
- 用户可自助：进应用后在「实验能力」面板逐个打开即可（开关本就写这个文件），不必理解 overlay。
- 恢复语义可解释给用户：**"把备份的配置覆盖回去"**——一句话，且备份文件名（`.bak-<时间戳>`）
  在现有 UI 里已是被认知的资产。

**负面的 / 代价**：

- 安全模式**会改用户的配置文件**（ADR-0025 的"零文件改动"卖点消失）。缓解：覆写前备份
  （fail-closed）+ 一键恢复 + 恢复前再备份当前态。
- 恢复会**覆盖**进入安全模式之后对该配置的改动（维护者明确接受；那份改动也会先备份）。
- WSL 客体档仍不支持（缺客体侧写原语，与本条同口径：**显式报错，不回落宿主**）。

## 5. 独立复核后的加固（同日，第三方子 agent 反方核查）

复核确认了机制（枚举判据、进入/恢复、单一真相源与回开路径、旧机制拆除），并抓到四处需要加固：

1. **档位意识（P1，真缺陷）**：原实现固定用 `resolve::user_dsh_home()`，而**快照档**的 home 是
   `<data_dir>/runtimes/fallback-home`（每次启动由 `sync_fallback_home` 重同步覆写）。按用户 home
   写会**改错文件**（动用户的 `~/.dsh` 同名 profile）且不可能生效——旧机制本有档位意识（`--patch`
   只在引擎档传），换代时丢了。修：`Executor::dsh_home()` 在 spawn 时记账（`ShellState::last_boot_home`），
   `terminal_action` 用它；**与用户 home 不同即显式拒绝**（口径同 WSL：宁可报错，不回落宿主）。
2. **记账按 home 键（P2）**：dev / 正式档共用同一份壳数据目录，只按 profile 名记账会让另一侧显示
   "安全模式中"并给一个必然失败的恢复按钮。修：`Journal.dsh_home` + `state()/exit()` 归属校验
   （不属于本 home → 视为未启用）；进入时若发现**别的 home** 的记账，先归档为
   `<profile>.json.other-home-<ts>`，不静默抹掉那一侧的恢复指针。
3. **写坏配置的风险（P3，内核既有缺陷落在救援路径）**：`PatchFile::render` 逐字拼接原文片段，
   而末段来自 `body[s..len]`——**文件末行无换行**时追加 `- id: …` 会拼成 `name: '@x'- id: X`
   （`Nested mappings are not allowed in compact mappings`）。修：① 每个原文片段补行尾换行；
   ② 新增 `render_checked()`（渲染后**读回解析一次**再写，fail-closed），`write_with_backup` 一律走它。
   复现先行：`append_after_a_file_without_trailing_newline_stays_parsable`（撤掉修复即红，已验）。
4. **判据的两处口径修正**：① 随包 bundle 取**全表 6 个**（上游 `PROFILE_TEMPLATES` + 安装期
   owned tuple：base / web-app / headless / acp-app / sdk-app / sdk-minimal）——只列 2 个时，
   同时含 web-app 与 headless 的清单会把 headless 行误停；② **无归属行（`None`）改为"不停"**：
   它只在 dump 格式漂移时出现，那时"当成用户行"会连随包行一起停 ⇒ 主动制造 `exit 1`；
   反方向最多是"没救到那几行"，由空计划诚实门报错。**宁可不救，不可救坏**。
5. **层序边界（已记档，未实现）**：profile 层的停用桩停不到 **home 层**
   （`$DSH_HOME/cordis.patch.yml`，层序更晚）的 insert 行。现在 `split_by_layer` 会把这部分
   如实归入"够不到"并在提示里点名（不再报"已全部停用"）；**根治要把桩写进 home 层文件**
   （两个文件、两份备份、恢复要还两份），触发条件 = 用户报"安全模式没救到某个插件"且其
   `$DSH_HOME/cordis.patch.yml` 里确有 insert 行（本机无该文件）。

## 6. 复审条件

- 上游提供官方"安全模式 / 跳过用户层启动"开关 → 本方案整体退役；
- `--dump-*` 的**段落标签格式**变更（判据依赖 `, patched by` 与主段是路径/包名）→ 枚举复评；
- 「实验能力」开关改为不再写 `cordis.patch.yml`（例如上游给官方禁用原语）→ 恢复动作可简化为
  调用该原语，本文的写入纪律随之复评。
