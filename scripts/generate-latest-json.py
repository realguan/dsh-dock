#!/usr/bin/env python3
"""从已上传的 Release 资产和 updater 签名生成 Tauri static update feed。"""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any
from urllib.parse import quote


# 当前 CI 发布的安装器集合。新增、删除或换架构时必须同步更新此契约与工作流矩阵。
REQUIRED_TARGETS = frozenset(
    {
        "darwin-aarch64",
        "linux-x86_64-appimage",
        "linux-x86_64-deb",
        "linux-x86_64-rpm",
        "windows-x86_64-msi",
        "windows-x86_64-nsis",
    }
)


def release_asset_name(name: str) -> str:
    """返回 GitHub Release 对含空格的上传文件采用的资产名。"""
    return name.replace(" ", ".")


def release_download_url(repository: str, tag: str, asset_name: str) -> str:
    """构造 Release 公开后稳定的资产下载 URL。"""
    if repository.count("/") != 1 or not tag:
        raise ValueError("repository 或 tag 无效")
    return (
        f"https://github.com/{repository}/releases/download/"
        f"{quote(tag, safe='')}/{quote(asset_name, safe='')}"
    )


def target_for_artifact(name: str) -> str | None:
    """把 Tauri updater 签名对象名映射为 static feed 的目标键。"""
    lower_name = name.lower()
    if lower_name.endswith(".app.tar.gz"):
        return "darwin-universal" if "universal" in lower_name else "darwin-aarch64"
    if lower_name.endswith(".appimage"):
        return "linux-x86_64-appimage"
    if lower_name.endswith(".deb"):
        return "linux-x86_64-deb"
    if lower_name.endswith(".rpm"):
        return "linux-x86_64-rpm"
    if lower_name.endswith(".msi"):
        return "windows-x86_64-msi"
    if lower_name.endswith(".setup.exe") or (
        "setup" in lower_name and lower_name.endswith(".exe")
    ):
        return "windows-x86_64-nsis"
    return None


def load_release_assets(path: Path) -> dict[str, str]:
    """读取 GitHub Release API 响应并返回资产名到下载 URL 的映射。"""
    data: Any = json.loads(path.read_text(encoding="utf-8"))
    entries = data.get("assets") if isinstance(data, dict) else data
    if not isinstance(entries, list):
        raise ValueError("Release API 响应缺少 assets 数组")

    assets: dict[str, str] = {}
    for entry in entries:
        if not isinstance(entry, dict):
            raise ValueError("Release API 响应包含无效资产")
        name = entry.get("name")
        url = entry.get("browser_download_url")
        if not isinstance(name, str) or not isinstance(url, str):
            raise ValueError("Release 资产缺少 name 或 browser_download_url")
        if name in assets:
            raise ValueError(f"Release 资产名重复：{name}")
        assets[name] = url
    return assets


def load_signatures(updater_root: Path) -> dict[str, str]:
    """读取 updater artifact 中的签名，键为被签名的原始文件名。"""
    if not updater_root.is_dir():
        raise ValueError(f"updater artifact 目录不存在：{updater_root}")

    signatures: dict[str, str] = {}
    for signature_path in sorted(updater_root.rglob("*.sig")):
        artifact_name = signature_path.name.removesuffix(".sig")
        signature = signature_path.read_text(encoding="utf-8").strip()
        if not signature:
            raise ValueError(f"签名文件为空：{signature_path}")
        existing = signatures.get(artifact_name)
        if existing is not None and existing != signature:
            raise ValueError(f"同名资产出现不一致签名：{artifact_name}")
        signatures[artifact_name] = signature
    return signatures


def build_platforms(
    assets: dict[str, str], signatures: dict[str, str], repository: str, tag: str
) -> dict[str, dict[str, str]]:
    """关联签名与已上传资产，并校验当前发行契约完整无缺。"""
    platforms: dict[str, dict[str, str]] = {}
    for artifact_name, signature in signatures.items():
        target = target_for_artifact(artifact_name)
        if target is None:
            continue

        asset_name = release_asset_name(artifact_name)
        url = assets.get(asset_name)
        if url is None:
            raise ValueError(f"签名对象未上传到 Release：{asset_name}")
        if target in platforms:
            raise ValueError(f"updater 目标重复：{target}")
        # Draft Release 的 browser_download_url 使用临时 untagged 路径；feed 在
        # 发布前生成，必须预先构造发布后稳定的 tag 下载 URL。
        platforms[target] = {
            "url": release_download_url(repository, tag, asset_name),
            "signature": signature,
        }

    missing = REQUIRED_TARGETS.difference(platforms)
    unexpected = set(platforms).difference(REQUIRED_TARGETS)
    if missing or unexpected:
        details: list[str] = []
        if missing:
            details.append(f"缺少目标：{', '.join(sorted(missing))}")
        if unexpected:
            details.append(f"未登记目标：{', '.join(sorted(unexpected))}")
        raise ValueError("updater 资产契约不完整（" + "；".join(details) + "）")
    return platforms


def generate_manifest(
    version: str,
    notes: str,
    assets: dict[str, str],
    signatures: dict[str, str],
    repository: str,
    tag: str,
) -> dict[str, Any]:
    """生成符合 Tauri static updater 格式的发布清单。"""
    return {
        "version": version,
        "notes": notes or f"DSH Dock {version}",
        "pub_date": datetime.now(timezone.utc)
        .isoformat(timespec="seconds")
        .replace("+00:00", "Z"),
        "platforms": build_platforms(assets, signatures, repository, tag),
    }


def parse_args() -> argparse.Namespace:
    """解析命令行参数。"""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", required=True)
    parser.add_argument("--repository", required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--assets-json", type=Path, required=True)
    parser.add_argument("--updater-root", type=Path, required=True)
    parser.add_argument("--notes", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args()


def main() -> None:
    """加载 Release 资产，生成并写入 latest.json。"""
    args = parse_args()
    assets = load_release_assets(args.assets_json)
    signatures = load_signatures(args.updater_root)
    notes = args.notes.read_text(encoding="utf-8").strip()
    manifest = generate_manifest(
        args.version, notes, assets, signatures, args.repository, args.tag
    )
    args.output.write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print(json.dumps(manifest, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
