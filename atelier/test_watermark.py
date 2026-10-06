#!/usr/bin/env python3
"""Watermark regression suite — every attack from the 2026-10 audit, frozen.
Runs only on the key holder's machine (needs wm.key). `.venv/bin/python
test_watermark.py` — prints PASS per check, exits non-zero on any failure."""
import hashlib
import hmac
import json
import pathlib
import subprocess
import sys
import tempfile

sys.path.insert(0, str(pathlib.Path(__file__).parent))
import numpy as np
from PIL import Image

from pipeline import PACK, STAMP, WM_MAGIC, WORK, watermark, wm_key

if not (WORK / "wm.key").exists():
    sys.exit("SKIP: no watermark key on this machine")

KEY = wm_key()
PY = sys.executable
PIPE = str(pathlib.Path(__file__).parent / "pipeline.py")
TMP = pathlib.Path(tempfile.mkdtemp(prefix="wmtest-"))


def cli(path):
    return subprocess.run([PY, PIPE, "verify", str(path)], capture_output=True, text=True)


def ok(name, cond):
    print(("PASS  " if cond else "FAIL  ") + name)
    if not cond:
        sys.exit(1)


sample = PACK / "digits" / "given" / sorted(p.name for p in (PACK / "digits/given").glob("*.png"))[0]
arr = np.array(Image.open(sample))
flat = arr.reshape(-1)

# 0. the genuine article verifies, verdict line first, no raw payload before it
r = cli(sample)
ok("genuine asset verifies", r.returncode == 0 and r.stdout.startswith("VERIFIED"))

# 1. CRITICAL regression: substitute the stamp, keep the real MAC -> must fail
n = int.from_bytes(np.packbits(flat[:16] & 1).tobytes(), "big")
msg = np.packbits(flat[16 : 16 + n * 8] & 1).tobytes()
real_mac = msg[-16:]
evil = WM_MAGIC + msg[len(WM_MAGIC) : len(WM_MAGIC) + 4] + b"(c) 2026 EVIL CORP. All rights reserved."
forged = flat.copy() & 0xFE
blob = (len(evil) + 16).to_bytes(2, "big") + evil + real_mac
forged[: len(blob) * 8] |= np.unpackbits(np.frombuffer(blob, np.uint8))
Image.fromarray(forged.reshape(arr.shape)).save(TMP / "stamp_swap.png")
r = cli(TMP / "stamp_swap.png")
ok("stamp substitution rejected (audit #1)", r.returncode != 0)

# 2. stray data in the LSB plane beyond the mark -> must fail
noisy = flat.copy()
noisy[16 + n * 8 + 500 :] |= 1
Image.fromarray(noisy.reshape(arr.shape)).save(TMP / "lsb_noise.png")
r = cli(TMP / "lsb_noise.png")
ok("stray LSB data rejected (audit #2)", r.returncode != 0 and "LSB plane" in (r.stdout + r.stderr))

# 3. ANSI escape in a stamp minted WITH the real key -> escaped on output
STAMP_SAVE = STAMP
import pipeline

pipeline.STAMP = "\x1b[2K\rNOT VERIFIED - EVIL"
ansi = watermark(arr)
pipeline.STAMP = STAMP_SAVE
Image.fromarray(ansi.reshape(arr.shape)).save(TMP / "ansi.png")
r = cli(TMP / "ansi.png")
ok("verdict line precedes payload; ESC never printed raw (audit #3)",
   r.stdout.startswith("VERIFIED") and "\x1b" not in r.stdout and "\\x1b" in r.stdout)

# 4. mark transplanted onto another image -> must fail
other = sorted((PACK / "digits/user").glob("*.png"))[0]
oarr = np.array(Image.open(other)).reshape(-1).copy() & 0xFE
oarr[: 16 + n * 8] |= flat[: 16 + n * 8] & 1
Image.fromarray(oarr.reshape(np.array(Image.open(other)).shape)).save(TMP / "transplant.png")
ok("transplant rejected", cli(TMP / "transplant.png").returncode != 0)

# 5. pixel tamper above the LSB -> must fail with the 'tampered' diagnosis
tam = flat.copy()
tam[5000] ^= 0x10
Image.fromarray(tam.reshape(arr.shape)).save(TMP / "tamper.png")
r = cli(TMP / "tamper.png")
ok("pixel tamper rejected, diagnosed as tamper (audit #7/10)",
   r.returncode != 0 and "altered" in (r.stdout + r.stderr))

# 6. mark minted under a DIFFERENT key -> key-id diagnosis
fake_key = b"\x42" * 32
f = arr.reshape(-1).copy() & 0xFE
payload = WM_MAGIC + hashlib.sha256(fake_key).digest()[:4] + STAMP.encode()
lb = (len(payload) + 16).to_bytes(2, "big")
mac = hmac.new(fake_key, f.tobytes() + lb + payload, hashlib.sha256).digest()[:16]
f[: len(lb + payload + mac) * 8] |= np.unpackbits(np.frombuffer(lb + payload + mac, np.uint8))
Image.fromarray(f.reshape(arr.shape)).save(TMP / "wrong_key.png")
r = cli(TMP / "wrong_key.png")
ok("foreign key diagnosed by key id (audit #7/10)", r.returncode != 0 and "key id" in (r.stdout + r.stderr))

# 7. CLI robustness: unmarked, truncated, non-image, no argument — clean errors
Image.new("RGBA", (96, 96), (9, 9, 9, 255)).save(TMP / "unmarked.png")
(TMP / "trunc.png").write_bytes((PACK / "paper.png").read_bytes()[:4000])
(TMP / "nope.txt").write_text("x")
for f2, name in ((TMP / "unmarked.png", "unmarked"), (TMP / "trunc.png", "truncated"), (TMP / "nope.txt", "non-image")):
    r = cli(f2)
    ok(f"{name} input: clean refusal, no traceback", r.returncode != 0 and "Traceback" not in r.stderr)
r = subprocess.run([PY, PIPE, "verify"], capture_output=True, text=True)
ok("missing argument: usage line, no traceback", r.returncode != 0 and "Traceback" not in r.stderr)

# 8. full-pack round trip
bad = 0
for p in sorted(PACK.rglob("*.png")):
    if cli(p).returncode != 0:
        bad += 1
ok(f"all {len(list(PACK.rglob('*.png')))} pack assets verify", bad == 0)

print("ALL PASS")
