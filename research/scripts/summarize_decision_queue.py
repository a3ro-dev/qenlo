"""Reduce completed blocks without pooling queries or hiding incomplete queues."""
import argparse, csv, json, math, random, statistics
from collections import defaultdict
from pathlib import Path

def properties(p):
    return dict(line.split('=',1) for line in p.read_text().splitlines() if '=' in line)

def main():
    p=argparse.ArgumentParser(description=__doc__); p.add_argument('directory',type=Path); args=p.parse_args()
    root=args.directory.resolve(); queue=json.loads((root/'queue.json').read_text())
    groups=defaultdict(dict)
    for i,(kind,destination,cmd) in enumerate(queue['jobs']):
        if not (root/f'checkpoint-{i:04d}.json').exists(): raise SystemExit('Incomplete queue; no confirmatory reduction.')
        if kind!='measurement': continue
        dest=Path(destination); summary=properties(dest/'summary.txt')
        assert summary['status']=='completed' and summary['recall_target_passed']=='true'
        assert int(summary['filter_violations'])==0
        rows=list(csv.DictReader((dest/'samples.csv').open()))
        actual={r['actual_backend'] for r in rows}
        assert all(r['fallback']=='false' for r in rows), 'Unexpected fallback: retain failure and stop'
        parent=dest.parent.parent.name; seed,cell=parent.split('-',1)
        groups[(cell,seed,dest.parent.name)][dest.name]={'p95':int(summary['median_run_p95_batch_ns']),
            'backend':next(iter(actual)) if len(actual)==1 else 'mixed'}
    ratios=defaultdict(lambda:defaultdict(list))
    for (cell,seed,block),arms in groups.items():
        if queue['experiment']=='E1':
            pairs=[('cpu-b','cpu-a'),('gpu-b','gpu-a')]
            pairs += [('auto','cpu-a' if arms['auto']['backend']=='Cpu' else 'gpu-a'),('gpu-a','cpu-a')]
        elif queue['experiment']=='E2': pairs=[('one-pass','legacy-two-pass'),('cached','one-pass'),('one-pass','cpu')]
        else: pairs=[('gpu','cpu')]
        for a,b in pairs: ratios[(cell,a+'/'+b)][seed].append(arms[a]['p95']/arms[b]['p95'])
    rng=random.Random(20260923); out=[]
    for (cell,comparison),seeds in sorted(ratios.items()):
        # Stratify by dataset seed, resample complete paired blocks within each seed.
        # This interval is conditional on these two datasets, not unseen corpora.
        values=[v for vs in seeds.values() for v in vs]
        observed=math.exp(statistics.mean(map(math.log,values)))
        boot=[]
        for _ in range(20000):
            picked=[rng.choice(vs) for vs in seeds.values() for _ in vs]
            boot.append(math.exp(statistics.mean(map(math.log,picked))))
        boot.sort()
        out.append({'cell':cell,'comparison':comparison,'paired_blocks':len(values),'dataset_seeds':len(seeds),
                    'geomean_p95_ratio':observed,'ci95_lo':boot[499],'ci95_hi':boot[19499],
                    'ci98_75_lo':boot[124],'ci98_75_hi':boot[19874],
                    'worst_block_ratio':max(values),'best_block_ratio':min(values)})
    with (root/'paired-results.csv').open('w',newline='') as f:
        w=csv.DictWriter(f,fieldnames=list(out[0]));w.writeheader();w.writerows(out)
    print(json.dumps({'contrasts':out,'interpretation':'Ratios below one favor numerator. Conditional paired-block intervals; only two dataset seeds. Adjust for the preregistered comparison family; no optional stopping or population claim.'},indent=2))

if __name__=='__main__':main()
