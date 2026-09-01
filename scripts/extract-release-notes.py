#!/usr/bin/env python3
"""
extract-release-notes.py
从 docs/broadcasts.md 或 git 历史自动提取当前发布版本的详细 Release Notes。
供 GitHub Actions CI 与 Tauri latest.json 自更新清单无缝集成。
"""

import os
import re
import sys
import subprocess

def extract_from_release_notes(tag_name: str, release_notes_path: str) -> str:
    if not os.path.isfile(release_notes_path):
        return ""

    with open(release_notes_path, "r", encoding="utf-8") as f:
        content = f.read()

    clean_ver = tag_name.lstrip("v")
    # 按照 '## ' 分割各个版本条目
    sections = re.split(r'\n(?=## )', content)

    for sec in sections:
        if not sec.startswith("## "):
            continue
        header_line = sec.split("\n", 1)[0]
        # 匹配标题中的版本号如 ## [v0.9.1] 或 ## [0.9.1]
        if f"[{clean_ver}]" in header_line or f"[v{clean_ver}]" in header_line or clean_ver in header_line:
            # 返回该版本的整节内容
            return sec.strip()

    return ""

def extract_from_broadcasts(tag_name: str, broadcasts_path: str) -> str:
    if not os.path.isfile(broadcasts_path):
        return ""

    with open(broadcasts_path, "r", encoding="utf-8") as f:
        content = f.read()

    clean_ver = tag_name.lstrip("v")
    sections = re.split(r'\n(?=### )', content)
    
    matched_section = None
    for sec in sections:
        if not sec.startswith("### "):
            continue
        header_line = sec.split("\n", 1)[0]
        if f"v{clean_ver}" in header_line or f" {clean_ver} " in header_line or clean_ver in header_line:
            matched_section = sec
            break

    if not matched_section:
        for sec in sections:
            if sec.startswith("### ") and ("完成通知" in sec or "发布" in sec or "升级" in sec or "v0." in sec):
                matched_section = sec
                break

    if not matched_section:
        return ""

    lines = matched_section.strip().splitlines()
    title = lines[0].lstrip("#").strip()
    body_lines = lines[1:]
    clean_body = "\n".join(body_lines).strip()
    return f"## {title}\n\n{clean_body}"

def extract_from_git() -> str:
    try:
        cmd = ["git", "log", "-n", "10", "--pretty=format:- %s (%h)"]
        out = subprocess.check_output(cmd, text=True, stderr=subprocess.DEVNULL)
        if out.strip():
            return "## 变更历史\n\n" + out.strip()
    except Exception:
        pass
    return ""

def main():
    tag_name = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("GITHUB_REF_NAME", "v0.9.2")
    output_path = sys.argv[2] if len(sys.argv) > 2 else "release_notes.md"
    
    docs_dir = os.path.join(os.path.dirname(__file__), "..", "docs")
    release_notes_path = os.path.join(docs_dir, "RELEASE_NOTES.md")
    broadcasts_path = os.path.join(docs_dir, "broadcasts.md")

    # 优先级：docs/RELEASE_NOTES.md -> docs/broadcasts.md -> git log
    notes = extract_from_release_notes(tag_name, release_notes_path)
    if not notes:
        notes = extract_from_broadcasts(tag_name, broadcasts_path)
    if not notes:
        notes = extract_from_git()
    if not notes:
        notes = f"DSH Dock {tag_name} 正式发布，包含性能优化、稳定性修复与功能升级。"

    with open(output_path, "w", encoding="utf-8") as f:
        f.write(notes + "\n")

    print(f"✅ Release notes extracted ({len(notes)} chars) -> {output_path}")
    print("--- PREVIEW ---")
    print(notes[:500] + ("..." if len(notes) > 500 else ""))
    print("---------------")

if __name__ == "__main__":
    main()
