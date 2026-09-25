"""Pack a raw-data directory into DIR.tar.gz next to it and print its sha256 (source dir is left untouched).
    python research/scripts/pack_raw.py DIR [DIR...]"""
import hashlib, sys, tarfile
from pathlib import Path
for d in map(Path, sys.argv[1:]):
    out = d.with_name(d.name + ".tar.gz")
    if out.exists():
        raise SystemExit(f"refusing to overwrite {out}")
    with tarfile.open(out, "w:gz", compresslevel=9) as t:
        t.add(d, arcname=d.name)
    with tarfile.open(out) as t:
        n = sum(1 for m in t.getmembers() if m.isfile())
    src = sum(1 for p in d.rglob("*") if p.is_file())
    assert n == src, (n, src)
    h = hashlib.sha256(out.read_bytes()).hexdigest()
    print(f"{out} files={n} bytes={out.stat().st_size} sha256={h}")
