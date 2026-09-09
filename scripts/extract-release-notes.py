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
    pattern = rf'\[v?{re.escape(clean_ver)}\]'

    for sec in sections:
        if not sec.startswith("## "):
            continue
        header_line = sec.split("\n", 1)[0]
        # 精确匹配标题中的版本号如 ## [v0.9.1] 或 ## [0.9.1]（防 0.9.1 误伤 0.9.10 等子串）
        if re.search(pattern, header_line):
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
    pattern = rf'\bv?{re.escape(clean_ver)}\b'
    for sec in sections:
        if not sec.startswith("### "):
            continue
        header_line = sec.split("\n", 1)[0]
        if re.search(pattern, header_line):
            matched_section = sec
            break

    if not matched_section:
        for sec in sections:
            if sec.startswith("### ") and ("完成通知" in sec or "发布" in sec or "升级" in sec):
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
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    strict = "--strict" in sys.argv or os.environ.get("STRICT_RELEASE_NOTES") == "1"

    tag_name = args[0] if len(args) > 0 else os.environ.get("GITHUB_REF_NAME", "v0.9.2")
    output_path = args[1] if len(args) > 1 else "release_notes.md"
    
    docs_dir = os.path.join(os.path.dirname(__file__), "..", "docs")
    release_notes_path = os.path.join(docs_dir, "RELEASE_NOTES.md")
    broadcasts_path = os.path.join(docs_dir, "broadcasts.md")

    # 优先级：docs/RELEASE_NOTES.md -> docs/broadcasts.md -> git log
    notes = extract_from_release_notes(tag_name, release_notes_path)
    source = "docs/RELEASE_NOTES.md"

    if not notes:
        if strict:
            print(f"::error::未在 {release_notes_path} 中找到 {tag_name} 的发布说明（strict 模式）", file=sys.stderr)
            sys.exit(1)
        print(f"::warning::未在 {release_notes_path} 中找到 {tag_name}，降级回退到 broadcasts.md", file=sys.stderr)
        notes = extract_from_broadcasts(tag_name, broadcasts_path)
        source = "docs/broadcasts.md (fallback)"

    if not notes:
        print(f"::warning::未在 broadcasts.md 中找到 {tag_name}，降级回退到 git log", file=sys.stderr)
        notes = extract_from_git()
        source = "git log (fallback)"

    if not notes:
        notes = f"DSH Dock {tag_name} 正式发布，包含性能优化、稳定性修复与功能升级。"
        source = "default string (fallback)"

    with open(output_path, "w", encoding="utf-8") as f:
        f.write(notes + "\n")

    print(f"✅ Release notes extracted from {source} ({len(notes)} chars) -> {output_path}")
    print("--- PREVIEW ---")
    print(notes[:500] + ("..." if len(notes) > 500 else ""))
    print("---------------")

if __name__ == "__main__":
    main()
