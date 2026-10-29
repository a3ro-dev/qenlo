"""Inventory every run directory: timestamps, rc, git_revision/dirty, adapter, recall, filter violations,
row counts of samples.csv. Usage: python -I inventory_runs.py LABEL=DIR [LABEL=DIR ...]"""
import csv, json, sys, glob, os, collections, datetime as dt
UTC = dt.timezone.utc
def conf(p):
    d = {}
    try:
        for l in open(p, encoding='utf-8', errors='replace'):
            if '=' in l:
                k, v = l.rstrip('\n').split('=', 1); d[k] = v
    except FileNotFoundError:
        pass
    return d
for arg in sys.argv[1:]:
    label, root = arg.split('=', 1)
    runs = sorted(glob.glob(os.path.join(root, 'block-*', '*', 'run.json')))
    rev = collections.Counter(); adapters = collections.Counter(); rec = collections.Counter(); fv = collections.Counter()
    nsamp = collections.Counter(); rcs = collections.Counter(); starts = []; status = collections.Counter(); blocks = set()
    min_recall_rows = []
    for rj in runs:
        d = os.path.dirname(rj); j = json.load(open(rj)); rcs[j.get('returncode')] += 1
        blocks.add(os.path.basename(os.path.dirname(d)))
        starts.append(j['started_unix'])
        c = conf(os.path.join(d, 'bench', 'configuration.txt'))
        rev[(c.get('git_revision', '?')[:7], c.get('git_worktree_dirty', '?'))] += 1
        adapters[c.get('gpu_adapter', c.get('adapter', '-'))] += 1
        s = conf(os.path.join(d, 'bench', 'summary.txt'))
        status[s.get('status', 'missing')] += 1
        rec[(s.get('evaluation_recall_at_10', '?'), s.get('tuning_recall_at_10', '?'))] += 1
        fv[s.get('filter_violations', '?')] += 1
        sp = os.path.join(d, 'bench', 'samples.csv')
        if os.path.exists(sp):
            n = 0; rmin = 1.0
            with open(sp) as f:
                for r in csv.DictReader(f):
                    n += 1
                    try: rmin = min(rmin, float(r['recall_at_10']))
                    except: pass
            nsamp[n] += 1
            if rmin < 1.0: min_recall_rows.append((os.path.relpath(d, root), rmin))
        else:
            nsamp['no samples.csv'] += 1
    f = lambda t: dt.datetime.fromtimestamp(t, UTC).strftime('%Y-%m-%dT%H:%M:%SZ')
    print(f'=== {label}: {len(runs)} runs, blocks={sorted(blocks)}')
    if starts: print('  first start', f(min(starts)), ' last start', f(max(starts)))
    print('  returncodes', dict(rcs)); print('  summary status', dict(status))
    print('  git_revision/dirty', dict(rev)); print('  adapters', dict(adapters))
    print('  (eval_recall@10, tuning_recall@10)', dict(rec)); print('  filter_violations', dict(fv))
    print('  samples.csv rows', dict(nsamp))
    print('  runs with any per-call recall_at_10 < 1:', min_recall_rows[:20], 'count', len(min_recall_rows))
