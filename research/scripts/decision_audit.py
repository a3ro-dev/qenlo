"""Read-only evidence audit; write new decision artifacts, never raw inputs."""
from __future__ import annotations
import csv, hashlib, importlib.util, io, json, math, statistics, struct, subprocess, sys, tarfile, zlib
from collections import Counter, defaultdict
from pathlib import Path
import numpy as np

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'research/data/processed/decision-audit-2026-09-23'
OUT.mkdir(parents=True, exist_ok=True)

def save(name, value):
    (OUT / name).write_text(json.dumps(value, indent=2, default=lambda x: x.item()) + '\n', encoding='utf-8')

def table(name, rows):
    with (OUT / name).open('w', newline='', encoding='utf-8') as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0])); w.writeheader(); w.writerows(rows)

def digest(p):
    with p.open('rb') as f: return hashlib.file_digest(f, 'sha256').hexdigest()

def props(s): return dict(line.split('=', 1) for line in s.splitlines() if '=' in line)
def pct(x, q): return sorted(x)[math.ceil(q * len(x))-1]

def corpus_audit():
    p = ROOT / 'data/ag-news/ag-news-100k-384.qnb'
    with p.open('rb') as f:
        header = f.read(56)
        assert header[:8] == b'QNLOB001'
        d,n,t,e,seed,source = struct.unpack('<6Q', header[8:])
        crc = zlib.crc32(header); remaining = (n+t+e)*d*4
        while remaining:
            block = f.read(min(1024*1024, remaining)); assert block
            crc = zlib.crc32(block, crc); remaining -= len(block)
        assert crc == struct.unpack('<I', f.read(4))[0] and not f.read(1)
    x = np.memmap(p, mode='r', dtype='<f4', offset=56, shape=(n+t+e,d))
    maps, stats, overlap = {}, [], {}
    for label, start, stop in [('corpus',0,n),('tuning',n,n+t),('evaluation',n+t,n+t+e)]:
        counts = Counter(); norms=[]; bad=zero=0
        for first in range(start,stop,1024):
            block=x[first:min(first+1024,stop)]
            norms.extend(np.linalg.norm(block.astype(np.float64),axis=1).tolist())
            bad += int(np.sum(~np.isfinite(block).all(axis=1)))
            zero += int(np.sum(np.all(block == 0,axis=1)))
            counts.update(hashlib.sha256(row.tobytes()).hexdigest() for row in block)
        maps[label]=counts
        stats.append(dict(split=label,rows=stop-start,nonfinite_rows=bad,zero_rows=zero,
                          duplicate_excess=sum(v-1 for v in counts.values()),
                          norm_min=min(norms),norm_p50=statistics.median(norms),norm_max=max(norms)))
    for a,b in [('corpus','tuning'),('corpus','evaluation'),('tuning','evaluation')]:
        common=maps[a].keys() & maps[b].keys()
        overlap[a+'__'+b]={'shared_unique_vectors':len(common),'affected_rows_in_second':sum(maps[b][h] for h in common)}
    meta=json.loads((ROOT/'data/ag-news/ag-news-100k-384.f32.json').read_text())
    raw=ROOT/'data/ag-news/ag-news-100k-384.f32'
    result=dict(path=str(p.relative_to(ROOT)),sha256=digest(p),crc32=f'{crc:08x}',dimension=d,seed=seed,source=source,
                raw_sha256_verified=digest(raw)==meta['output_hashes']['sha256'],splits=stats,exact_content_overlap=overlap,
                limitations=['Byte equality only; near duplicates, article identity and semantic relevance not established.',
                             'These are retrieval benchmark splits, not embedding-model training splits.'])
    save('corpus-audit.json',result)
    return result

