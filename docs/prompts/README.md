# Prompt 模板库（团队共享资产）

> 目标：让任何成员用任何 AI 工具，产出风格一致、符合仓库约束的结果。
> 模板是工具中立的——它们只是结构化的任务输入，粘贴进 Claude Code / Copilot /
> Aider / Cursor / Windsurf 或任何对话窗口均可。

## 使用规则

1. **先填空再发送**。模板里的 `<!-- ... -->` 是必填空位，空着就发 = 让 AI 猜 = 返工。
2. **配合 AGENTS.md**。本仓库的 AI 会话应已加载根目录 `AGENTS.md`；若你的工具不自动
   读它，把「请先读 AGENTS.md 再动手」写进任务开头。
3. **一次一个模板**。不要在一条消息里塞两个场景的任务。
4. **结果仍走流程**。AI 的产出按 `docs/CONTRIBUTING.md` §6 纪律处理：
   人肉读 diff、补测试、规范提交。模板降低沟通成本，不豁免人的责任。
5. **改进模板走 PR**。模板是仓库资产，改完在 commit 里说明踩了什么坑。

## 模板索引

| 文件 | 场景 | 什么时候用 |
| :--- | :--- | :--- |
| [feature.md](./feature.md) | 新功能实现 | 从需求到可验收代码 |
| [code-review.md](./code-review.md) | AI 辅助代码审查 | 提交前自审 / review 前预检 |
| [bugfix.md](./bugfix.md) | Bug 排查 | 有报错/异常行为需要定位 |
| [refactor.md](./refactor.md) | 受限重构 | 改结构不改行为 |

## 快速示例（以 bugfix.md 为例）

```text
【角色】你是本仓库（dsh-dock，Tauri v2 Rust 壳）的资深排查者。先读 AGENTS.md。

【现象】Windows 11 + WSL2(Ubuntu 22.04)：点「在 WSL 中打开」后启动页卡在第 3 步，
约 20 秒后报 Stalled。macOS 正常。

【错误信息】
（粘贴 dsh-shell.log 尾部 50 行）

【已尝试】重装 WSL 发行版无效；local 模式正常。

【约束】只许改 src-tauri/src/executor.rs 与 shell.rs；禁止触网、禁止动契约；
先给出 3 个候选根因与验证方法，等我确认后再改代码。
```

> 完整模板见各文件。共同结构：**角色 → 输入材料 → 硬约束 → 输出要求 → 验收标准**，
> 五段缺一不可——少了约束段，AI 会自由发挥；少了验收段，你无法判断它做没做完。
