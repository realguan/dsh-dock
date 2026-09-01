# Prompt 模板：版本发布与 Release Notes 生成

> 适用场景：准备发布新版本、生成版本升级日志、更新 `docs/RELEASE_NOTES.md`。

---

## 任务 Prompt 模板

```text
【角色】你是 DSH Dock（Tauri v2 桌面管理面板）的版本发布与文档专员。请遵循 AGENTS.md 规范与 docs/templates/release-notes-template.md 模板。

【输入材料】
1. 目标版本号：<!-- 例如 v0.9.1 -->
2. 本次变更的 Commit 列表 / 功能清单 / Issue 修复记录：
<!-- 粘贴 git log、Issue 列表或主要改动摘要 -->

【硬约束】
1. 严格使用 docs/templates/release-notes-template.md 标准结构（包含核心亮点、新增功能、体验优化、缺陷修复、安全稳定性五大板块）。
2. 文案必须客观清晰、面向终端开发者与用户，突出功能价值与操作体验提升。
3. 遵循前端品牌规范：文档与 UI 文案严禁杂乱的 emoji 或非标准符号，保持严谨工程风格。
4. 将生成的版本日志追加记录到 `docs/RELEASE_NOTES.md` 顶部。

【输出要求】
1. 输出符合规范的 Markdown 格式 Release Notes。
2. 确认已更新 `docs/RELEASE_NOTES.md` 并保持历史版本的连续性。
```
