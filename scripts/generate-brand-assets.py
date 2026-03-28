#!/usr/bin/env python3
"""Generate deterministic raster previews (PPM) from a simple procedural logo — no extra deps."""

from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / "assets"
GEN = ASSETS / "generated"


def write_ppm(path: Path, size: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    header = f"P6\n{size} {size}\n255\n".encode("ascii")
    buf = bytearray(size * size * 3)
    for y in range(size):
        for x in range(size):
            # Gradient + soft "Z" cutout (original design, not copied from Windows assets).
            t = (x + y) / (2 * (size - 1 or 1))
            r = int(14 + t * 80)
            g = int(120 + t * 60)
            b = int(200 + t * 40)
            zx = x / size
            zy = y / size
            in_z = (zy > 0.22 and zy < 0.38 and zx > 0.18 and zx < 0.82) or (
                zy > 0.44 and zy < 0.56 and zx > 0.18 and zx < 0.82
            ) or (zy > 0.62 and zy < 0.78 and zx > 0.18 and zx < 0.82)
            if in_z:
                r, g, b = min(255, r + 40), min(255, g + 40), 255
            i = (y * size + x) * 3
            buf[i : i + 3] = bytes((r, g, b))
    path.write_bytes(header + bytes(buf))


def main() -> None:
    for n in (32, 64, 128):
        write_ppm(GEN / f"zirconos-mark-{n}.ppm", n)
    manifest = ASSETS / "manifest.json"
    data = json.loads(manifest.read_text(encoding="utf-8"))
    print("Updated PPM previews; manifest:", manifest)
    print("Raster entries:", json.dumps(data["assets"], indent=2))


if __name__ == "__main__":
    main()
