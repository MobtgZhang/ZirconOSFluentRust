#!/usr/bin/env python3
"""Resize icon masters in resources/icons/_sources/*.png to standard desktop sizes.

Masters should be square PNGs; the script trims opaque bounds, recenters on a transparent
square with even padding (avoids downscaled icons looking “squashed”), removes uniform
light corners when all four match (AI often outputs white matting), then LANCZOS resizes.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

try:
    from PIL import Image
except ImportError:
    print("Install Pillow: pip install pillow", file=sys.stderr)
    raise SystemExit(1)

ROOT = Path(__file__).resolve().parents[1]
SOURCES = ROOT / "resources" / "icons" / "_sources"
OUT_ROOT = ROOT / "resources" / "icons"
SIZES = (16, 24, 32, 48, 256)
ICON_NAMES = ("computer", "folder", "settings", "terminal", "trash")

# Extra transparent margin around trimmed content (fraction of max side).
PAD_RATIO = 0.14


def lift_uniform_light_background(im: Image.Image, rgb_tol: int = 40, luma_min: int = 205) -> Image.Image:
    """If all four corners share a similar light color, treat it as matte and clear alpha."""
    im = im.convert("RGBA")
    px = im.load()
    w, h = im.size
    if w < 2 or h < 2:
        return im
    corners_rgb = [
        px[0, 0][:3],
        px[w - 1, 0][:3],
        px[0, h - 1][:3],
        px[w - 1, h - 1][:3],
    ]

    def dist2(a: tuple[int, int, int], b: tuple[int, int, int]) -> int:
        return (a[0] - b[0]) ** 2 + (a[1] - b[1]) ** 2 + (a[2] - b[2]) ** 2

    base = corners_rgb[0]
    if any(dist2(base, c) > 35 * 35 for c in corners_rgb[1:]):
        return im

    ar = sum(c[0] for c in corners_rgb) // 4
    ag = sum(c[1] for c in corners_rgb) // 4
    ab = sum(c[2] for c in corners_rgb) // 4
    if (ar + ag + ab) // 3 < luma_min:
        return im

    for y in range(h):
        for x in range(w):
            r, g, b, a = px[x, y]
            if (
                abs(r - ar) <= rgb_tol
                and abs(g - ag) <= rgb_tol
                and abs(b - ab) <= rgb_tol
            ):
                px[x, y] = (r, g, b, 0)
    return im


def normalize_square_canvas(im: Image.Image) -> Image.Image:
    """Crop to alpha bbox, paste centered on transparent square with symmetric padding."""
    im = im.convert("RGBA")
    alpha = im.split()[3]
    bbox = alpha.getbbox()
    if bbox is None:
        return im
    cropped = im.crop(bbox)
    w, h = cropped.size
    side = int(max(w, h) * (1.0 + 2.0 * PAD_RATIO))
    side = max(side, 1)
    out = Image.new("RGBA", (side, side), (0, 0, 0, 0))
    ox = (side - w) // 2
    oy = (side - h) // 2
    out.paste(cropped, (ox, oy), cropped)
    return out


def downscale_sharp(im: Image.Image, sz: int) -> Image.Image:
    """Two-step downscale for small sizes reads cleaner than one jump from huge masters."""
    if im.size[0] <= sz:
        return im.resize((sz, sz), Image.Resampling.LANCZOS)
    mid = max(sz * 4, 256)
    mid = min(mid, im.size[0])
    if im.size[0] > mid:
        im = im.resize((mid, mid), Image.Resampling.LANCZOS)
    return im.resize((sz, sz), Image.Resampling.LANCZOS)


def main() -> None:
    manifest_path = ROOT / "resources" / "manifest.json"
    data = json.loads(manifest_path.read_text(encoding="utf-8"))
    assets: list[dict] = [a for a in data.get("assets", []) if not a.get("id", "").startswith("icon.")]

    for name in ICON_NAMES:
        src = SOURCES / f"{name}.png"
        if not src.is_file():
            print("skip (missing source):", src, file=sys.stderr)
            continue
        im = Image.open(src)
        im = lift_uniform_light_background(im)
        im = normalize_square_canvas(im)
        out_dir = OUT_ROOT / name
        out_dir.mkdir(parents=True, exist_ok=True)
        for sz in SIZES:
            out = out_dir / f"icon-{sz}.png"
            thumb = downscale_sharp(im, sz)
            thumb.save(out, format="PNG", optimize=True)
            assets.append(
                {
                    "id": f"icon.{name}.{sz}",
                    "path": f"icons/{name}/icon-{sz}.png",
                    "role": "icon",
                    "note": f"{name} app icon, {sz}px, transparent PNG",
                }
            )
        print("generated", name, "->", out_dir)

    data["assets"] = assets
    manifest_path.write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
    print("updated", manifest_path)


if __name__ == "__main__":
    main()
