"""Generate E0/E2 partial-campaign table bodies from the retained derived CSVs.

Inputs are the independent reductions of runpod-e0-e2-partial-20260924.tar.gz:
research/data/processed/runpod-e0-e2-analysis-20260924/ (latency statistics) and
research/data/processed/runpod-e0-e2-mechanisms-20260924/ (diagnostics, witness).
Missing cells stay missing; nothing is interpolated, rounded to 1, or dropped.
"""
from pathlib import Path
import csv

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'paper/tables/final'
LAT = ROOT / 'research/data/processed/runpod-e0-e2-analysis-20260924'
MECH = ROOT / 'research/data/processed/runpod-e0-e2-mechanisms-20260924'
ENGINES = ['cpu', 'gpu-rows', 'gpu-predicate', 'gpu-mask']


def read(p):
    return list(csv.DictReader(p.open(newline='', encoding='utf-8')))


def write(name, rows):
    (OUT / (name + '.tex')).write_text('\n'.join(' & '.join(map(str, r)) + r' \\' for r in rows) + '\n',
                                       encoding='utf-8', newline='\r\n')


def ms(ns):
    return f'{float(ns) / 1e6:.3f}'


def ci(point, low, high, scale=1.0):
    return f'{float(point) / scale:.3f} [{float(low) / scale:.3f}, {float(high) / scale:.3f}]'


def frac(e):
    return f'{int(e) / 100000:g}'


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    cov = read(LAT / 'coverage.csv')
    cells = sorted({(r['experiment'], int(r['batch']), int(r['eligible'])) for r in cov})
    rows = []
    for exp, b, e in cells:
        by = {r['engine']: r for r in cov if (r['experiment'], int(r['batch']), int(r['eligible'])) == (exp, b, e)}
        counts = [f"{by[g]['completed']}/{by[g]['expected']}" if g in by else '--' for g in ENGINES]
        done = sum(int(by[g]['completed']) for g in by)
        want = sum(int(by[g]['expected']) for g in by)
        rows.append((exp.upper(), b, f'{e:,}', frac(e), *counts, f'{done}/{want}'))
    assert sum(int(r[-1].split('/')[0]) for r in rows) == 227 and sum(int(r[-1].split('/')[1]) for r in rows) == 260
    write('e0e2_coverage', rows)

    noise = read(LAT / 'e0_noise.csv')
    write('e0_noise', [(
        'CPU' if r['engine'] == 'cpu' else 'gpu-rows', r['metric'][:3].upper(), r['n_blocks'],
        ci(r['median_ns'], r['ci95_low_ns'], r['ci95_high_ns'], 1e6),
        f"{float(r['max_within_block_max_min_ratio']):.3f}",
        f"{float(r['pairwise_abs_log_ratio_p95']):.3f}",
        f"{r['adjacent_outside_25pct_count']}/{r['n_disjoint_adjacent_aa']}; {r['pairwise_outside_25pct_count']}/{r['n_all_pairwise_aa_dependent']}",
    ) for r in noise])

    lat = {(int(r['batch']), int(r['eligible']), r['engine'], r['metric']): r
           for r in read(LAT / 'latency_summary.csv') if r['experiment'] == 'e2'}
    for metric in ('p50_ns', 'p95_ns'):
        body = []
        for b in (1, 16):
            for e in (100, 1000, 3000, 10000, 30000, 100000):
                vals = []
                for g in ENGINES:
                    r = lat.get((b, e, g, metric))
                    if r is None:
                        vals.append('missing')
                    elif int(r['n_blocks']) == 1:
                        vals.append(f"{ms(r['median_ns'])} (n=1)")
                    else:
                        vals.append(ci(r['median_ns'], r['ci95_low_ns'], r['ci95_high_ns'], 1e6) + ('' if r['n_blocks'] == '5' else f" (n={r['n_blocks']})"))
                body.append((b, frac(e), *vals))
        write(f'e2_latency_{metric[:3]}', body)

    paired = {(int(r['batch']), int(r['eligible']), r['metric'], r['left'], r['right']): r
              for r in read(LAT / 'paired_comparisons.csv')}

    def pr(b, e, metric, left, right):
        r = paired.get((b, e, metric, left, right))
        if r is None:
            return 'missing'
        mark = r'$^\dagger$' if r['left_advantage_exceeds_e0_band'] == 'True' or r['right_advantage_exceeds_e0_band'] == 'True' else ''
        return ci(r['median_left_over_right'], r['ci95_low_ratio'], r['ci95_high_ratio']) + mark

    for name, right in (('e2_rows_vs_predicate_cpu', None), ('e2_rows_vs_mask', 'gpu-mask')):
        body = []
        for b in (1, 16):
            for e in (100, 1000, 3000, 10000, 30000, 100000):
                n = paired.get((b, e, 'p95_ns', 'gpu-rows', 'gpu-predicate'))
                n = n['n_paired_blocks'] if n else '0'
                if right:
                    body.append((b, frac(e), n, pr(b, e, 'p50_ns', 'gpu-rows', right), pr(b, e, 'p95_ns', 'gpu-rows', right),
                                 pr(b, e, 'p50_ns', 'gpu-predicate', right), pr(b, e, 'p95_ns', 'gpu-predicate', right)))
                else:
                    body.append((b, frac(e), n, pr(b, e, 'p50_ns', 'gpu-rows', 'gpu-predicate'), pr(b, e, 'p95_ns', 'gpu-rows', 'gpu-predicate'),
                                 pr(b, e, 'p50_ns', 'gpu-rows', 'cpu'), pr(b, e, 'p95_ns', 'gpu-rows', 'cpu')))
        write(name, body)

    diag = {(int(r['batch']), int(r['eligible']), r['engine']): r for r in read(MECH / 'diagnostics_by_cell.csv') if r['experiment'] == 'e2'}
    body = []
    for e in (100, 3000, 30000, 100000):
        for g in ENGINES[1:]:
            r = diag[(1, e, g)]
            body.append((f'{e:,}', g.removeprefix('gpu-'), ms(r['median_batch_latency_ns']), ms(r['median_row_materialization_ns']),
                         ms(r['median_device_selection_ns']), ms(r['median_device_scoring_ns']), f"{int(float(r['median_upload_bytes'])):,}"))
    write('e2_mechanisms', body)

    # E0 pairs exist only in the mechanism reduction; E2 pairs reuse the common paired bootstrap.
    body = []
    for r in read(MECH / 'edb_ordering_witness.csv'):
        b, e, q = int(r['batch']), int(r['eligible']), r['metric']
        ratio = f"{float(r['rows_over_cpu']):.3f} [{float(r['ci95_low']):.3f}, {float(r['ci95_high']):.3f}]"
        if r['experiment'] == 'e2':
            p = paired[(b, e, q + '_ns', 'gpu-rows', 'cpu')]
            assert p['n_paired_blocks'] == r['paired_blocks'] and abs(float(p['median_left_over_right']) - float(r['rows_over_cpu'])) < 1e-3
            ratio = ci(p['median_left_over_right'], p['ci95_low_ratio'], p['ci95_high_ratio'])
        body.append((r['experiment'].upper(), b, f'{e:,}', f"{int(r['edb']):,}", q.upper(), r['paired_blocks'], r['cpu_faster_blocks'], ratio))
    write('edb_witness', body)
    print('Generated E0/E2 table bodies')


if __name__ == '__main__':
    main()
