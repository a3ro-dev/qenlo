"""Generate all numerical manuscript tables from audited CSV reductions."""
from pathlib import Path
import csv, json
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'paper/tables/final'
DATA=ROOT/'paper/audit/reduced/historical'
def read(p):return list(csv.DictReader(p.open(newline='',encoding='utf-8')))
def esc(x):return str(x).replace('&',r'\&').replace('_',r'\_').replace('%',r'\%')
def write(name, rows):
    (OUT/(name+'.tex')).write_text('\n'.join(' & '.join(map(esc,r))+r' \\' for r in rows)+'\n',encoding='utf-8')
def main():
    OUT.mkdir(parents=True,exist_ok=True)
    m=read(ROOT/'research/artifacts/runpod-small-2026-09-05/report/performance-matrix.csv')
    defs=[]
    for r in m:
        if r['configuration']=='rtx4090-eu-ro-secure-reference-retry' and r['workload'].startswith('c') and r['engine']=='current-gpu' and r['status']=='completed':
            defs.append((r['workload'].split('-')[0].upper(),f"{int(r['rows']):,}",r['dimensions'],r['batch'],f"{100*float(r['eligible_fraction']):g}%"))
    assert len(defs)==6
    write('common_definitions',defs)
    labels={'current-cpu':'Qenlo CPU','current-gpu':'Qenlo WGPU','faiss-flat':'FAISS Flat','numpy':'NumPy','torch-cpu':'Torch CPU','torch-cuda':'Torch CUDA'}
    f=[r for r in m if r['configuration'].endswith('deep768-admission-fix') and r['status']=='completed']
    write('headline',[(('Full, B=1, k=1' if r['workload'].startswith('d4') else '10%, B=8, k=64'),labels[r['engine']],f"{float(r['p50_completed_ns'])/1e6:.3f}",f"{float(r['p95_completed_ns'])/1e6:.3f}",r['recall_at_k'],'192 / 3' if r['batch']=='1' else '24 / 3') for r in sorted(f,key=lambda r:(r['workload'],list(labels).index(r['engine'])))])
    for name,fields in [('windows_summary',['eligible','system','p95_ms','recall_at_10']),('native_crossover_summary',['eligible','cpu_p95_ms','gpu_rows_p95_ms','recall_at_10']),('eligibility_ablation_summary',['eligible','mode','median_run_p50_ms','median_run_p95_ms','median_run_p99_ms','recall_at_10']),('a6000_exact_summary',['eligible','system','p50_ms','p95_ms','p99_ms']),('a6000_strict_summary',['system','p50_ms','p95_ms','p99_ms','recall_at_10']),('linux_library_summary',['system','search_type','p95_ms','recall_at_10']),('android_device_lab',['suite','cell','rows','batch','samples','p50_ms','p95_ms','p99_ms'])]:
        rows=[]
        for r in read(DATA/(name+'.csv')):
            values=[]
            for field in fields:
                v=r[field]
                if field.endswith('_ms'):v=f'{float(v):.4f}'
                elif 'recall' in field:v=f'{float(v):.5f}'
                v=v.replace('qenlo_cuda_buffered_prototype','CUDA prototype').replace('faiss_gpu_flat_ip','FAISS GPU Flat').replace('CPU exhaustive FP64 accum.','CPU exhaustive').replace('research CUDA exhaustive prototype','CUDA prototype').replace('cuVS brute force (disqualified)','cuVS (disqualified)')
                values.append(v)
            rows.append(values)
        write(name,rows)
    good={(r['configuration'],r['workload'],r['engine']):r for r in m if r['status']=='completed' and r['qualified']=='True'}
    pairs=[]
    for (c,w,e),r in good.items():
        if e=='current-gpu' and (c,w,'baseline-gpu') in good:
            ratio=float(good[(c,w,'baseline-gpu')]['p95_completed_ns'])/float(r['p95_completed_ns'])
            pairs.append((c.replace('rtx4090-eu-ro-secure-','4090 / ').replace('-secure',''),w,f'{ratio:.3f}'))
    write('selector',pairs)
    write('real_small',[(r['engine'],f"{float(r['p95_completed_ns'])/1e6:.4f}",f"{float(r['recall_at_k']):.5f}") for r in m if r['workload'].startswith('real-') and r['status']=='completed'])
    configs=['a40-ca-mtl-secure','a40-eu-se-secure','rtx4090-eu-ro-secure-reference-retry']
    common=[]
    for c in configs:
        for w in sorted(set(r['workload'] for r in m if r['configuration']==c and r['workload'].startswith('c'))):
            cpu=good.get((c,w,'current-cpu'));gpu=good.get((c,w,'current-gpu'))
            if cpu or gpu:common.append((c.replace('rtx4090-eu-ro-secure-reference-retry','4090 EU-RO').replace('-secure',''),w.split('-')[0].upper(),*(f"{float(r['p95_completed_ns'])/1e6:.4f}" if r else '--' for r in [cpu,gpu])))
    write('common',common)
    short={'cpu-exact-all':'CPU exhaustive','gpu-exact-all':'WGPU exhaustive','gpu-native-batch-8':'WGPU batch 8','gpu-ivf-flat-recall-95':'IVF-Flat','gpu-ivf-sq8-recall-95':'IVF-SQ8','automatic-selective-cpu-route':'Auto selective CPU','automatic-selective-batch-8':'Auto selective WGPU'}
    arc=json.loads((ROOT/'benchmarks/2026-08-31/device-lab/intel-arc/reports.json').read_text())
    write('intel_arc',[(report['suite']+str(i+1) if report['suite']=='quick' else report['suite'],short[c['name']],c['rows'],c['batch_size'],c['samples'],f"{c['p50_us']/1000:.3f}",f"{c['p95_us']/1000:.3f}",f"{c['p99_us']/1000:.3f}") for i,report in enumerate(arc) for c in report['cells']])
    print('Generated numerical table bodies')
if __name__=='__main__':main()
