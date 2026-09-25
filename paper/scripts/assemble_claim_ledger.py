"""Combine reviewed claim records and all matrix rows into the paper ledger."""
from pathlib import Path
import csv, json, hashlib
ROOT=Path(__file__).resolve().parents[2]
def main():
    audit=ROOT/'paper/audit'
    components={k:json.loads((audit/(k+'-ledger.json')).read_text(encoding='utf-8')) for k in ['historical','campaign','system','citation']}
    archive_summary_path=ROOT/'research/data/processed/archive-reanalysis/summary.json'
    archive_summary=json.loads(archive_summary_path.read_text(encoding='utf-8'))
    archive_sources=[
        'research/data/processed/archive-reanalysis/summary.json',
        'research/data/processed/archive-reanalysis/crossover_portability.csv',
        'research/data/processed/archive-reanalysis/alpha5_heldout_router.csv',
        'research/data/processed/archive-reanalysis/phase2_cpu_optimization.csv',
        'research/data/processed/archive-reanalysis/sample_series_inventory.csv',
    ]
    components['archive_reanalysis']={
        'schema':'qenlo-archive-reanalysis-claims-v1',
        'generated_by':'research/scripts/analyze_full_archive.py',
        'claims':[
            {'id':f'AR-{i:02d}','exact_claim':claim,'source_artifact':archive_sources,
             'qualification':'Retained evidence only; cohorts remain separate; post-hoc thresholds are diagnostic.'}
            for i,claim in enumerate(archive_summary['claims'],1)
        ],
        'source_archives':archive_summary['source_archives'],
        'sample_series_inventory':archive_summary['sample_series_inventory'],
        'limitations':archive_summary['limitations'],
    }
    e0e2=ROOT/'research/data/processed/runpod-e0-e2-analysis-20260924'
    mech=ROOT/'research/data/processed/runpod-e0-e2-mechanisms-20260924'
    def art(*paths):
        return [{'path':str(p.relative_to(ROOT)).replace('\\','/'),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()} for p in paths]
    archive=ROOT/'research/data/raw/runpod-e0-e2-partial-20260924.tar.gz'
    e0e2_claims=[
        ('E0E2-01','Partial campaign: 227 of 260 planned block summaries (E0 20/20, E2 207/240), no completion marker; 13 missing at B=16/E=30,000 and 20 at B=16/E=100,000.',art(archive,e0e2/'coverage.csv',mech/'audit.json'),'Never described as complete; no imputation.'),
        ('E0E2-02','One included gpu-mask block (B=1/E=100/block 2) exited 101 after writing its summary; one CPU destination (B=16/E=30,000/block 1) has no summary and counts as missing.',art(e0e2/'block_metrics.csv',mech/'audit.json'),'Included and flagged; omission changes rows/mask P95 ratio 0.197 to 0.198.'),
        ('E0E2-03','All 20 B=1/E=100,000 summaries (four engines, CPU included) report recall@10 exactly 0.99998; the other 207 report 1.',art(mech/'audit.json'),'Reported unrounded; oracle-boundary property of the dense cell.'),
        ('E0E2-04','E0 same-cell block ratios reach about 1.94x (P50) and 1.99x (P95) at the all-pairs 95th percentile; gpu-rows is bimodal.',art(e0e2/'e0_noise.csv',e0e2/'block_metrics.csv'),'One cell (B=1, E=3,000), one host; 45 dependent pairs are descriptive.'),
        ('E0E2-05','Faster GPU blocks are associated with higher end-of-run SM clock snapshots (80 concordant, 31 discordant, 114 tied within-cell pairs).',art(mech/'gpu_clock_snapshots_b1.csv',mech/'audit.json'),'Association from before/after snapshots; DVFS causation is a hypothesis.'),
        ('E0E2-06','At f<=0.01 and B in {1,16}, gpu-rows is 3.2-6.1x faster than gpu-predicate and gpu-mask (paired P50/P95), clearing the conservative E0 screen; predicate is faster at B=1/f=1 without clearing it.',art(e0e2/'paired_comparisons.csv'),'Single corpus, predicate shape, N, D, k; no f*(16) estimate.'),
        ('E0E2-07','Same-host ordering witness: gpu-rows wins at EDB=614,400 (B=16/E=100, 5/5 blocks) while CPU wins at EDB=1,152,000 (B=1/E=3,000, 9/10 E0 blocks).',art(mech/'edb_ordering_witness.csv',e0e2/'paired_comparisons.csv'),'Post hoc; E0 ratio does not clear the same-engine A/A band.'),
        ('E0E2-08','In the measured binary, host row materialization is 2.206 of 4.333 ms (51%) of the median gpu-rows call at B=1/E=30,000 and is also paid by predicate and mask modes.',art(mech/'diagnostics_by_cell.csv'),'Overlapping counters; mechanism verified in measured source, not causally decomposed. Audited source now avoids the unused list for required predicates.'),
        ('E0E2-09','qenlo-bench summed per-response batch-total upload/readback/lock-wait fields, overstating batch-B values B-fold (E2 B=16 upload 6,794,240 = 16 x 424,640; S2 batch-8 517,120 = 8 x 64,640).',art(mech/'diagnostics_by_cell.csv',ROOT/'crates/qenlo-bench/src/main.rs'),'Fixed in run format v4; retained raw fields unchanged; latency unaffected.'),
        ('E0E2-10','Required shader-predicate batches now count eligibility without materializing or sorting an unused host row list.',art(ROOT/'crates/qenlo-core/src/lib.rs',ROOT/'crates/qenlo/src/lib.rs'),'Behavior and diagnostics tested locally; automatic predicate execution still retains fallback rows; no campaign-host speedup claim.'),
        ('E0E2-11','The E0/E2 runner now refuses to resume past an existing summary unless its sibling run record exists, has exit code zero, and the summary passes completion and recall gates.',art(ROOT/'research/scripts/run_e0_e2_runpod.py'),'Future-run integrity change only; retained exit-101 evidence is untouched and remains included/flagged.'),
    ]
    components['e0_e2_partial']={
        'schema':'qenlo-e0-e2-partial-claims-v1',
        'generated_by':['research/data/processed/runpod-e0-e2-analysis-20260924/analyze.py','research/scripts/audit_e0_e2_mechanisms.py'],
        'archive_sha256':hashlib.sha256(archive.read_bytes()).hexdigest(),
        'claims':[{'id':i,'exact_claim':c,'source_artifact':a,'qualification':q} for i,c,a,q in e0e2_claims],
    }
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
    target=ROOT/'paper/tables/claim-to-artifact.json';target.write_text(json.dumps(output,indent=2)+'\n',encoding='utf-8',newline='\r\n')
    flat=[]
    for i,c in enumerate(claims):
        flat.append({'claim_id':c.get('id',c.get('claim_id',f'claim-{i+1}')),'claim':c.get('exact_claim',c.get('claim',str(c.get('statement','See full ledger')))),'evidence':json.dumps(c.get('source_artifact',c.get('sources',c.get('source',[])))),'component':c['ledger_component']})
    flat += [{'claim_id':c['claim_id'],'claim':c['claim'],'evidence':c['source_artifact']+' row '+str(c['matrix_row']),'component':'campaign-row'} for c in entries]
    with (ROOT/'paper/tables/claim-to-artifact.csv').open('w',newline='',encoding='utf-8') as f:
        w=csv.DictWriter(f,fieldnames=['claim_id','claim','evidence','component'],lineterminator='\r\n');w.writeheader();w.writerows(flat)
    print(f'{len(claims)} audited claim records, {len(entries)} campaign rows')
if __name__=='__main__':main()
