"""Render paper figures from retained evidence; never rerun a benchmark.

Historical generators are imported with redirected output to preserve originals.
Every generated file has an input entry in figure-sources.json.
"""
from pathlib import Path
import csv
import json
import importlib.util
import hashlib
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import numpy as np

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'paper/figures/final'
MATRIX = ROOT / 'research/artifacts/runpod-small-2026-09-05/report/performance-matrix.csv'
DATA = ROOT / 'research/data/processed'
COLORS = ['#0072B2', '#D55E00', '#009E73', '#CC79A7', '#E69F00', '#595959']
LABELS = {'current-cpu':'Qenlo CPU', 'current-gpu':'Qenlo WGPU', 'faiss-flat':'FAISS Flat', 'numpy':'NumPy', 'torch-cpu':'Torch CPU', 'torch-cuda':'Torch CUDA'}
sources = []

def module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod

def record(name, inputs, note):
    sources.append(dict(figure=name, generator='paper/scripts/generate_final_figures.py', inputs=[dict(path=str(p.relative_to(ROOT)).replace('\\','/'), sha256=hashlib.sha256(p.read_bytes()).hexdigest()) for p in inputs], interpretation=note))

def save(fig, name, note, inputs=None):
    fig.tight_layout()
    fig.savefig(OUT / (name+'.pdf'), bbox_inches='tight', metadata={'CreationDate':None,'ModDate':None})
    fig.savefig(OUT / (name+'.png'), dpi=180, bbox_inches='tight')
    plt.close(fig)
    record(name,inputs or [MATRIX],note)

