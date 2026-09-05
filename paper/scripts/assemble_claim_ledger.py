"""Combine reviewed claim records and all matrix rows into the paper ledger."""
from pathlib import Path
import csv, json, hashlib
ROOT=Path(__file__).resolve().parents[2]
def main():
    audit=ROOT/'paper/audit'
    components={k:json.loads((audit/(k+'-ledger.json')).read_text(encoding='utf-8')) for k in ['historical','campaign','system','citation']}
    claims=[]
    for kind,d in components.items():
        for i,c in enumerate(d.get('claims',[])):
            claims.append({'ledger_component':kind,**c})
    matrix=ROOT/'research/artifacts/runpod-small-2026-09-05/report/performance-matrix.csv'
    entries=[]
    campaign=ROOT/'research/artifacts/runpod-small-2026-09-05'
    def props(p):
        return dict(line.split('=',1) for line in p.read_text(encoding='utf-8').splitlines() if '=' in line)
    with matrix.open(newline='',encoding='utf-8') as f:
        for i,r in enumerate(csv.DictReader(f),1):
            entry={'claim_id':f'S-row-{i:03d}','claim':f"{r['configuration']} / {r['workload']} / {r['engine']}: status {r['status']}, qualified {r['qualified']}, P95 {r['p95_completed_ns'] or 'unmeasured'} ns, recall {r['recall_at_k'] or 'unmeasured'}",
              'source_artifact':str(matrix.relative_to(ROOT)).replace('\\','/'),'matrix_row':i,
              'source_revision_or_archive_hash':r['source_bundle_sha256'] or 'not recorded / unavailable',
              'environment':{'gpu':r['gpu'],'data_center':r['data_center'],'cloud':r['cloud'],'configuration':r['configuration'],'detail':'Per-configuration environment.txt and completion record; campaign audit gives corrected hardware/software.'},
              'dataset_provenance':'Retained configuration/summary/truth and QNB checksum; AG News when workload starts real-, synthetic otherwise; lifecycle is 10K x 384 durable fixture.',
              'N':r['rows'],'D':r['dimensions'],'batch':r['batch'],'k':r['k'],'eligible_fraction':r['eligible_fraction'],
              'filter_representation':r['filter_scope'] or 'not recorded / unavailable',
              'timing_boundary':r['timing_scope'],'sample_count':r['sample_count'],
              'sample_unit':'repetitions' if r['engine'] in ['numpy','faiss-flat','torch-cpu','torch-cuda'] or r['engine'].startswith('chroma') else 'native batch calls or lifecycle observations',
              'correctness_oracle':'FP64 truth for qualified search; lifecycle uses deletion/rebuild checks. Failed/unavailable is not qualified.',
              'memory_scope':{'process_rss_high_water_bytes':r['peak_process_rss_bytes'],'owned_allocation_bytes':r['qenlo_allocation_bytes'],'interpretation':'Native accelerator allocation and adapter tensor allocation are distinct; never add to RSS.'},
              'qualification_status':{'status':r['status'],'qualified':r['qualified'],'failure':r['failure_detail']},
              'limitations':['No cross-revision pooling. current-gpu denotes archived candidate, not final local selector.','External matrix filtering precedes query; replay sample_count is repetitions.','No significance or universal deployment inference.'],'raw_matrix_fields':r}
            if r['workload'].startswith('lifecycle:'):
                entry['timing_boundary']='Search-only after mutation/reopen; mutation and Collection::open timed separately. Raw harness supersedes broad matrix label.'
            conf_dirs=list(campaign.glob(f"*/{r['configuration']}"))
            candidates=[]
            for conf_dir in conf_dirs:
                entry['configuration_artifacts']=[str(p.relative_to(ROOT)).replace('\\','/') for p in conf_dir.glob('*.json')]
                candidates+=list(conf_dir.glob(f"**/runs/{r['workload']}/{r['engine']}"))
                if r['workload'].startswith('lifecycle:'):
                    candidates+=list(conf_dir.glob('**/lifecycle-current-gpu'))
            if candidates:
                raw=candidates[0]
                artifacts=[]
                for name in ['configuration.txt','configuration.json','summary.txt','summary.json','samples.csv','runs.csv','truth.csv','metadata.csv','lifecycle.csv']:
                    p=raw/name
                    if p.exists():artifacts.append({'path':str(p.relative_to(ROOT)).replace('\\','/'),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()})
                entry['raw_artifacts']=artifacts
                cfg=raw/'configuration.txt';summ=raw/'summary.json'
                if cfg.exists():
                    values=props(cfg);entry['retained_configuration']=values
                    entry['dataset_provenance']={k:v for k,v in values.items() if any(t in k for t in ['dataset','seed','corpus','tuning','evaluation','metadata'])}
                    entry['filter_representation']={k:v for k,v in values.items() if any(t in k for t in ['filter','fraction','eligible','predicate'])}
                if summ.exists():
                    values=json.loads(summ.read_text());entry['retained_summary']=values
                    entry['environment']['adapter_software']={k:values.get(k) for k in ['package','package_version','python','platform','device']}
                    entry['dataset_provenance']={'sha256':values.get('dataset_sha256'),'reference_configuration_sha256':values.get('reference_configuration_sha256'),'scope':'Prefiltered subset of retained canonical reference data.'}
                run=raw/'runs.csv'
                if run.exists():
                    runrows=list(csv.DictReader(run.open(encoding='utf-8')));entry['repetitions']=len(runrows)
            entries.append(entry)
    output={'schema':'qenlo-definitive-paper-claims-v1','components':components,'headline_claims':claims,'campaign_rows':entries,'source_matrix_sha256':hashlib.sha256(matrix.read_bytes()).hexdigest(),'figure_sources':json.loads((audit/'figure-sources.json').read_text()),'scope':'Research synthesis only; historical and experimental source roles are distinct from final worktree.'}
    target=ROOT/'paper/tables/claim-to-artifact.json';target.write_text(json.dumps(output,indent=2)+'\n',encoding='utf-8')
    flat=[]
    for i,c in enumerate(claims):
        flat.append({'claim_id':c.get('id',c.get('claim_id',f'claim-{i+1}')),'claim':c.get('exact_claim',c.get('claim',str(c.get('statement','See full ledger')))),'evidence':json.dumps(c.get('source_artifact',c.get('sources',c.get('source',[])))),'component':c['ledger_component']})
    flat += [{'claim_id':c['claim_id'],'claim':c['claim'],'evidence':c['source_artifact']+' row '+str(c['matrix_row']),'component':'campaign-row'} for c in entries]
    with (ROOT/'paper/tables/claim-to-artifact.csv').open('w',newline='',encoding='utf-8') as f:
        w=csv.DictWriter(f,fieldnames=['claim_id','claim','evidence','component']);w.writeheader();w.writerows(flat)
    print(f'{len(claims)} audited claim records, {len(entries)} campaign rows')
if __name__=='__main__':main()
