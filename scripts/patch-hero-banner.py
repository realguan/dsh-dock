#!/usr/bin/env python3
"""README 首图（assets/hero-banner.png）文案修正工具（2026-09-10）。

背景：hero-banner.png 没有矢量/HTML 源文件，是位图产物；文案一旦与产品实际能力
不符（例如把路线图里的「时间线回放」当成已上线能力、平台列表漏掉 Linux），
只能原位修字。本脚本记录该位图的全部可测几何量，供后续同类修正复用。

位图几何（1200x500，1x）：
    角色立绘   x < 440（1024x1024 透明立绘 assets/whale-chan-cutout.png 裁切）
    文本面板   x 440..1151，y 45..456，圆角 ~14px，边框 ~1px rgba(255,255,255,.07)
    胶囊       y 96..127   x 484..731   白字 11px letter-spacing ≈ .14em
    标题       y 126..163  x 481..879   "DSH Dock"  ~#E2E8F0
    副标题     y 191..212  x 481..879   17px        #A0AFC8
    四条要点   y 236 / 266 / 296 / 326（行距 30px）
               前缀「·」x 486..488，正文起始 x 500，17px #A0AFC8
    chip 行    y 385..417（高 33，圆角 7，边框 #334155，间隙 10，内边距 12）
               chip 文字 14px #93C5FD

字体：与前端 index.css 同栈 —— 拉丁 -apple-system（SF Pro，/System/Library/Fonts/SFNS.ttf），
      中日韩 PingFang SC（macOS 位于 AssetsV2 资产目录，见下方 PINGFANG 常量）。

用法：只能对**未修正前的原始位图**运行（擦除依赖上下相邻的干净背景行）。
      原始位图备份若已丢失，请从 git 历史取回后运行。
"""
import shutil
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

BANNER = Path(__file__).resolve().parent.parent / "assets" / "hero-banner.png"
SF = "/System/Library/Fonts/SFNS.ttf"
PINGFANG = (
    "/System/Library/AssetsV2/com_apple_MobileAsset_Font8/"
    "86ba2c91f017a3749571a82f2c6d890ac7ffb2fb.asset/AssetData/PingFang.ttc"
)
FONT_INDEX_SC = 3  # PingFang SC Regular

BODY = (160, 175, 200)  # #A0AFC8 正文/副标题
CHIP_TEXT = (147, 197, 253)  # #93C5FD
CHIP_BORDER = (51, 65, 85)  # #334155

# 需要修正的要点行：y 区间 → 新文案（行距 30px，正文起始 x=500）
LINES = {
    (296, 310): "会话维护与智能自愈：坏档检测、世代分叉修复、全自动无损自愈",
}
# 行尾替换：只换词尾，保持整行其余部分不动
TAIL_SWAP = {"WSL2": "Linux"}
CHIPS = [
    "Tauri v2",
    "React 19",
    "Tailwind v4",
    "Rust Core",
    "macOS / Win / Linux",
    "MIT License",
]


def erase(px, x0, x1, y0, y1, ref_top, ref_bottom):
    """用上下两条干净背景行做垂直插值，抹掉 [x0..x1, y0..y1] 的文字。"""
    for y in range(y0, y1 + 1):
        t = (y - ref_top) / (ref_bottom - ref_top)
        for x in range(x0, x1 + 1):
            a, b = px[x, ref_top], px[x, ref_bottom]
            px[x, y] = tuple(round(a[i] + (b[i] - a[i]) * t) for i in range(3))


def paste_text(im, text, font, color, ink_left, ink_top):
    layer = Image.new("L", (900, 70), 0)
    ImageDraw.Draw(layer).text((5, 5), text, font=font, fill=255)
    box = layer.getbbox()
    im.paste(Image.new("RGB", layer.size, color),
             (ink_left - box[0], ink_top - box[1]), layer)
    return box[2] - box[0], box[3] - box[1]


def main():
    if not Path(PINGFANG).exists():
        sys.exit(f"PingFang 字体缺失：{PINGFANG}")
    backup = BANNER.with_suffix(".png.orig")
    if not backup.exists():
        shutil.copy2(BANNER, backup)
        print(f"已备份原始位图 → {backup.name}")

    im = Image.open(backup).convert("RGB")
    px = im.load()
    body_font = ImageFont.truetype(PINGFANG, 17, index=FONT_INDEX_SC)
    latin_font = ImageFont.truetype(SF, 17)

    for (y0, y1), text in LINES.items():
        erase(px, 497, 1020, y0 - 5, y1 + 6, y0 - 7, y1 + 8)
        w, h = paste_text(im, text, body_font, BODY, 500, y0)
        print(f"要点行 y{y0}：重绘 {w}x{h}px")

    # 行尾词替换（拉丁部分用 SF Pro）
    for old, new in TAIL_SWAP.items():
        band = [y for y in range(320, 350)
                if any(sum(px[x, y]) / 3 > 70 for x in range(950, 1000))]
        if not band:
            continue
        erase(px, 948, 1002, band[0] - 7, band[-1] + 8, band[0] - 9, band[-1] + 10)
        w, h = paste_text(im, new, latin_font, BODY, 953, band[0])
        print(f"行尾 {old} → {new}：重绘 {w}x{h}px")

    # chip 行整体重绘（自左向右排布，内边距 12 / 间隙 10）
    erase(px, 470, 1140, 380, 422, 378, 424)
    draw = ImageDraw.Draw(im)
    chip_font = ImageFont.truetype(SF, 14)
    x = 480
    for label in CHIPS:
        layer = Image.new("L", (300, 40), 0)
        ImageDraw.Draw(layer).text((5, 5), label, font=chip_font, fill=255)
        box = layer.getbbox()
        width = box[2] - box[0]
        draw.rounded_rectangle((x, 385, x + width + 23, 417),
                               radius=7, outline=CHIP_BORDER, width=1)
        im.paste(Image.new("RGB", layer.size, CHIP_TEXT),
                 (x + 12 - box[0], 395 - box[1]), layer)
        x += width + 34
    print(f"chip 行重绘 {len(CHIPS)} 枚，右端 x={x - 10}")

    im.save(BANNER)
    print(f"已写入 {BANNER}")


if __name__ == "__main__":
    main()
