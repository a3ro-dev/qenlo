"""Bounded native benchmark queue. Dry-run default; never provisions cloud resources."""
from __future__ import annotations
import argparse, csv, ctypes, hashlib, io, json, os, platform, random, shutil, subprocess, tarfile, time
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]

def sha(p):
    with p.open('rb') as f: return hashlib.file_digest(f,'sha256').hexdigest()

def capture(cmd):
    try:
        p=subprocess.run(cmd,capture_output=True,text=True,timeout=20)
        return {'returncode':p.returncode,'stdout':p.stdout,'stderr':p.stderr}
    except (OSError,subprocess.TimeoutExpired) as e: return {'error':str(e)}

def free_ram():
    if os.name=='nt':
        class Memory(ctypes.Structure):
            _fields_=[('length',ctypes.c_ulong),('load',ctypes.c_ulong)]+[(n,ctypes.c_ulonglong) for n in ('total','available','page_total','page_available','virtual_total','virtual_available','extended')]
        m=Memory(); m.length=ctypes.sizeof(m)
        if not ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(m)): raise OSError('memory query failed')
        return m.available
    fields=dict(line.split(':',1) for line in Path('/proc/meminfo').read_text().splitlines())
    return int(fields['MemAvailable'].split()[0])*1024

def plan(kind):
    # name, corpus rows, dimensions, batch, eligible, k
    if kind=='smoke':
        return [('tiny',256,16,8,5,10),('empty',256,16,1,0,10)], [20260923], 1, 32, [('cpu','cpu','one-pass'),('gpu','gpu-rows','one-pass')]
    if kind=='E1':
        cells=[('failure',10000,128,8,1221,10),('weak-witness',10000,768,16,61,10),('near-tie',10000,384,1,3906,1),('all-rows',1000,128,8,1000,10)]
        arms=[('cpu-a','cpu','one-pass'),('cpu-b','cpu','one-pass'),('gpu-a','gpu-rows','one-pass'),('gpu-b','gpu-rows','one-pass'),('auto','automatic','one-pass')]
    elif kind=='E2':
        cells=[(f'n{n}-e{e}',n,384,1,e,10) for n in (10000,100000) for e in (1000,4000,6000)]
        arms=[('cpu','cpu','one-pass')]+[(m,'gpu-rows',m) for m in ('legacy-two-pass','one-pass','cached')]
    else:
        cells=[(f'd{d}-b{b}-k{k}',10000,d,b,e,k) for d,b,e in ((384,1,2048),(128,8,768),(768,16,64)) for k in (1,10,64)]
        arms=[('cpu','cpu','one-pass'),('gpu','gpu-rows','one-pass')]
    return cells,[20261001,20261002],4,4096,arms

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--experiment',choices=['smoke','E1','E2','E3'],required=True)
    p.add_argument('--binary',type=Path,default=ROOT/'target/release'/('qenlo-bench.exe' if os.name=='nt' else 'qenlo-bench'))
    p.add_argument('--output',type=Path,required=True)
    p.add_argument('--execute',action='store_true',help='Run only after resource approval, except inexpensive smoke.')
    p.add_argument('--max-minutes',type=float,default=90)
    p.add_argument('--min-free-ram-gib',type=float,default=4)
    args=p.parse_args(); args.output=args.output.resolve(); args.binary=args.binary.resolve()
    if args.max_minutes<=0 or args.min_free_ram_gib<0: p.error('invalid resource bound')
    cells,seeds,blocks,evaluation,arms=plan(args.experiment)
    jobs=[]; prepared=set(); tuning=32 if args.experiment=='smoke' else 128
    # All references are untimed controls outside randomized measurement blocks.
    for seed in seeds:
        for name,n,d,b,e,k in cells:
            data=args.output/f'data-n{n}-d{d}-s{seed}.qnb'
            if data not in prepared:
                jobs.append(('prepare',str(data),[str(args.binary),'prepare','--dataset',str(data),'--rows',str(n),'--dimensions',str(d),'--tuning',str(tuning),'--evaluation',str(evaluation),'--seed',str(seed)]))
                prepared.add(data)
            def command(out,backend,mode,block,reference=None):
                cmd=[str(args.binary),'run','--dataset',str(data),'--output',str(out),'--dimensions',str(d),'--backend',backend,
                     '--eligible-count',str(e),'--batch',str(b),'--k',str(k),'--warmups','8' if args.experiment=='smoke' else '128',
                     '--repetitions','1','--order-seed',str(seed+block),'--recall-target','0.99','--gpu-row-preparation',mode,
                     '--diagnostics','detailed','--distribution','independent','--vector-budget-mib','512','--gpu-budget-mib','512']
                if reference: cmd+=['--oracle-reference',str(reference)]
                return cmd
            ref=args.output/f's{seed}-{name}'/'reference'
            jobs.append(('reference',str(ref),command(ref,'cpu','one-pass',0)))
            for block in range(blocks):
                ordered=list(arms); random.Random(seed+block*101+sum(map(ord,name))).shuffle(ordered)
                for label,backend,mode in ordered:
                    out=args.output/f's{seed}-{name}'/f'block-{block}'/label
                    jobs.append(('measurement',str(out),command(out,backend,mode,block,ref)))
    spec={'experiment':args.experiment,'seeds':seeds,'blocks_per_seed':blocks,'evaluation_queries':evaluation,
          'calls':len(jobs),'measurement_calls':sum(j[0]=='measurement' for j in jobs),'jobs':jobs,
          'max_minutes':args.max_minutes,'min_free_ram_gib':args.min_free_ram_gib,
          'note':'Separate fresh processes; preparation/oracle outside timer. No pooling with historical data.'}
    if not args.execute:
        print(json.dumps(spec,indent=2)); return
    if not args.binary.is_file(): raise SystemExit('Build a provenance-recorded gpu-wgpu binary first.')
    args.output.mkdir(parents=True,exist_ok=True)
    spec['binary_sha256']=sha(args.binary)
    # Bind resume to the runner and tracked Rust inputs, not merely to a dirty Git HEAD.
    inputs=subprocess.check_output(['git','ls-files','crates','Cargo.toml','Cargo.lock','rust-toolchain.toml'],cwd=ROOT,text=True).splitlines()
    spec['source_hashes']={n:sha(ROOT/n) for n in inputs if (ROOT/n).is_file()}
    spec['runner_sha256']=sha(Path(__file__))
    bundle=args.output/'source-snapshot.tar'
    if not bundle.exists():
        with tarfile.open(bundle,'w') as archive:
            for name in sorted(spec['source_hashes']):
                contents=(ROOT/name).read_bytes(); info=tarfile.TarInfo(name); info.size=len(contents)
                archive.addfile(info,io.BytesIO(contents))
    with tarfile.open(bundle) as archive:
        archived={m.name:hashlib.sha256(archive.extractfile(m).read()).hexdigest() for m in archive if m.isfile()}
    if archived!=spec['source_hashes']: raise SystemExit('Source snapshot mismatch; use a fresh directory.')
    spec['source_bundle_sha256']=sha(bundle)
    config=args.output/'queue.json'
    if config.exists():
        if json.loads(config.read_text())!=json.loads(json.dumps(spec)): raise SystemExit('Resume identity mismatch; use a new directory.')
    else: config.write_text(json.dumps(spec,indent=2)+'\n')
    stamp=str(time.time_ns())
    env=os.environ.copy(); env.setdefault('WGPU_BACKEND','dx12' if os.name=='nt' else 'vulkan')
    env['QENLO_SOURCE_BUNDLE_SHA256']=spec['source_bundle_sha256']
    environment={'platform':platform.platform(),'free_ram':free_ram(),'free_disk':shutil.disk_usage(args.output).free,
                 'gpu':capture(['nvidia-smi']),'rust':capture(['rustc','--version']),
                 'cargo':capture(['cargo','--version']),'wgpu_backend':env['WGPU_BACKEND'],
                 'source_identity_kind':'tracked Rust source snapshot and separate binary hash; build correspondence requires retained build log',
                 'git_status':capture(['git','status','--short'])}
    (args.output/f'environment-{stamp}.json').write_text(json.dumps(environment,indent=2))
    started=time.monotonic()
    budget=args.output/'compute-budget.json'
    spent=json.loads(budget.read_text())['seconds'] if budget.exists() else 0
    for index,(kind,destination,cmd) in enumerate(jobs):
        dest=Path(destination); marker=args.output/f'checkpoint-{index:04d}.json'
        if marker.exists():
            record=json.loads(marker.read_text())
            if all(Path(n).is_file() and sha(Path(n))==h for n,h in record['hashes'].items()): continue
            raise SystemExit(f'Changed completed output: {dest}')
        if dest.exists(): raise SystemExit(f'Incomplete output retained: {dest}; inspect it, then use a fresh output directory. Never silently overwrite.')
        if free_ram()<args.min_free_ram_gib*2**30: raise SystemExit('Insufficient free host RAM; checkpoint retained.')
        if shutil.disk_usage(args.output).free<5*2**30: raise SystemExit('Less than 5 GiB disk free.')
        remaining=min(args.max_minutes*60-spent,args.max_minutes*60-(time.monotonic()-started))
        if remaining<=0: raise SystemExit('Time budget reached; checkpoint retained.')
        if sha(args.binary)!=spec['binary_sha256']: raise SystemExit('Binary changed during run.')
        gpu=capture(['nvidia-smi','--query-gpu=memory.free,temperature.gpu','--format=csv,noheader,nounits'])
        (args.output/f'telemetry-{index:04d}.json').write_text(json.dumps(gpu,indent=2))
        if gpu.get('returncode')==0:
            free,temp=map(int,gpu['stdout'].splitlines()[0].split(','))
            if free<1024 or temp>=85: raise SystemExit('GPU headroom/temperature gate failed.')
        dest.parent.mkdir(parents=True,exist_ok=True)
        allowance=min(remaining,900)
        # Reserve before dispatch; an interrupted driver conservatively consumes the reservation.
        budget.write_text(json.dumps({'seconds':spent+allowance}))
        job_started=time.monotonic()
        with (args.output/f'job-{index:04d}.log').open('w',encoding='utf-8') as log:
            try: completed=subprocess.run(cmd,cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=allowance)
            except subprocess.TimeoutExpired: raise SystemExit('Job timeout; partial output and log retained. No automatic retry.')
        spent+=time.monotonic()-job_started
        budget.write_text(json.dumps({'seconds':spent}))
        if completed.returncode: raise SystemExit(f'Job {index} failed ({completed.returncode}); inspect log. Queue stopped.')
        if kind!='prepare':
            summary=dict(line.split('=',1) for line in (dest/'summary.txt').read_text().splitlines() if '=' in line)
            if summary.get('status')!='completed' or summary.get('recall_target_passed')!='true': raise SystemExit('Completion/correctness gate failed.')
            with (dest/'samples.csv').open() as stream:
                if any(row['fallback']!='false' for row in csv.DictReader(stream)): raise SystemExit('Unexpected fallback; queue stopped.')
        files=[dest] if kind=='prepare' else list(dest.glob('*'))
        marker.write_text(json.dumps({'command':cmd,'hashes':{str(f):sha(f) for f in files if f.is_file()}},indent=2))
        print(f'{index+1}/{len(jobs)} {kind} {dest.name}',flush=True)
    print('Queue completed; analyze block-level outcomes before any follow-on experiment.')

if __name__=='__main__': main()