def main():
    OUT.mkdir(exist_ok=True,parents=True)
    plt.rcParams.update({'font.size':9, 'axes.spines.top':False,'axes.spines.right':False,'pdf.fonttype':42})
    rows = list(csv.DictReader(MATRIX.open(encoding='utf-8',newline='')))
    good = [r for r in rows if r['status']=='completed' and r['qualified']=='True']
    index = {(r['configuration'],r['workload'],r['engine']):r for r in good}
    fixed = [r for r in good if r['configuration'].endswith('deep768-admission-fix')]
    fig, axes = plt.subplots(1,2,figsize=(10,3.4),sharey=True)
    for ax,work in zip(axes, sorted(set(r['workload'] for r in fixed))):
        rr = {r['engine']:r for r in fixed if r['workload']==work}
        vals = [float(rr[e]['p95_completed_ns'])/1e6 for e in LABELS]
        ax.barh(list(LABELS.values()), vals, color=COLORS)
        for y,v in enumerate(vals): ax.text(v*1.06,y,f'{v:.3f}',va='center',fontsize=8)
        ax.set_xscale('log'); ax.set_xlim(.2,60)
        ax.set_xlabel('Completed-call P95 (ms; log scale)')
        ax.set_title('B=1, k=1, E/N=100%' if work.startswith('d4') else 'B=8, k=64, E/N=10%')
    axes[0].invert_yaxis()
    save(fig,'small_768','RTX 4090 corrected archive bb195fed; 100K x 768; no error bars; external filters prepared outside call; all oracle recall 1.')

    configs = ['a40-ca-mtl-secure','a40-eu-se-secure','rtx4090-eu-ro-secure-reference-retry']
    fig, axes = plt.subplots(1,3,figsize=(10,3.8),sharey=True)
    cells = ['c1-r1000-d128-b1-f1','c2-r1000-d384-b16-f001','c3-r10000-d128-b16-f1','c4-r10000-d384-b1-f001','c5-r100000-d128-b1-f001','c6-r100000-d384-b16-f1']
    for ax,conf,title in zip(axes,configs,['A40 / CA-MTL','A40 / EU-SE','RTX 4090 / EU-RO']):
        for ei,e in enumerate(['current-cpu','current-gpu']):
            pairs = [(i,index[(conf,c,e)]) for i,c in enumerate(cells) if (conf,c,e) in index]
            ax.scatter([i+(-.08 if ei==0 else .08) for i,r in pairs],[float(r['p95_completed_ns'])/1e6 for i,r in pairs],color=COLORS[ei],marker=['o','s'][ei],label=LABELS[e])
        ax.set_xticks(range(6),['C1','C2','C3','C4','C5','C6']); ax.set_title(title); ax.set_yscale('log'); ax.grid(axis='y',alpha=.2)
        ax.set_xlabel('Workload cell (see table)')
    axes[0].set_ylabel('Completed-call P95 (ms; log scale)'); axes[-1].legend(fontsize=8)
    save(fig,'small_common','Paired cells within each configuration only; dimensions/batches/selectivity differ across C1-C6. No interpolation or error bars; source 730c0500.')

    pairs=[]
    for (conf,work,e),r in index.items():
        base=index.get((conf,work,'baseline-gpu'))
        if e=='current-gpu' and base:
            pairs.append((conf,work,float(base['p95_completed_ns'])/float(r['p95_completed_ns'])))
    assert len(pairs)==12 and sum(p[2]>1 for p in pairs)==5
    fig, ax = plt.subplots(figsize=(9,4.3))
    names = [f'{i+1:02d}  '+c.replace('rtx4090-eu-ro-secure-','4090 / ').replace('-secure','').replace('rtx2000-eu-ro','2000 Ada')+' / '+w.split('-')[0] for i,(c,w,v) in enumerate(pairs)]
    ax.barh(names,[p[2] for p in pairs],color=[COLORS[2] if p[2]>1 else COLORS[1] for p in pairs]); ax.axvline(1,color='black',lw=.8)
    ax.invert_yaxis(); ax.set_xlabel('Frozen baseline P95 / candidate P95 (>1 favors candidate)')
    save(fig,'small_selector','12 qualified frozen-source/candidate pairs; descriptive; paired archive hashes differ; no uncertainty bars.')

    r=next(r for r in fixed if r['engine']=='current-gpu' and r['workload'].startswith('d5'))
    fields=['device_scoring_ns','device_selection_ns','backend_execution_ns','p95_completed_ns']
    fig, axes=plt.subplots(1,2,figsize=(9,3.4))
    axes[0].barh(['Scoring median','Selection median','Backend median','Completed-call P95'],[float(r[f])/1e6 for f in fields],color=[COLORS[0],COLORS[1],COLORS[5],COLORS[2]])
    axes[0].invert_yaxis(); axes[0].set_xlabel('Time (ms; overlapping scopes)')
    for i,r in enumerate([r for r in fixed if r['engine']=='current-gpu']):
        x=float(r['qenlo_allocation_bytes'])/1e6; y=float(r['p95_completed_ns'])/1e6
        axes[1].scatter(x,y,color=COLORS[i],s=55); axes[1].annotate('B=1, k=1' if r['batch']=='1' else 'B=8, k=64',(x,y),xytext=(0,12 if i==0 else -20),textcoords='offset points',ha='center',fontsize=8)
    axes[1].set(xlim=(309,317),ylim=(.89,.905),xlabel='Qenlo-owned accelerator allocation (MB)',ylabel='Completed-call P95 (ms)')
    save(fig,'small_resources','Corrected RTX 4090, d4/d5. Phase bars are nested scopes and unlike statistics, never summed. Allocation excludes process RSS; two distinct workloads, no fitted relationship.')

    life=[r for r in good if r['workload'].startswith('lifecycle:')]
    fig,ax=plt.subplots(figsize=(8,2.8))
    for i,r in enumerate(life):
        v=float(r['p95_completed_ns'])/1e6
        ax.barh(i,v,color=COLORS[1] if 'reopen' in r['workload'] else COLORS[0]);ax.text(v*1.1,i,f'{v:.3f}; rebuild {float(r["rebuilt_fraction"]):g}',va='center',fontsize=8)
    ax.set_yticks(range(len(life)),[r['workload'].split(':')[1] for r in life]); ax.invert_yaxis(); ax.set_xscale('log');ax.set_xlim(.1,700);ax.set_xlabel('First search after mutation or reopen (ms; log scale)')
    save(fig,'small_lifecycle','Source 730c0500, reference RTX 4090 10K x 384 B1 k10. Three observations/mutation and one reopen. Mutation and opening durations excluded; raw harness timers supersede broad matrix timing_scope. No error bars.')

    pairs=[]
    for (conf,work,e),r in index.items():
        if e=='current-gpu':
            for ce in ['chroma-hnsw','chroma']:
                cr=index.get((conf,work,ce))
                if cr: pairs.append((conf,work,float(cr['p95_completed_ns'])/float(r['p95_completed_ns']),cr['recall_at_k']))
    if len(pairs)!=7:
        pairs=[]
        for (conf,work,e),cr in index.items():
            r=index.get((conf,work,'current-gpu'))
            if e.startswith('chroma') and r:pairs.append((conf,work,float(cr['p95_completed_ns'])/float(r['p95_completed_ns']),cr['recall_at_k']))
    assert len(pairs)==7
    fig,ax=plt.subplots(figsize=(9,3.2))
    labels=[c.replace('rtx4090-eu-ro-secure-','4090 / ').replace('-secure','')+' / '+w.split('-')[0]+f'  (recall {float(rec):.4f})' for c,w,v,rec in pairs]
    ax.barh(labels,[v for c,w,v,rec in pairs],color=COLORS[0]);ax.set_xscale('log');ax.set_xlabel('Chroma P95 / Qenlo WGPU P95 (log scale)');ax.invert_yaxis()
    save(fig,'small_chroma','Seven qualified same-configuration/workload pairs; ANN recall printed; three gate failures excluded from ratios but retained in ledger. Distinct database and filter/API boundaries.')

    # Reuse original historical plotting functions in a new output directory.
    hist=module('qenlo_historical_plots',ROOT/'research/scripts/generate_plots.py');hist.FIG=OUT;hist.style()
    for fn, name, inputs in [('routing_regret','routing_regret',['static_router_regret.csv']),('eligibility_ablation','eligibility_ablation',['eligibility_ablation_summary.csv']),('windows_strategies','windows_strategies',['windows_summary.csv']),('android_matched','android_matched',['android_device_lab.csv']),('linux_context','linux_library_context',['linux_library_summary.csv'])]:
        getattr(hist,fn)();record(name,[DATA/p for p in inputs],'Historical cohort; original generator and original figure preserved. Lines are guides only. See manuscript caption for cohort and timing.')
    # Keep the matched localization sweep separate from older endpoint revisions.
    cross=list(csv.DictReader((DATA/'native_crossover_summary.csv').open()))
    a6=list(csv.DictReader((DATA/'a6000_exact_summary.csv').open()))
    fig,axes=plt.subplots(1,2,figsize=(9,3.3))
    for field,label,col in [('cpu_p95_ms','Qenlo CPU',COLORS[0]),('gpu_rows_p95_ms','WGPU compact rows',COLORS[1])]:
        axes[0].plot([int(r['eligible']) for r in cross],[float(r[field]) for r in cross],'o-',label=label,color=col)
    for system,label,col in [('faiss_gpu_flat_ip','FAISS GPU Flat',COLORS[0]),('qenlo_cuda_buffered_prototype','Research CUDA prototype',COLORS[1])]:
        rr=[r for r in a6 if r['system']==system]
        axes[1].plot([int(r['eligible']) for r in rr],[float(r['p95_ms']) for r in rr],'o-',label=label,color=col)
    for ax,title in zip(axes,['Historical RTX 4050 / 384D / B=1','Historical A6000 / 768D / B=1']):
        ax.set(xscale='log',yscale='log',xlabel='Eligible rows (log scale)',ylabel='P95 (ms; log scale)',title=title);ax.legend(fontsize=7)
    save(
        fig,
        'phase_map',
        'Separate panels, not pooled. Native matched revision 3e2a4a9 only; older endpoints deliberately omitted. A6000 pooled per-call descriptive P95 within cell, cross-implementation agreement only. Lines guide eyes.',
        [DATA/'native_crossover_summary.csv',DATA/'a6000_exact_summary.csv'],
    )

    archive=DATA/'archive-reanalysis'
    router_path=archive/'alpha5_heldout_router.csv'
    crossover_path=archive/'crossover_portability.csv'
    router=list(csv.DictReader(router_path.open()))
    portability=list(csv.DictReader(crossover_path.open()))
    fig,axes=plt.subplots(1,2,figsize=(10,3.6))
    groups=[('128D / B=8',[r for r in router if int(r['dimension'])==128 and int(r['rows'])==10000]),
            ('384D / B=1',[r for r in router if int(r['dimension'])==384]),
            ('768D / B=16',[r for r in router if int(r['dimension'])==768])]
    markers=['o','s','^']
    for (label,rr),marker,color in zip(groups,markers,COLORS):
        axes[0].scatter([float(r['work_units'])/1e6 for r in rr],
                        [float(r['gpu_p95_ns'])/float(r['cpu_p95_ns']) for r in rr],
                        label=label,marker=marker,color=color,s=46)
    other=[r for r in router if int(r['rows'])==1000]
    axes[0].scatter([float(r['work_units'])/1e6 for r in other],
                    [float(r['gpu_p95_ns'])/float(r['cpu_p95_ns']) for r in other],
                    label='1K dense control',marker='D',color=COLORS[5],s=46)
    axes[0].axvline(1,color='black',ls='--',lw=.9,label='preregistered threshold')
    axes[0].axhline(1,color='0.4',lw=.8)
    axes[0].set(xlabel=r'Scalar work $E D B$ (millions)',ylabel='GPU P95 / CPU P95',yscale='log',title='Held-out scalar-router test')
    axes[0].legend(fontsize=7)
    for cohort,label,color,marker in [
        ('h1-rtx4050-windows-dx12','RTX 4050 / DX12',COLORS[0],'o'),
        ('phase0-rtx4090-linux-vulkan','RTX 4090 / Vulkan',COLORS[1],'s')]:
        rr=[r for r in portability if r['cohort']==cohort]
        axes[1].plot([int(r['eligible_rows']) for r in rr],
                     [float(r['gpu_p95_ns'])/float(r['cpu_p95_ns']) for r in rr],
                     marker=marker,color=color,label=label)
    axes[1].axhline(1,color='0.4',lw=.8)
    axes[1].set(xlabel='Eligible rows E',ylabel='GPU P95 / CPU P95',xscale='log',yscale='log',title='Environment-conditioned crossover')
    axes[1].legend(fontsize=7)
    save(fig,'archive_router_failure','Left: 16 preregistered RTX 4050 workloads; near-equal work has different observed winners; monotone counterexamples and uncertainty are discussed in the text. Right: separate 384D, B=1, k=10 cohorts; lines guide the eye and latencies are never pooled. Ratio below one favors GPU. No error bars.',[router_path,crossover_path])

    e0e2 = DATA/'runpod-e0-e2-analysis-20260924'
    blocks_path, latency_path = e0e2/'block_metrics.csv', e0e2/'latency_summary.csv'
    blocks = list(csv.DictReader(blocks_path.open(encoding='utf-8')))
    latency = list(csv.DictReader(latency_path.open(encoding='utf-8')))
    fig, axes = plt.subplots(1, 3, figsize=(10.5, 3.4))
    for engine, color, marker in [('cpu', COLORS[0], 'o'), ('gpu-rows', COLORS[1], 's')]:
        rr = sorted((r for r in blocks if r['experiment'] == 'e0' and r['engine'] == engine), key=lambda r: int(r['block']))
        axes[0].plot([int(r['block']) for r in rr], [float(r['p95_ns'])/1e6 for r in rr], marker=marker, color=color, label=engine)
    axes[0].set(xlabel='E0 block (execution order)', ylabel='Block P95 (ms)', title='E0: B=1, E=3,000 repeated')
    axes[0].legend(fontsize=7)
    engines = [('cpu', 'CPU', COLORS[0], 'o'), ('gpu-rows', 'rows', COLORS[1], 's'), ('gpu-predicate', 'predicate', COLORS[2], '^'), ('gpu-mask', 'mask', COLORS[3], 'D')]
    for ax, batch in zip(axes[1:], ('1', '16')):
        for engine, label, color, marker in engines:
            rr = sorted((r for r in latency if r['experiment'] == 'e2' and r['batch'] == batch and r['engine'] == engine and r['metric'] == 'p95_ns'), key=lambda r: float(r['fraction']))
            x = [float(r['fraction']) for r in rr]
            y = [float(r['median_ns'])/1e6 for r in rr]
            lo = [v - float(r['ci95_low_ns'])/1e6 for v, r in zip(y, rr)]
            hi = [float(r['ci95_high_ns'])/1e6 - v for v, r in zip(y, rr)]
            ax.errorbar(x, y, yerr=[lo, hi], marker=marker, color=color, label=label, capsize=2, lw=1, ms=4)
        ax.set(xscale='log', yscale='log', xlabel='Eligible fraction f', ylabel='Median block P95 (ms)', title=f'E2: B={batch}' + (' (f=0.3 partial, f=1 missing)' if batch == '16' else ''))
        ax.legend(fontsize=7)
    save(fig, 'e0e2_noise_representation', 'Partial E0/E2 archive e4ca1336 (227/260 summaries). Left: E0 block P95, all blocks retained. Middle/right: E2 median block P95 with 95% block-bootstrap intervals; missing cells absent, not interpolated.', [blocks_path, latency_path])
    (ROOT/'paper/audit/figure-sources.json').write_text(json.dumps(sources,indent=2)+'\n',newline='\r\n')
    print(f'Generated {len(sources)} figure pairs')

if __name__=='__main__':main()
