#!/usr/bin/env python3
"""Generate dependency-free SVG figures from processed benchmark results."""
import csv
import html
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "research/data/processed/a6000_exact_summary.csv"
FIG = ROOT / "paper/figures"


def bar_svg(title, labels, values, ylabel, note=""):
    w,h,left,top,bottom=760,430,85,55,90
    ymax=max(values)*1.15 if max(values)>0 else 1
    bw=(w-left-35)/len(values)*0.62
    gap=(w-left-35)/len(values)
    out=[f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}"><rect width="100%" height="100%" fill="white"/>',
         f'<text x="{w/2}" y="25" text-anchor="middle" font-family="sans-serif" font-size="17" font-weight="bold">{html.escape(title)}</text>',
         f'<line x1="{left}" y1="{h-bottom}" x2="{w-25}" y2="{h-bottom}" stroke="#222"/><line x1="{left}" y1="{top}" x2="{left}" y2="{h-bottom}" stroke="#222"/>']
    for i,(label,val) in enumerate(zip(labels,values)):
        x=left+gap*(i+.2); bh=(h-top-bottom)*val/ymax; y=h-bottom-bh
        color="#1b6ca8" if i%2==0 else "#d95f02"
        out += [f'<rect x="{x:.1f}" y="{y:.1f}" width="{bw:.1f}" height="{bh:.1f}" fill="{color}"/>',
                f'<text x="{x+bw/2:.1f}" y="{y-6:.1f}" text-anchor="middle" font-family="sans-serif" font-size="12">{val:.3g}</text>',
                f'<text x="{x+bw/2:.1f}" y="{h-bottom+18}" transform="rotate(28 {x+bw/2:.1f} {h-bottom+18})" text-anchor="start" font-family="sans-serif" font-size="11">{html.escape(label)}</text>']
    out += [f'<text x="18" y="{h/2}" transform="rotate(-90 18 {h/2})" text-anchor="middle" font-family="sans-serif" font-size="13">{html.escape(ylabel)}</text>',
            f'<text x="{w/2}" y="{h-8}" text-anchor="middle" font-family="sans-serif" font-size="11" fill="#555">{html.escape(note)}</text></svg>']
    return '\n'.join(out)+"\n"


def status_svg(title, rows):
    colors={"measured":"#3a923a","bounded":"#76a83b","partial":"#d69224","unmeasured":"#b83b3b"}
    out=['<svg xmlns="http://www.w3.org/2000/svg" width="760" height="430" viewBox="0 0 760 430"><rect width="100%" height="100%" fill="white"/>',
         f'<text x="380" y="30" text-anchor="middle" font-family="sans-serif" font-size="17" font-weight="bold">{html.escape(title)}</text>']
    for i,(name,status) in enumerate(rows):
        y=65+i*42; key=status if status in colors else "partial"
        out += [f'<text x="40" y="{y+20}" font-family="sans-serif" font-size="13">{html.escape(name)}</text>',
                f'<rect x="430" y="{y}" width="270" height="28" rx="5" fill="{colors[key]}"/>',
                f'<text x="565" y="{y+19}" text-anchor="middle" font-family="sans-serif" font-size="12" fill="white">{html.escape(status)}</text>']
    out.append('</svg>'); return '\n'.join(out)+"\n"


def native_crossover_svg():
    rows = list(csv.DictReader((ROOT / "research/data/processed/native_crossover_summary.csv").open(newline="")))
    w,h,left,top,right,bottom=760,430,80,45,25,70
    xs=[int(r["eligible"]) for r in rows]
    ymax=max(max(float(r["cpu_p95_ms"]),float(r["gpu_rows_p95_ms"])) for r in rows)*1.12
    def xc(x): return left+(x-min(xs))/(max(xs)-min(xs))*(w-left-right)
    def yc(y): return top+(1-y/ymax)*(h-top-bottom)
    series=[("cpu_p95_ms","CPU exact","#d95f02"),("gpu_rows_p95_ms","WGPU compact rows","#1b6ca8")]
    out=[f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}"><rect width="100%" height="100%" fill="white"/>',
         '<text x="380" y="25" text-anchor="middle" font-family="sans-serif" font-size="17" font-weight="bold">Native CPU/WGPU crossover</text>',
         f'<line x1="{left}" y1="{h-bottom}" x2="{w-right}" y2="{h-bottom}" stroke="#222"/><line x1="{left}" y1="{top}" x2="{left}" y2="{h-bottom}" stroke="#222"/>']
    for i in range(5):
        val=ymax*i/4; y=yc(val)
        out += [f'<line x1="{left}" y1="{y:.1f}" x2="{w-right}" y2="{y:.1f}" stroke="#ddd"/>',f'<text x="{left-9}" y="{y+4:.1f}" text-anchor="end" font-family="sans-serif" font-size="12">{val:.1f}</text>']
    for x in xs:
        out.append(f'<text x="{xc(x):.1f}" y="{h-bottom+22}" text-anchor="middle" font-family="sans-serif" font-size="12">{x//1000}k</text>')
    for key,label,color in series:
        pts=[(xc(int(r["eligible"])),yc(float(r[key]))) for r in rows]
        path=' '.join(("M" if i==0 else "L")+f" {x:.1f} {y:.1f}" for i,(x,y) in enumerate(pts))
        out.append(f'<path d="{path}" fill="none" stroke="{color}" stroke-width="3"/>')
        out += [f'<circle cx="{x:.1f}" cy="{y:.1f}" r="4" fill="{color}"/>' for x,y in pts]
    out += [f'<text x="{w/2}" y="{h-12}" text-anchor="middle" font-family="sans-serif" font-size="14">eligible vectors E</text>',
            f'<text x="18" y="{h/2}" transform="rotate(-90 18 {h/2})" text-anchor="middle" font-family="sans-serif" font-size="14">P95 completed-call latency (ms)</text>',
            '<text x="430" y="62" font-family="sans-serif" font-size="12" fill="#555">winner reverses between E=2k and E=3k</text>']
    for i,(_,label,color) in enumerate(series):
        y=82+i*20; out += [f'<line x1="500" y1="{y}" x2="530" y2="{y}" stroke="{color}" stroke-width="3"/>',f'<text x="538" y="{y+4}" font-family="sans-serif" font-size="12">{label}</text>']
    out.append('</svg>')
    return '\n'.join(out)+"\n"


