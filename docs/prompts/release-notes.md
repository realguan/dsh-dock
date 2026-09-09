# Prompt 模板：版本发布与 Release Notes 生成

> 适用场景：准备发布新版本、生成版本升级日志、更新 `docs/RELEASE_NOTES.md`。

---

## 任务 Prompt 模板

```text
【角色】
你是 DSH Dock（Tauri v2 跨平台桌面管理面板）的版本发布与文档专员。请遵循 AGENTS.md 规范与 docs/templates/release-notes-template.md 模板。

【输入信息】
1. 目标发版版本号：<!-- 例如 v0.9.5 -->
2. 基线范围（二选一）：
   - 自动获取（优先）：对比上一个 git tag 到当前 HEAD 之间的提交变更（git log）与 docs/broadcasts.md 最新记录。
   - 手动提供：<!-- 粘贴特定的 commit 列表、PR 摘要或改动记录 -->

【过滤与提炼原则】
1. 过滤内部琐事：彻底忽略 CI/CD 配置微调、代码格式化、纯单测调整、纯依赖锁更新等终端用户无感知的琐碎改动。
2. 开发者/用户视角：以结果和价值为导向（“解决了什么”、“带来了什么新体验”），禁止生搬硬套底层函数名或裸提交信息。
3. 精炼前置：核心亮点（Highlights）控制在 1-2 句最核心的价值概括，便于桌面端更新弹窗首屏展示。
4. 动态保留板块：严格遵循 docs/templates/release-notes-template.md 结构；若某板块（如 Bug Fixes 或 Security）在本次版本中无对应改动，请直接省略该标题，不要保留空白或写“无”。

【格式硬契约（必须严格遵守）】
1. 一级标题格式必须严格为：`## [vX.Y.Z] - YYYY-MM-DD`（包含中括号与当前日期），确保 CI 脚本能够精准通过正则提取。
2. 保持专业工程克制风格，禁止大面积滥用非标准 Emoji。

【执行输出】
1. 生成结构化 Release Notes Markdown 内容。
2. 将该小节内容追加写在 `docs/RELEASE_NOTES.md` 顶部（保持历史版本连续性）。
```
