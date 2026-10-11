"""The IP layer: the secret key, the invisible watermark, stamped writes,
and verification. See test_watermark.py for the audit regression suite."""
import hashlib
import hmac
import sys

import cv2
import numpy as np

from config import STAMP, WORK

def wm_key(create=False):
    """Secret watermark key — lives in gitignored atelier-work, never in git.
    Losing it means old marks can't be verified: back it up with the photos.
    NOTHING mints a key as a side effect (audit: genpaper on a fresh clone
    used to silently mark public assets under a junk key nobody holds) —
    creation happens only through `pipeline.py genkey`."""
    kp = WORK / "wm.key"
    if not kp.exists():
        if not create:
            sys.exit(
                f"no watermark key at {kp} — run 'pipeline.py genkey' (key holder's machine) "
                "or restore the backup before marking/verifying"
            )
        import secrets

        kp.parent.mkdir(parents=True, exist_ok=True)
        kp.write_bytes(secrets.token_bytes(32))
        kp.chmod(0o600)
    key = kp.read_bytes()
    if len(key) != 32:
        sys.exit(f"corrupt watermark key at {kp}: {len(key)} bytes, want 32 — restore the backup")
    if kp.stat().st_mode & 0o077:
        kp.chmod(0o600)
    return key


def stage_genkey():
    kp = WORK / "wm.key"
    if kp.exists():
        sys.exit(f"refusing to overwrite the existing key at {kp}")
    import hashlib

    key = wm_key(create=True)
    print(f"new key at {kp} (id {hashlib.sha256(key).hexdigest()[:8]}) — BACK IT UP with the photos")


WM_MAGIC = b"ASWM2"


def watermark(img):
    """Invisible provenance mark (v2, post-audit): every pixel LSB is zeroed,
    then the head LSBs carry [2B len | ASWM2 | 4B key-id | STAMP | 16B HMAC].
    The HMAC covers the zeroed pixels AND the length AND the payload — the
    v1 mark authenticated only the pixels, so anyone could rewrite the stamp
    text and keep the tag (audit finding #1). Beyond the mark the LSB plane
    is all zeros by construction, and verify enforces that, so stray hidden
    data also fails (#2).
    Honest ceiling: HMAC is symmetric — demonstrating the mark to a third
    party means disclosing the key, which then enables forgery; and the mark
    proves linkage to the key, not precedence. If public verification ever
    matters (stores, disputes), switch to Ed25519 signatures. Survives file
    copies, not re-encode/resize."""
    import hashlib
    import hmac

    key = wm_key()
    flat = img.reshape(-1).copy()
    flat &= 0xFE
    payload = WM_MAGIC + hashlib.sha256(key).digest()[:4] + STAMP.encode()
    len_bytes = (len(payload) + 16).to_bytes(2, "big")
    mac = hmac.new(key, flat.tobytes() + len_bytes + payload, hashlib.sha256).digest()[:16]
    bits = np.unpackbits(np.frombuffer(len_bytes + payload + mac, np.uint8))
    if bits.size > flat.size:
        sys.exit("image too small to watermark")
    flat[: bits.size] |= bits
    return flat.reshape(img.shape)


# per-digit blend tuning for rows that need tighter, calmer blends than the
# global recipe (higher alpha = more even mixing, lower amp = less warp)
def stage_verify(path):
    import hashlib
    import hmac

    from PIL import Image

    try:
        img = Image.open(path)
        img.load()
    except Exception as e:
        sys.exit(f"cannot read image '{path}': {e}")
    arr = np.array(img)
    if arr.dtype != np.uint8:
        sys.exit(f"unsupported pixel format ({arr.dtype}) — the mark lives in 8-bit PNGs")
    key = wm_key()
    my_id = hashlib.sha256(key).digest()[:4]
    flat = arr.reshape(-1)
    n = int.from_bytes(np.packbits(flat[:16] & 1).tobytes(), "big")
    end = 16 + n * 8
    if n < len(WM_MAGIC) + 4 + 16 + 1 or end > flat.size:
        sys.exit(f"NOT VERIFIED — no plausible mark in '{path}' (payload length {n} out of range)")
    msg = np.packbits(flat[16:end] & 1).tobytes()
    payload, mac = msg[:-16], msg[-16:]
    if not payload.startswith(WM_MAGIC):
        sys.exit(f"NOT VERIFIED — no {WM_MAGIC.decode()} mark in '{path}' (unmarked, or a pre-v2 mark)")
    # the MAC binds pixels, length AND payload — a rewritten stamp can no
    # longer ride on a genuine tag (audit finding #1)
    want = hmac.new(key, (flat & 0xFE).tobytes() + n.to_bytes(2, "big") + payload, hashlib.sha256).digest()[:16]
    key_id, stamp = payload[len(WM_MAGIC) : len(WM_MAGIC) + 4], payload[len(WM_MAGIC) + 4 :]
    if not hmac.compare_digest(want, mac):
        if key_id != my_id:
            sys.exit(
                f"NOT VERIFIED — mark claims key id {key_id.hex()}, your key is {my_id.hex()}: "
                "not minted with this key (or payload forged)"
            )
        sys.exit("NOT VERIFIED — key matches, but pixels or payload were altered (tampered)")
    # the whole LSB plane beyond the mark must be zero: hidden extra data
    # would otherwise verify as 'untampered' (audit finding #2)
    if (flat[end:] & 1).any():
        sys.exit("NOT VERIFIED — valid mark, but extra data is hidden in the LSB plane (altered after marking)")
    # verdict first, on its own line; attacker-influenced bytes only after,
    # escaped — a crafted stamp once ANSI-overwrote the verdict (finding #3)
    print(f"VERIFIED — authentic mark of key {my_id.hex()}; pixels, stamp and LSB plane all authenticated")
    # control chars escaped explicitly: ESC is ASCII, so backslashreplace
    # alone would pass ANSI sequences straight to the terminal
    decoded = stamp.decode("utf-8", "replace")
    safe = "".join(ch if 32 <= ord(ch) < 127 else f"\\x{ord(ch):02x}" for ch in decoded)
    print("stamp: " + safe)
    if stamp != STAMP.encode():
        print("note: stamp text differs from the current STAMP constant (historical mark?)")


def save_png(path, img):
    """Write an asset: invisible watermark in the pixels + IP stamp in tEXt."""
    from PIL import Image, PngImagePlugin

    rgb = cv2.cvtColor(img, cv2.COLOR_BGRA2RGBA if img.shape[2] == 4 else cv2.COLOR_BGR2RGB)
    info = PngImagePlugin.PngInfo()
    info.add_text("Copyright", STAMP)
    info.add_text("Author", "Bernardo Cohen")
    Image.fromarray(watermark(rgb)).save(str(path), pnginfo=info)