def router_audit():
    p=ROOT/'research/data/raw/alpha5-router-heldout-rtx4050.tar.gz'
    assert digest(p)=='8e3b5fe6ec6ab9f295752ec894c89b094c4ddf4dd0ee38849d9d78c9a7fcc01e'
    errors=[]; cells={}; summaries=[]; missing=Counter(); total=0; config_flags=Counter()
    with tarfile.open(p) as t:
        def read(n): return t.extractfile(n).read().decode('utf-8')
        manifest=json.loads(read('alpha5-router-heldout-rtx4050/manifest.json'))
        for work in manifest['workloads']:
            name=work['name']; cells[name]={}
            signatures={}; truth_hashes={}
            for engine in ('cpu','gpu-rows','automatic'):
                base=f'alpha5-router-heldout-rtx4050/{name}/{engine}/'
                cfg=props(read(base+'configuration.txt')); summary=props(read(base+'summary.txt'))
                rows=list(csv.DictReader(io.StringIO(read(base+'samples.csv'))))
                runs=list(csv.DictReader(io.StringIO(read(base+'runs.csv'))))
                truth=list(csv.DictReader(io.StringIO(read(base+'truth.csv'))))
                metadata=list(csv.DictReader(io.StringIO(read(base+'metadata.csv'))))
                truth_hashes[engine]=hashlib.sha256(read(base+'truth.csv').encode()).hexdigest()
                signatures[engine]=tuple(cfg.get(k) for k in ('dataset_crc32','dimensions','rows','seed','order_seed','eligible_count','batch','k','filter_timestamp_from','filter_timestamp_to','filter_user_id'))
                if cfg.get('git_worktree_dirty')=='true': config_flags['dirty_source_cells']+=1
                if cfg.get('source_bundle_sha256')=='unavailable': config_flags['missing_source_bundle_hash_cells']+=1
                assert len({int(r['id']) for r in metadata}) == len(metadata) == work['rows']
                selected=set()
                for r in metadata:
                    if all(not cfg.get(k) or op(int(r[field]),int(cfg[k])) for k,field,op in [
                        ('filter_user_id','user_id',lambda a,b:a==b),('filter_timestamp_from','timestamp_micros',lambda a,b:a>=b),
                        ('filter_timestamp_to','timestamp_micros',lambda a,b:a<b)]): selected.add(int(r['id']))
                assert len(selected)==work['eligible_rows']
                for r in truth:
                    ids=[int(i) for i in r['ids'].split(';') if i]
                    assert len(ids)==len(set(ids))==min(work['k'],len(selected)) and set(ids)<=selected
                assert len(truth)==768
                groups=defaultdict(list); indices=defaultdict(list); seen=set(); order=[]
                for r in rows:
                    total+=1
                    missing.update(k for k,v in r.items() if v=='')
                    key=(int(r['run']),int(r['batch_index'])); assert key not in seen; seen.add(key)
                    v=int(r['batch_latency_ns']); assert v>0
                    query_ids=[int(i) for i in r['query_indices'].split(';')]
                    assert len(query_ids)==int(r['query_count'])==work['batch']
                    assert int(r['eligible_count'])==len(selected)
                    assert int(r['result_count'])==min(work['k'],len(selected))*len(query_ids)
                    assert 0<=float(r['recall_at_k'])<=1
                    groups[key[0]].append(v); indices[key[0]].extend(query_ids); order.append(r['query_indices'])
                assert len(groups)==len(runs)==5
                for ids in indices.values(): assert sorted(ids)==list(range(640))
                for r in runs: assert pct(groups[int(r['run'])],.95)==int(r['p95_batch_ns'])
                arr=np.array([pct(groups[i],.95) for i in sorted(groups)],dtype=float)
                med=float(np.median(arr)); assert med==float(summary['median_run_p95_batch_ns'])
                backend=sorted({r['actual_backend'] for r in rows}); assert len(backend)==1
                fallback=sum(r['fallback']!='false' for r in rows)
                p50=float(np.median([pct(v,.5) for v in groups.values()]))
                summaries.append(dict(cell=name,engine=engine,runs=5,batches_per_run=len(rows)//5,p95_us=med/1000,
                    p50_us=p50/1000,minimum_recall=min(float(r['recall_at_k']) for r in rows),fallback_batches=fallback,
                    max_run_p95_over_min=float(max(arr)/min(arr)),last_run_over_first=float(arr[-1]/arr[0])))
                cells[name][engine]=dict(p95=arr,median=med,p50=p50,backend=backend[0],order=order,work=work)
            assert len(set(signatures.values()))==1 and len(set(truth_hashes.values()))==1
            assert cells[name]['cpu']['order']==cells[name]['gpu-rows']['order']==cells[name]['automatic']['order']
    rng=np.random.default_rng(20260923); output=[]
    for name,c in cells.items():
        cpu,gpu,auto=(c[k] for k in ('cpu','gpu-rows','automatic'))
        def boots(v): return np.median(v[rng.integers(0,5,size=(20000,5))],axis=1)
        ratio=boots(cpu['p95'])/boots(gpu['p95'])
        lo,hi=np.quantile(ratio,[.025,.975]); slo,shi=np.quantile(ratio,[.05/32,1-.05/32])
        same=cpu if auto['backend']=='Cpu' else gpu
        best=min(cpu['median'],gpu['median'])
        output.append(dict(cell=name,**{k:cpu['work'][k] for k in ('dimension','batch','eligible_rows','k')},
            cpu_p95_us=cpu['median']/1000,gpu_p95_us=gpu['median']/1000,auto_p95_us=auto['median']/1000,
            cpu_over_gpu=cpu['median']/gpu['median'],ci95_lo=lo,ci95_hi=hi,
            bonferroni_percentile_lo=slo,bonferroni_percentile_hi=shi,
            auto_over_same_backend=auto['median']/same['median'],auto_backend=auto['backend'],
            cpu_regret=cpu['median']/best-1,gpu_regret=gpu['median']/best-1,
            candidate_choice_regret=same['median']/best-1,realized_auto_regret=auto['median']/best-1,
            p50_winner='cpu' if cpu['p50']<gpu['p50'] else 'gpu',p95_winner='cpu' if cpu['median']<gpu['median'] else 'gpu'))
    table('router-cells.csv',output); table('router-runs.csv',summaries)
    baseline=[]
    for key in ('cpu_regret','gpu_regret','candidate_choice_regret','realized_auto_regret'):
        v=[r[key] for r in output]
        baseline.append(dict(policy=key,median_regret=statistics.median(v),mean_regret=statistics.mean(v),max_regret=max(v),cells_above_25pct=sum(x>.25 for x in v)))
    result=dict(cells=16,engine_cells=48,sample_rows=total,checks='schema, numeric ranges, metadata joins, truth cardinality, split coverage, matched order, summary equality passed',
        source_flags=dict(config_flags),missing_fields=dict(missing),baselines=baseline,
        cpu_winners=sum(r['p95_winner']=='cpu' for r in output),
        auto_same_outside_25pct=sum(not .8<=r['auto_over_same_backend']<=1.25 for r in output),
        pointwise_ci_excludes_one=sum(r['ci95_lo']>1 or r['ci95_hi']<1 for r in output),
        simultaneous_percentile_excludes_one=sum(r['bonferroni_percentile_lo']>1 or r['bonferroni_percentile_hi']<1 for r in output),
        caveat='Whole-run independent bootstrap; only five consecutive runs per engine. Does not remove session/order confounding. Bonferroni percentile intervals are exploratory, not validated coverage with n=5. Auto/forced is not a true A/A control.')
    save('router-audit.json',result)
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    fig,axs=plt.subplots(1,2,figsize=(13,7),layout='constrained')
    y=np.arange(len(output)); vals=np.array([r['cpu_over_gpu'] for r in output])
    axs[0].errorbar(vals,y,xerr=[vals-np.array([r['ci95_lo'] for r in output]),np.array([r['ci95_hi'] for r in output])-vals],fmt='o',capsize=3)
    axs[0].set_yticks(y,[r['cell'] for r in output],fontsize=8); axs[0].set_xscale('log'); axs[0].axvline(1,color='black',lw=1)
    axs[0].set_xlabel('CPU / GPU median-run P95 (95% whole-run bootstrap)'); axs[0].set_title('Values below 1 favor CPU')
    axs[1].scatter([r['auto_over_same_backend'] for r in output],y)
    axs[1].axvspan(.8,1.25,alpha=.15,color='green'); axs[1].axvline(1,color='black',lw=1); axs[1].set_yticks(y,[])
    axs[1].set_xlabel('Automatic / forced same-backend P95'); axs[1].set_title('Path + state + order; not isolated overhead')
    fig.suptitle('Historical alpha.5 gate: 16 selected cells, 5 runs/engine; no causal inference')
    fig.savefig(OUT/'router-evidence.png',dpi=160); plt.close(fig)
    return result

def main():
    # Existing reducers are reused with output redirected into this new directory.
    p=ROOT/'research/scripts/analyze_full_archive.py'
    spec=importlib.util.spec_from_file_location('archive_reanalysis',p); mod=importlib.util.module_from_spec(spec); spec.loader.exec_module(mod)
    mod.OUT=OUT/'archive-reanalysis'; mod.OUT.mkdir(exist_ok=True)
    with (OUT/'archive-reanalysis.log').open('w',encoding='utf-8') as f:
        import contextlib
        with contextlib.redirect_stdout(f): mod.main()
    result={'corpus':corpus_audit(),'router':router_audit()}
    sources=[p,ROOT/'research/data/raw/alpha5-router-heldout-rtx4050.tar.gz',ROOT/'research/artifacts/runpod-small-2026-09-05/report/performance-matrix.csv',Path(__file__)]
    save('provenance.json',{'revision':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
        'python':sys.version,'numpy':np.__version__,'inputs':{str(p.relative_to(ROOT)):digest(p) for p in sources}})
    print(json.dumps(result,indent=2,default=lambda x:x.item()))

if __name__=='__main__': main()
