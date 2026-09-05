"""Regenerate reductions into paper/audit, preserving original evidence."""
from pathlib import Path
import importlib.util
import shutil
import csv
import json
ROOT=Path(__file__).resolve().parents[2]
OUT=ROOT/'paper/audit/reduced'
def load(name,path):
    spec=importlib.util.spec_from_file_location(name,path)
    m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m);return m
def main():
    OUT.mkdir(parents=True,exist_ok=True)
    historical=load('historical_reduce',ROOT/'research/scripts/analyze_results.py')
    historical.OUT=OUT/'historical';historical.OUT.mkdir(exist_ok=True)
    shutil.copy2(ROOT/'research/data/processed/android_device_lab.csv',historical.OUT/'android_device_lab.csv')
    historical.main()
    small=load('small_reduce',ROOT/'scripts/analyze_small_campaign.py')
    result=small.collect(ROOT/'research/artifacts/runpod-small-2026-09-05')
    dest=OUT/'small';dest.mkdir(exist_ok=True);small.write_report(result,dest)
    def rows(path):return list(csv.DictReader(path.open(encoding='utf-8',newline='')))
    retained=rows(ROOT/'research/artifacts/runpod-small-2026-09-05/report/performance-matrix.csv')
    regenerated=rows(dest/'performance-matrix.csv')
    assert retained==regenerated,'Campaign reduction differs from retained matrix'
    checks={'small_matrix_identical':True,'small_rows':len(result),'historical':{}}
    for p in historical.OUT.glob('*.csv'):
        orig=ROOT/'research/data/processed'/p.name
        checks['historical'][p.name]={'identical':orig.exists() and rows(p)==rows(orig)}
    (ROOT/'paper/audit/reduction-verification.json').write_text(json.dumps(checks,indent=2)+'\n')
    print(json.dumps(checks,indent=2))
if __name__=='__main__':main()