def main():
    rows = list(csv.DictReader(DATA.open(newline="")))
    FIG.mkdir(parents=True, exist_ok=True)
    w, h, left, top, right, bottom = 760, 430, 80, 35, 25, 65
    xs = sorted({int(r["eligible"]) for r in rows})
    ymax = max(float(r["p95_ms"]) for r in rows) * 1.08
    xlog = [math.log10(x) for x in xs]
    def xcoord(x): return left + (math.log10(x) - min(xlog)) / (max(xlog) - min(xlog)) * (w-left-right)
    def ycoord(y): return top + (1-y/ymax) * (h-top-bottom)
    colors = {"faiss_gpu_flat_ip":"#d95f02", "qenlo_cuda_buffered_prototype":"#1b6ca8"}
    labels = {"faiss_gpu_flat_ip":"FAISS GPU Flat", "qenlo_cuda_buffered_prototype":"Qenlo CUDA prototype"}
    lines = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">',
             '<rect width="100%" height="100%" fill="white"/>',
             f'<line x1="{left}" y1="{h-bottom}" x2="{w-right}" y2="{h-bottom}" stroke="#222"/>',
             f'<line x1="{left}" y1="{top}" x2="{left}" y2="{h-bottom}" stroke="#222"/>']
    for i in range(5):
        val = ymax*i/4; y=ycoord(val)
        lines += [f'<line x1="{left}" y1="{y:.1f}" x2="{w-right}" y2="{y:.1f}" stroke="#ddd"/>',
                  f'<text x="{left-9}" y="{y+4:.1f}" text-anchor="end" font-family="sans-serif" font-size="12">{val:.1f}</text>']
    for x in xs:
        xc=xcoord(x); lines.append(f'<text x="{xc:.1f}" y="{h-bottom+22}" text-anchor="middle" font-family="sans-serif" font-size="12">{x:,}</text>')
    for system in colors:
        pts=[]
        for x in xs:
            r=next(r for r in rows if r["system"]==system and int(r["eligible"])==x)
            pts.append((xcoord(x),ycoord(float(r["p95_ms"]))))
        path=' '.join(("M" if i==0 else "L")+f" {x:.1f} {y:.1f}" for i,(x,y) in enumerate(pts))
        lines.append(f'<path d="{path}" fill="none" stroke="{colors[system]}" stroke-width="3"/>')
        lines += [f'<circle cx="{x:.1f}" cy="{y:.1f}" r="4" fill="{colors[system]}"/>' for x,y in pts]
    lines += [f'<text x="{w/2}" y="{h-12}" text-anchor="middle" font-family="sans-serif" font-size="14">eligible vectors (log scale)</text>',
              f'<text x="18" y="{h/2}" transform="rotate(-90 18 {h/2})" text-anchor="middle" font-family="sans-serif" font-size="14">P95 end-to-end latency (ms)</text>']
    for i,system in enumerate(colors):
        y=20+i*20; lines += [f'<line x1="470" y1="{y}" x2="500" y2="{y}" stroke="{colors[system]}" stroke-width="3"/>',f'<text x="508" y="{y+4}" font-family="sans-serif" font-size="12">{html.escape(labels[system])}</text>']
    lines.append('</svg>')
    (FIG / "a6000_exact_crossover.svg").write_text('\n'.join(lines)+"\n")
    (FIG / "architecture.svg").write_text('''<svg xmlns="http://www.w3.org/2000/svg" width="760" height="300" viewBox="0 0 760 300"><rect width="760" height="300" fill="white"/><style>text{font-family:sans-serif}.b{fill:#eef4fa;stroke:#1b6ca8;stroke-width:2}.d{fill:#fff4e8;stroke:#d95f02;stroke-width:2}.a{stroke:#333;stroke-width:2;marker-end:url(#m)}</style><defs><marker id="m" markerWidth="8" markerHeight="8" refX="7" refY="3" orient="auto"><path d="M0,0 L0,6 L8,3 z" fill="#333"/></marker></defs><rect class="b" x="30" y="95" width="190" height="105" rx="8"/><text x="125" y="125" text-anchor="middle" font-size="17">Canonical CoreStore</text><text x="125" y="151" text-anchor="middle" font-size="13">vectors / metadata / tombstones</text><text x="125" y="174" text-anchor="middle" font-size="13">snapshot + WAL generation</text><rect class="d" x="300" y="25" width="190" height="60" rx="8"/><text x="395" y="60" text-anchor="middle">CPU exact (AVX2/scalar)</text><rect class="d" x="300" y="120" width="190" height="60" rx="8"/><text x="395" y="155" text-anchor="middle">USearch / HNSW</text><rect class="d" x="300" y="215" width="190" height="60" rx="8"/><text x="395" y="241" text-anchor="middle">WGPU exact / IVF</text><text x="395" y="260" text-anchor="middle" font-size="12">mask / rows / predicate</text><rect class="b" x="560" y="95" width="165" height="105" rx="8"/><text x="642" y="132" text-anchor="middle">observable router</text><text x="642" y="156" text-anchor="middle" font-size="12">actual backend</text><text x="642" y="175" text-anchor="middle" font-size="12">fallback + transfers</text><line class="a" x1="220" y1="125" x2="300" y2="60"/><line class="a" x1="220" y1="150" x2="300" y2="150"/><line class="a" x1="220" y1="175" x2="300" y2="240"/><line class="a" x1="490" y1="55" x2="560" y2="120"/><line class="a" x1="490" y1="150" x2="560" y2="150"/><line class="a" x1="490" y1="245" x2="560" y2="180"/><text x="380" y="294" text-anchor="middle" font-size="12">Canonical truth determines existence; search structures are rebuildable derived state.</text></svg>''')
    (FIG / "selectivity_p95.svg").write_text(bar_svg("Native exact latency versus selectivity", ["CPU 1%","GPU rows 1%","GPU predicate 1%","CPU 100%","GPU predicate 100%"], [.2304,.6766,2.8832,16.6567,3.2404], "P95 latency (ms)", "100k x 384 RTX 4050; batch 1; real embeddings"))
    (FIG / "native_cpu_gpu_crossover.svg").write_text(native_crossover_svg())
    (FIG / "absolute_eligible_p95.svg").write_text((FIG / "a6000_exact_crossover.svg").read_text())
    (FIG / "dimension_crossover.svg").write_text(status_svg("Dimensionality crossover evidence", [("384-dimensional native cohort","bounded"),("768-dimensional CUDA/FAISS cohort","bounded"),("matched host and E across dimensions","unmeasured")]))
    (FIG / "batch_effect.svg").write_text(bar_svg("Batch amortization on Intel Arc", ["GPU batch 1","GPU batch 8","auto CPU: E=1k, B=1","auto GPU: E=1k, B=8"], [4.444,1.245,.273,.555], "reported P95 (ms)", "100k x 384 soak; 512 samples; do not compare bars with different E as speedups"))
    (FIG / "android_soak_p95.svg").write_text(bar_svg("Android device-lab soak", ["CPU exact","WGPU exact","WGPU B=8","IVF-Flat","IVF-SQ8","auto CPU 1%","auto GPU 1% B=8"], [27.019,17.678,11.131,21.453,37.090,.544,9.890], "reported P95 (ms)", "MT6897 / Mali-G615 MC6 / Vulkan; 100k x 384; 512 samples; batch statistics are not normalized"))
    (FIG / "hnsw_tradeoff.svg").write_text(bar_svg("Controlled exact versus HNSW", ["USearch ef=128","WGPU exact"], [3.6245,3.2404], "P95 latency (ms)", "recall@10: 0.99224 vs 0.99998; speedup interval crosses parity"))
    (FIG / "fastest_backend_heatmap.svg").write_text(status_svg("Observed fastest backend by region", [("E=1k, D=384, B=1","measured: CPU"),("E=100k, D=384, B=1","measured: WGPU"),("E<=10k, D=768, B=1","measured: FAISS"),("E>=100k, D=768, B=1","measured: CUDA prototype")]))
    (FIG / "routing_regret.svg").write_text(status_svg("Automatic routing evidence", [("E=1k, B=1 selected CPU","measured"),("E=1k, B=8 selected GPU","measured"),("matched fixed-policy counterfactuals","unmeasured"),("routing regret","unmeasured")]))
    (FIG / "matched_corpus_size.svg").write_text(status_svg("Matched absolute eligible-count study", [("100k corpus at E=1k","measured"),("1M corpus at E about 10k","measured"),("same E and host across N","unmeasured")]))
    (FIG / "metadata_robustness.svg").write_text(status_svg("Metadata-distribution robustness", [("independent distribution","measured"),("positive correlation","unmeasured"),("negative correlation","unmeasured"),("skewed distribution","test coverage only")]))


if __name__ == "__main__": main()
