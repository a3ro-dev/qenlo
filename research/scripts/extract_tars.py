"""Extract campaign tarballs into a scratch dir (never into raw data). usage: extract_tars.py DEST TAR..."""
import sys, tarfile
from pathlib import Path
dest = Path(sys.argv[1]); dest.mkdir(parents=True, exist_ok=True)
for t in sys.argv[2:]:
    with tarfile.open(t) as tf:
        tf.extractall(dest, filter="data")
        print(t, len(tf.getnames()))
