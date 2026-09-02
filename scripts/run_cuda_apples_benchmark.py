#!/usr/bin/env python3
"""Matched exact batch-1 Qenlo-CUDA, FAISS-GPU, and CPU baseline cohort."""
import argparse, csv, json, os, platform, subprocess, sys, time
from datetime import datetime, timezone
from pathlib import Path
import numpy as np
import torch


def stats(x):
    a=np.asarray(x,dtype=float)
    return {"count":int(a.size),"p50_ms":float(np.percentile(a,50)),"p95_ms":float(np.percentile(a,95)),"p99_ms":float(np.percentile(a,99)),"mean_ms":float(a.mean()),"throughput_qps":float(1000/a.mean())}
def main():
 p=argparse.ArgumentParser(); p.add_argument('--corpus',type=Path,required=True);p.add_argument('--queries',type=Path,required=True);p.add_argument('--output',type=Path,required=True);p.add_argument('--rows',type=int,default=1_000_000);p.add_argument('--dimensions',type=int,default=768);p.add_argument('--warmups',type=int,default=20);p.add_argument('--repetitions',type=int,default=5);a=p.parse_args()
 if a.output.exists(): raise SystemExit('refusing to overwrite output')
 if not torch.cuda.is_available(): raise SystemExit('CUDA unavailable')
 try:
  import faiss; import faiss.contrib.torch_utils
 except Exception as e: raise SystemExit(f'FAISS GPU import failed: {e}')
 qrows=a.queries.stat().st_size//(4*a.dimensions)
 if a.corpus.stat().st_size != a.rows*a.dimensions*4: raise SystemExit('corpus size mismatch')
 corpus=np.memmap(a.corpus,dtype='<f4',mode='r',shape=(a.rows,a.dimensions)); queries=np.memmap(a.queries,dtype='<f4',mode='r',shape=(qrows,a.dimensions))
 ids=np.arange(a.rows,dtype=np.int64); eligible=(ids%100)==0; candidate_ids=ids[eligible]; cand=np.array(corpus[eligible],copy=True)
 # Independent oracle: NumPy CPU exact IP, never FAISS/Torch GPU.
 oracle=[]
 for q in queries:
  s=cand@q; ix=np.argpartition(-s,10)[:10]; oracle.append(ix[np.argsort(-s[ix],kind='stable')])
 dev=torch.device('cuda:0'); cand_gpu=torch.from_numpy(cand).to(dev); query_gpu=torch.from_numpy(np.array(queries,copy=True)).to(dev); torch.cuda.synchronize()
 res=faiss.StandardGpuResources(); res.setDefaultNullStreamAllDevices(); index=faiss.GpuIndexFlatIP(res,0,a.dimensions); t0=time.perf_counter(); index.add(cand_gpu); torch.cuda.synchronize(); faiss_build=time.perf_counter()-t0
 def qenlo(q): return torch.topk(torch.mv(cand_gpu,q),10,largest=True,sorted=True).indices
 def fgpu(q): return index.search(q.reshape(1,-1),10)[1][0]
 def cpu(q):
  s=cand@q; ix=np.argpartition(-s,10)[:10]; return ix[np.argsort(-s[ix],kind='stable')]
 for i in range(a.warmups): qenlo(query_gpu[i%qrows]); fgpu(query_gpu[i%qrows]); cpu(queries[i%qrows])
 torch.cuda.synchronize(); samples=[]
 def run_gpu(name,fn):
  for rep in range(a.repetitions):
   for qi in range(qrows):
    st,en=torch.cuda.Event(True),torch.cuda.Event(True); wall=time.perf_counter(); st.record(); got=fn(query_gpu[qi]); en.record(); torch.cuda.synchronize(); kernel=float(st.elapsed_time(en)); e2e=(time.perf_counter()-wall)*1000; ids_out=np.asarray(got.cpu() if hasattr(got,'cpu') else got,dtype=np.int64); rec=len(set(ids_out).intersection(map(int,oracle[qi])))/10
    samples.append({'backend':name,'repetition':rep,'query':qi,'kernel_ms':kernel,'end_to_end_ms':e2e,'recall_at_10':rec})
 def run_cpu():
  for rep in range(a.repetitions):
   for qi in range(qrows):
    wall=time.perf_counter(); got=cpu(queries[qi]); elapsed=(time.perf_counter()-wall)*1000; rec=len(set(map(int,got)).intersection(map(int,oracle[qi])))/10
    samples.append({'backend':'cpu-numpy-exact','repetition':rep,'query':qi,'kernel_ms':None,'end_to_end_ms':elapsed,'recall_at_10':rec})
 run_gpu('qenlo-cuda-predicate-exact',qenlo); run_gpu('faiss-gpu-indexflatip-exact',fgpu); run_cpu()
 a.output.mkdir();
 with open(a.output/'raw_samples.csv','w',newline='') as f: w=csv.DictWriter(f,fieldnames=samples[0]);w.writeheader();w.writerows(samples)
 summary={}
 for b in sorted({r['backend'] for r in samples}):
  rows=[r for r in samples if r['backend']==b]; summary[b]={'end_to_end':stats([r['end_to_end_ms'] for r in rows]),'recall_at_10_min':min(r['recall_at_10'] for r in rows),'recall_at_10_mean':float(np.mean([r['recall_at_10'] for r in rows]))}
  if rows[0]['kernel_ms'] is not None: summary[b]['gpu_kernel']=stats([r['kernel_ms'] for r in rows])
 env={'gpu':torch.cuda.get_device_name(0),'torch':torch.__version__,'cuda':torch.version.cuda,'faiss':getattr(faiss,'__version__','unknown'),'python':sys.version,'platform':platform.platform()}
 m={'status':'completed','timestamp_utc':datetime.now(timezone.utc).isoformat(),'protocol':'matched candidate-conditioned exact IP; same preloaded 1%-eligible vectors, queries, k=10, and independent NumPy oracle','qenlo_status':'CUDA predicate prototype; not integrated Rust/WGPU core','workload':{'rows':a.rows,'dimensions':a.dimensions,'eligible_rows':int(eligible.sum()),'eligible_fraction':0.01,'batch':1,'k':10,'queries':qrows,'warmups':a.warmups,'repetitions':a.repetitions},'timing':'GPU kernel CUDA events and separate in-process end-to-end wall time; no RPC; corpus/index construction excluded from query timing','indexing_seconds':{'faiss_gpu_indexflatip':faiss_build},'memory':{'corpus_bytes':a.rows*a.dimensions*4,'candidate_bytes':int(cand.nbytes),'gpu_peak_allocated_bytes':int(torch.cuda.max_memory_allocated())},'environment':env,'summary':summary}
 (a.output/'manifest.json').write_text(json.dumps(m,indent=2)+'\n'); print(json.dumps(summary,indent=2))
if __name__=='__main__': main()
