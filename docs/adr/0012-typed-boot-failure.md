# ADR-0012：启动失败错误类型化（`BootFailure`），子串分类降级为兜底

- **日期**：2026-09-08
- **状态**：草案（待维护者评审）
- **提出人**：guan（AI 起草，依据 `docs/architecture-review-2026-09-08.md` P5）
- **相关方**：`src-tauri/src/lib.rs`（boot 路径与错误卡）、`frontend/src/components/boot/ErrorCard.tsx`、
  `frontend/src/types/ipc.ts`、`docs/contract.md`（若 IPC 错误形状变化需同步）
- **关联**：架构评审 P5；`lib.rs::classify_boot_error`（子串分类）、`lib.rs::read_error_detail`（日志刮取）；
  先例：`ClientUpdate` 已用 tagged enum（`updates.rs`）

---

## 1. 背景与问题

启动失败时，壳要把「发生了什么 + 怎么办」呈现给用户。今天的判定链是：

1. 各路径返回 `Result<T, String>`（全仓约 110 处）；
2. `classify_boot_error(detail: &str)`（`lib.rs:2199`）把错误文本**小写后做子串匹配**，
   决定错误卡标题、建议与按钮集合：

```rust
if d.contains("credentials") || d.contains("must be a string") { /* 凭据格式不匹配 */ }
else if d.contains("unknown option") || d.contains("incompatible") { /* 参数不兼容 */ }
else if d.contains("network") || d.contains("registry") || d.contains("timeout") { /* 网络不可用 */ }
else { /* 兜底：启动失败 */ }
```

3. `read_error_detail`（`lib.rs:2229`）再从日志里刮 `Error:` 行当补充说明。

**问题**（三条，均可复现）：

- **措辞即契约**：上游（dsh / pnpm / node / OS）改一句文案，分类就静默落到兜底分支——
  用户从「升级即可解决」变成「详情见日志」，无任何测试能发现；
- **同词不同因**：任何含 `timeout` 的错误（含本地 socket 超时、用户取消）都会被判成
  「网络不可用」，给出的建议是错的；
- **不可穷尽**：字符串没有穷尽匹配，新增失败模式不会触发编译错误。

## 2. 约束与硬指标

1. **零新依赖**（AGENTS §5：不引测试依赖；此处同样不引 `thiserror`/`anyhow` 之外的运行时依赖）。
2. **增量**（AGENTS §8.2）：不得一次性把 110 处 `Result<_, String>` 全改——本 ADR 只覆盖
   **boot 失败路径**（错误卡的唯一消费方）。
3. **契约同步**（AGENTS §4.1）：错误形状若跨 IPC 变化，须同步 `docs/contract.md` 与
   `frontend/src/types/ipc-shapes.json`（形状闸门已存在）。
4. **旧路径不退化**：外部进程（dsh/pnpm/node）返回的仍是自由文本，必须有兜底分类，
   且兜底行为与今天一致（否则等于用「类型化」之名悄悄改变用户可见文案）。
5. **可测**：分类必须能被离线单测穷尽覆盖（fixture 内联）。

## 3. 备选方案及评估

### 方案 A：`BootFailure` 枚举 + 子串分类降级为兜底 —— ✅ 建议采纳

- 思路：定义 `#[derive(Serialize)] #[serde(tag = "kind", rename_all = "snake_case")] enum BootFailure`
  （`CredentialsMismatch` / `IncompatibleOptions` / `NetworkUnavailable` / `EngineNotReady` /
  `Unknown { detail: String }`）；**内部**产生错误的地方直接返回枚举；跨 IPC 时序列化为
  `{ kind, detail? }`。`classify_boot_error` 保留，改为 `BootFailure::from_legacy_detail(&str)`，
  只服务「无法类型化的外部文本」，映射表逐条入测。
- 优点：措辞变化不再影响分类；新增失败模式触发编译错误；建议/按钮集合可穷尽匹配；
  前端错误卡只需把「id → 文案」表的输入从字符串换成 `kind`。
