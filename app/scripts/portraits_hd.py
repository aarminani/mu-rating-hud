import os
import re
import subprocess
import sys
import unicodedata

import cv2
import numpy as np
from PIL import Image

APP = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TOOLKIT = os.environ.get("T8KIT_DIR", "")
ROSTER = os.path.join(APP, "crates", "core", "src", "roster.rs")
CACHE = os.path.join(APP, "scripts", ".portraits_src")
K = 3
TW, TH = 144, 176
MIN_SCORE = 0.85
PAD = 160


def char_key(name):
    folded = unicodedata.normalize("NFKC", name)
    return "".join(c for c in folded if c.isalnum()).casefold()


def ranked():
    src = open(ROSTER, encoding="utf-8").read()
    block = src[src.index("pub const RANKED"):]
    block = block[: block.index("];")]
    return re.findall(r'\("([a-z]{3})",\s*"([^"]+)",\s*"[^"]+"\)', block)


def decode(package, name):
    out = os.path.join(CACHE, name + ".png")
    if not os.path.exists(out):
        subprocess.run([sys.executable, "-m", "t8kit", "preview", package, "--out", out],
                       cwd=TOOLKIT, check=True, capture_output=True)
    return np.array(Image.open(out).convert("RGBA"))


def peak(res, loc):
    x, y = loc
    fx = fy = 0.0
    if 0 < x < res.shape[1] - 1:
        l, c, r = res[y, x - 1], res[y, x], res[y, x + 1]
        d = l - 2 * c + r
        fx = 0.5 * (l - r) / d if d != 0 else 0.0
    if 0 < y < res.shape[0] - 1:
        u, c, b = res[y - 1, x], res[y, x], res[y + 1, x]
        d = u - 2 * c + b
        fy = 0.5 * (u - b) / d if d != 0 else 0.0
    return x + float(np.clip(fx, -0.5, 0.5)), y + float(np.clip(fy, -0.5, 0.5))


def align(thumb, render):

    t = cv2.cvtColor(thumb[:, :, :3], cv2.COLOR_RGB2GRAY).astype(np.float32)
    mask = (thumb[:, :, 3] > 200).astype(np.float32)
    g = cv2.cvtColor(render[:, :, :3], cv2.COLOR_RGB2GRAY).astype(np.float32)
    g = cv2.copyMakeBorder(g, PAD, PAD, PAD, PAD, cv2.BORDER_REPLICATE)
    side = g.shape[1]

    def at(s):
        n = int(round(side / s))
        small = cv2.resize(g, (n, n), interpolation=cv2.INTER_AREA)
        s_true = side / n
        res = cv2.matchTemplate(small, t, cv2.TM_CCOEFF_NORMED, mask=mask)
        res = np.where(np.isfinite(res) & (np.abs(res) <= 1.0), res, -2).astype(np.float32)
        _, score, _, loc = cv2.minMaxLoc(res)
        return score, s_true, res, loc

    best = None
    for s in np.arange(2.2, 5.2, 0.02):
        score, s_true, res, loc = at(s)
        if best is None or score > best[0]:
            best = (score, s_true, res, loc)
    for s in np.arange(best[1] - 0.04, best[1] + 0.04, 0.004):
        score, s_true, res, loc = at(s)
        if score > best[0]:
            best = (score, s_true, res, loc)
    score, s, res, loc = best
    x, y = peak(res, loc)
    return score, s, x - PAD / s, y - PAD / s


