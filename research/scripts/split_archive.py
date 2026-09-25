"""Split a large archive into <=90 MB parts for GitHub (100 MB file limit) and verify reassembly.
    python research/scripts/split_archive.py FILE      -> FILE.part00, FILE.part01, ..., FILE.sha256
Reassemble:  python research/scripts/split_archive.py --join FILE   (checks FILE.sha256)"""
import hashlib, sys
from pathlib import Path
PART = 90 * 1024 * 1024
if sys.argv[1] == "--join":
    f = Path(sys.argv[2]); parts = sorted(f.parent.glob(f.name + ".part*"))
    data = b"".join(p.read_bytes() for p in parts)
    want = (f.parent / (f.name + ".sha256")).read_text().split()[0]
    assert hashlib.sha256(data).hexdigest() == want, "sha256 mismatch"
    f.write_bytes(data); print("joined", f, len(parts), "parts; sha256 ok")
else:
    f = Path(sys.argv[1]); data = f.read_bytes(); h = hashlib.sha256(data).hexdigest()
    for i in range(0, len(data), PART):
        (f.parent / f"{f.name}.part{i // PART:02d}").write_bytes(data[i:i + PART])
    (f.parent / (f.name + ".sha256")).write_text(f"{h}  {f.name}\n")
    assert hashlib.sha256(b"".join(p.read_bytes() for p in sorted(f.parent.glob(f.name + ".part*")))).hexdigest() == h
    print(f, "->", len(range(0, len(data), PART)), "parts; sha256", h)
