"""Collect started_unix/ended_unix/returncode from every run.json under a raw-data dir.
Usage: python -I run_timestamps.py <dir> [<dir>...]"""
import json, sys, os, glob, datetime as dt
IST = dt.timezone(dt.timedelta(hours=5, minutes=30))
for root in sys.argv[1:]:
    rows = []
    for p in glob.glob(os.path.join(root, '**', 'run.json'), recursive=True):
        try:
            j = json.load(open(p))
        except Exception as e:
            print('BAD', p, e); continue
        rows.append((j.get('started_unix'), j.get('ended_unix'), j.get('returncode'), os.path.relpath(os.path.dirname(p), root)))
    rows = [r for r in rows if r[0] is not None]
    rows.sort()
    rc = {}
    for r in rows: rc[r[2]] = rc.get(r[2], 0) + 1
    f = lambda t: dt.datetime.fromtimestamp(t, IST).strftime('%Y-%m-%d %H:%M:%S IST') + ' / ' + dt.datetime.fromtimestamp(t, dt.timezone.utc).strftime('%H:%M:%SZ')
    print(f'== {root}: n={len(rows)} returncodes={rc}')
    if rows:
        print('  first start:', f(rows[0][0]), rows[0][3])
        print('  last start :', f(rows[-1][0]), rows[-1][3])
        print('  last end   :', f(max(r[1] for r in rows if r[1])))
