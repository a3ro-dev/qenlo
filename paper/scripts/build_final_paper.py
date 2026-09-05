"""Build in a new isolated directory; reject unresolved references and boxes."""
from pathlib import Path
import subprocess, shutil, json, datetime, re, hashlib
ROOT=Path(__file__).resolve().parents[2]
PAPER=ROOT/'paper'
def main():
    build=PAPER/'tmp'/('clean-'+datetime.datetime.now().strftime('%Y%m%d-%H%M%S'))
    build.mkdir(parents=True,exist_ok=False)
    for name in ['paper.tex','appendix.tex','references.bib']:
        shutil.copy2(PAPER/name,build/name)
    shutil.copytree(PAPER/'figures/final',build/'figures/final')
    shutil.copytree(PAPER/'tables/final',build/'tables/final')
    commands=[['pdflatex','-interaction=nonstopmode','-halt-on-error','paper.tex'],['bibtex','paper']]+[['pdflatex','-interaction=nonstopmode','-halt-on-error','paper.tex']]*3
    records=[]
    for i,command in enumerate(commands):
        run=subprocess.run(command,cwd=build,text=True,encoding='utf-8',errors='replace',capture_output=True)
        (build/f'pass-{i}.txt').write_text(run.stdout+'\n'+run.stderr,encoding='utf-8')
        records.append({'command':command,'exit_code':run.returncode})
        if run.returncode:raise RuntimeError(f'Build failed: {build}/pass-{i}.txt')
    log=(build/'paper.log').read_text(encoding='utf-8',errors='replace')
    issues=[line for line in log.splitlines() if re.search(r'Overfull \\[hv]box|undefined|multiply defined|LaTeX Error|not found|ignored error|Infinite glue',line,re.I)]
    blg=(build/'paper.blg').read_text(errors='replace')
    issues += [l for l in blg.splitlines() if re.search(r'Warning--|error message',l,re.I) and not l.startswith('(There were 0')]
    output=PAPER/'output/pdf/qenlo-final-research-paper.pdf'
    report={'isolated_build':str(build),'commands':records,'issues':issues,'compile_passed':not issues,'visual_inspection':'pending','final_pdf':str(output)}
    (PAPER/'audit/build-verification.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2))
    if issues:raise RuntimeError('Final log checks failed; inspect report')
    output.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(build/'paper.pdf',output)
    report['pdf_sha256']=hashlib.sha256(output.read_bytes()).hexdigest()
    (PAPER/'audit/build-verification.json').write_text(json.dumps(report,indent=2)+'\n')
if __name__=='__main__':main()