def rebuild(thumb, render):
    score, s, x, y = align(thumb, render)
    W, H = TW * K, TH * K
    up_alpha = cv2.resize(thumb[:, :, 3].astype(np.float32), (W, H), interpolation=cv2.INTER_CUBIC)
    alpha = np.clip((up_alpha - 127.5) * 2.2 + 127.5, 0, 255).astype(np.uint8)
    if score < MIN_SCORE:
        plain = np.array(Image.fromarray(thumb).resize((W, H), Image.LANCZOS))
        plain[:, :, 3] = alpha
        return plain, score, None

    M = np.float32([[s / K, 0, (x + 0.5 / K) * s - 0.5], [0, s / K, (y + 0.5 / K) * s - 0.5]])
    crop = cv2.warpAffine(render, M, (W, H), flags=cv2.INTER_CUBIC | cv2.WARP_INVERSE_MAP,
                          borderMode=cv2.BORDER_REPLICATE)
    crop[:, :, 3] = cv2.warpAffine(render[:, :, 3], M, (W, H), flags=cv2.INTER_CUBIC | cv2.WARP_INVERSE_MAP,
                                   borderMode=cv2.BORDER_CONSTANT, borderValue=0)

    lab_t = cv2.cvtColor(thumb[:, :, :3], cv2.COLOR_RGB2LAB).astype(np.float32)
    lab = cv2.resize(lab_t, (W, H), interpolation=cv2.INTER_CUBIC)
    L = cv2.cvtColor(crop[:, :, :3], cv2.COLOR_RGB2LAB).astype(np.float32)[:, :, 0]
    L_small = cv2.resize(L, (TW, TH), interpolation=cv2.INTER_AREA)
    detail = L - cv2.resize(L_small, (W, H), interpolation=cv2.INTER_CUBIC)
    render_alpha = cv2.GaussianBlur(crop[:, :, 3].astype(np.float32) / 255, (0, 0), K)
    detail *= np.clip((render_alpha - 0.5) * 2, 0, 1)
    solid = thumb[:, :, 3] > 200
    gain = float(np.clip(np.polyfit(L_small[solid], lab_t[:, :, 0][solid], 1)[0], 0.8, 2.0))
    lab[:, :, 0] = np.clip(lab[:, :, 0] + gain * detail, 0, 255)
    rgb = cv2.cvtColor(np.round(lab).astype(np.uint8), cv2.COLOR_LAB2RGB)
    return np.dstack([rgb, alpha]), score, gain


def main(out_dir, sheet_path):
    if not os.path.isdir(TOOLKIT):
        print("Set T8KIT_DIR to the t8kit toolkit folder, which decodes the game's textures.")
        return 2
    os.makedirs(out_dir, exist_ok=True)
    os.makedirs(CACHE, exist_ok=True)
    rows = ranked()
    done, fallback, failed = [], [], []
    for code, short in rows:
        try:
            thumb = decode(f"/Game/UI/Rep_Texture/CS_Character_Thumb_Normal/T_UI_CS_Character_Thumb_Normal_{code}", f"thumb_{code}")
            render = decode(f"/Game/UI/Rep_Texture/CS_BG_Character_L/T_UI_CS_BG_Character_L_{code}", f"render_{code}")
            if thumb.shape[:2] != (TH, TW):
                raise ValueError(f"thumbnail is {thumb.shape[1]}x{thumb.shape[0]}, not {TW}x{TH}")
            img, score, gain = rebuild(thumb, render)
            key = char_key(short)
            Image.fromarray(img).save(os.path.join(out_dir, key + ".webp"), "WEBP", quality=90, alpha_quality=100, method=6)
            done.append((code, short, key, img, score))
            if gain is None:
                fallback.append((code, short, round(score, 3)))
            print(f"{code} {short:<12} score {score:.3f} {'gain %.2f' % gain if gain is not None else 'FALLBACK (upscaled thumbnail)'}")
        except (OSError, ValueError, subprocess.CalledProcessError) as e:
            failed.append((code, short, str(e)))

    cols, cw, ch = 7, TW * K // 2, TH * K // 2
    sheet = Image.new("RGBA", (cols * cw, ((len(done) + cols - 1) // cols) * ch), (27, 31, 39, 255))
    for i, (_, _, _, img, _) in enumerate(done):
        sheet.alpha_composite(Image.fromarray(img).resize((cw, ch), Image.LANCZOS), ((i % cols) * cw, (i // cols) * ch))
    sheet.convert("RGB").save(sheet_path)

    total = sum(os.path.getsize(os.path.join(out_dir, k + ".webp")) for _, _, k, _, _ in done)
    print(f"built {len(done)} of {len(rows)}; {total} B of webp; sheet {sheet_path}")
    for f in fallback:
        print("  fallback:", f)
    for f in failed:
        print("  failed:", f)
    return 0 if not failed else 1


if __name__ == "__main__":
    sys.exit(main(*sys.argv[1:3]))
