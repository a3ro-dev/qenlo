"""Per-run GPU-active (util>0) median SM and memory clock for locked experiments, plus lock.json lmc/lgc return codes.
Usage: python -I clock_checks.py RAW_DIR"""
import json, sys, glob, os, statistics as st, collections
root = sys.argv[1]
agg = collections.defaultdict(list)
for d in sorted(glob.glob(os.path.join(root, 'block-*', 'gpu-*'))):
    if not os.path.exists(os.path.join(d, 'lock.json')): continue
    sm, mem = [], []
    for line in open(os.path.join(d, 'clocks.csv')):
        f = [x.strip() for x in line.split(',')]
        if len(f) > 4 and f[1].isdigit() and f[2].isdigit() and f[4].isdigit() and int(f[4]) > 0:
            sm.append(int(f[1])); mem.append(int(f[2]))
    j = json.load(open(os.path.join(d, 'lock.json')))
    name = os.path.basename(d); blk = os.path.basename(os.path.dirname(d))
    cond = name.split('-', 3)[-1] if name.startswith('gpu-rows') else name.split('-', 3)[-1]
    key = name.split('-e')[0] + '|' + '-'.join(name.split('-')[2:])
    rc = {k: (v[0] if isinstance(v, list) else v) for k, v in (j.get('lock') or j.get('applied') or {}).items()} if isinstance(j.get('lock') or j.get('applied'), dict) else None
    agg[key].append((blk, st.median(sm) if sm else None, st.median(mem) if mem else None, sorted(set(mem)), j.get('requested'), j.get('temperature_max_c')))
for k, v in sorted(agg.items()):
    sms = sorted({x[1] for x in v}); mems = sorted({x[2] for x in v})
    print(f'{k:32s} n={len(v)} activeSMmed={sms} activeMEMmed={mems} requested={v[0][4]}')