- 代价/风险：错误卡前端需同时兼容「结构化对象」与「字符串」两种拒绝值（一次性）。
- 对照约束：①零新依赖（手写枚举 + serde，已有依赖）；②只动 boot 路径；③形状入
  `ipc-shapes.json`；④兜底表与现状逐条对齐并有测试；⑤`from_legacy_detail` 纯函数可穷尽测。

### 方案 B：保持字符串，强化分类器（正则/优先级表） —— ❌ 否决

- 思路：把子串匹配换成更精细的规则表。
- 否决理由：违反约束 4 的精神——**措辞仍即契约**，上游改文案依旧静默失效；同词不同因问题
  不解决（`timeout` 仍无法区分来源）。治标。

### 方案 C：全仓 `Result<_, String>` 一次性换成领域错误枚举 —— ❌ 否决

- 思路：110 处全改，每模块一个错误类型。
- 否决理由：违反约束 2（一次性大改）；与本次评审结论「先做能封住高危面的最小改动」相悖；
  改动面横跨全部模块，回归无法界定。

### 方案 D：引入 `thiserror` / `specta` 生成前端类型 —— ❌ 否决

- 思路：用现成库减少样板 / 直接生成 TS 类型。
- 否决理由：违反约束 1（新依赖）；`specta` 另需 ADR 且与既有 `ipc-shapes.json` 闸门重复
  （评审 §4「不建议做」已明确）。

## 4. 最终决策

**采用方案 A**：只为 boot 失败路径引入 `BootFailure` 枚举（含 `Unknown { detail }` 兜底），
`classify_boot_error` 改造成 `BootFailure::from_legacy_detail`，其映射表与今天的子串规则
**逐条等价**并纳入离线单测；前端错误卡按 `kind` 取文案、对非结构化拒绝值回退 `String(e)`。
其余模块的 `Result<_, String>` 本次不动。

## 5. 后果与后续行动项

### 正面后果
- 分类不再依赖上游措辞；新增失败模式触发编译错误。
- 错误卡文案/按钮集合可穷尽匹配，且能被 fixture 单测钉住。
- 为将来「把更多模块纳入类型化」提供可复制的样板（枚举 + `Unknown` 兜底 + 形状闸门）。

### 负面后果 / 新增债务
- IPC 拒绝值出现两种形态（结构化对象 / 字符串），前端需一次性兼容分支；兼容层若长期存在
  会成为「两套错误语义」的债——复审条件里已设清理触发。
- `Unknown { detail }` 仍会把自由文本带进前端（兜底本质如此），只是不再参与分类决策。

### 行动项

- [ ] 定义 `BootFailure` + `from_legacy_detail` 纯函数；把今天的四条子串规则写成**等价映射表**
      并补穷尽单测（含「同词不同因」反例：本地 socket 超时不得判成网络）。（负责人待定）
- [ ] boot 路径的错误产生点改返回 `BootFailure`（先做 `classify_boot_error` 的三个调用方）。
- [ ] 前端 `ErrorCard` 增加结构化分支；`types/ipc.ts` 加 `BootFailure` 形状并登记
      `frontend/src/types/ipc-shapes.json`（形状闸门自动生效）。
- [ ] 若跨 IPC 形状变化涉及契约，同步 `docs/contract.md` 并升 `MANIFEST_FORMAT`。
- [ ] AGENTS §9 索引补本 ADR 一行；`docs/known-issues` 的 P5 条目回收。

## 6. 复审条件

- 上游 dsh/pnpm 的启动错误文案发生结构性变化（兜底表命中率下降）→ 重评兜底策略；
- 前端兼容分支存在超过 2 个版本仍有 `String(e)` 拒绝值 → 强制收敛为单一形态；
- 出现第三种需要分类的错误来源（如网络面之外的第三方进程）→ 重评枚举边界。
