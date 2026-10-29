#!/usr/bin/env python3
"""Recompute every checkable number in "Power-State Variance in One WGPU
Vector-Search Kernel" (Kushwaha, PDF of 2026-10-25) from the manuscript's OWN
tables. No raw data are used: every input below is transcribed from the PDF
(table/section noted). Run:  python3 -I manuscript_recompute.py
"""
import itertools, math
import numpy as np
from scipy import stats

def hdr(s): print("\n" + "=" * 78 + "\n" + s + "\n" + "=" * 78)
def chk(label, computed, reported, tol):
    ok = abs(computed - reported) <= tol
    print(f"  {'OK ' if ok else 'MISMATCH'} {label}: computed={computed:.4f} reported={reported} (tol {tol})")
    return ok

# ---------------------------------------------------------------- Table 1
# (E, GPU, n, heavy_med, heavy_min, heavy_max, heavy_mm, light_med, light_min, light_max, light_mm)
T1 = [
 (1000,"4050 laptop",5,0.950,0.949,0.954,1.01,0.030,0.030,0.030,1.00),
 (1000,"4090 host A",5,0.711,0.711,0.711,1.00,0.053,0.051,0.065,1.27),
 (1000,"4090 pods B",5,0.681,0.650,0.695,1.07,0.028,0.027,0.056,2.08),
 (1000,"H100",3,0.987,0.982,0.987,1.00,0.039,0.039,0.039,1.02),
 (3000,"4050 laptop",5,0.952,0.951,0.956,1.01,0.106,0.055,0.230,4.16),
 (3000,"4090 host A",5,0.711,0.711,0.711,1.00,0.210,0.098,0.251,2.56),
 (3000,"4090 pods B",5,0.685,0.652,0.700,1.07,0.055,0.050,0.216,4.30),
 (3000,"H100",3,0.988,0.988,0.988,1.00,0.094,0.088,0.104,1.18),
 (10000,"4050 laptop",5,0.941,0.940,0.941,1.00,0.461,0.425,0.595,1.40),
 (10000,"4090 host A",5,0.703,0.703,0.703,1.00,0.440,0.337,0.512,1.52),
 (10000,"4090 pods B",5,0.673,0.643,0.687,1.07,0.142,0.128,0.589,4.61),
 (10000,"H100",3,0.977,0.972,0.977,1.01,0.205,0.188,0.206,1.10),
]
hdr("TABLE 1: max/min column vs (min-max) range, with rounding bounds")
for (E,g,n,hm,hlo,hhi,hmm,lm,llo,lhi,lmm) in T1:
    for path,lo,hi,mm in (("heavy",hlo,hhi,hmm),("light",llo,lhi,lmm)):
        r = hi/lo
        # displayed values are rounded to 3 decimals: ratio bounds
        rmin = (hi-0.0005)/(lo+0.0005); rmax = (hi+0.0005)/(lo-0.0005)
        flag = "OK " if rmin-0.005 <= mm <= rmax+0.005 else "MISMATCH"
        print(f"  {flag} E={E:>5} {g:12} {path:5}: {hi:.3f}/{lo:.3f} = {r:.3f}; feasible [{rmin:.3f},{rmax:.3f}]; reported {mm}")

hdr("Abstract/intro/caption spread claims: which E and GPU they come from")
print("  laptop light max/min by E:", {E: lmm for (E,g,*_ ,lmm) in [(r[0],r[1],r[10]) for r in T1] if g=="4050 laptop"} if False else
      [(r[0], r[10]) for r in T1 if r[1]=="4050 laptop"])
print("  -> abstract/contribution '4.2x on the laptop' = E=3,000 (4.16); at E=10,000 laptop spread is 1.40x")
print("  pods light max/min by E:", [(r[0], r[10]) for r in T1 if r[1]=="4090 pods B"])
print("  -> Sec.4 '4.3-4.6x across the five 4090 pods' omits E=1,000 (2.08x)")
print("  host A light max/min by E:", [(r[0], r[10]) for r in T1 if r[1]=="4090 host A"])
print("  -> Fig.1 caption 'up to 1.5x on a machine' = host A at E=10,000 (1.52); intro '1.4x on the laptop' = E=10,000")
hm = [r[5]/r[4] for r in T1 if r[1] in ("4050 laptop","4090 host A","H100")]
hp = [r[5]/r[4] for r in T1 if r[1]=="4090 pods B"]
print(f"  heavy within-machine max/min max = {max(hm):.4f} (claim <=1%); across pods max = {max(hp):.4f} (claim 7%)")
print(f"  4090 host A + pods light at E=10k: 0.589/0.128 = {0.589/0.128:.3f} (Sec.8 '4.6x span')")
print("  light faster than heavy in every Table 1 cell:", all(r[7] < r[3] for r in T1))

hdr("Sec. 8: device-time advantage heavy/light at E=10,000 (Table 1) and locked (Table 4)")
for r in T1:
    if r[0]==10000: print(f"  {r[1]:12}: {r[3]:.3f}/{r[7]:.3f} = {r[3]/r[7]:.2f}x")
print("  reported: 1.6 (host A), 2.0 (laptop), 4.7 (pods), 4.8 (H100)")
chk("locked-boost heavy/light E=10k (Table 4: 0.941/0.113)", 0.941/0.113, 8.3, 0.05)

# ---------------------------------------------------------------- Table 4
hdr("TABLE 4 (E-LOCK): U, Lhi, Llo light; heavy Lhi")
T4 = {1000:(0.069,2.84,0.024,1.00,0.096,1.00,0.948), 3000:(0.176,2.11,0.045,1.00,0.177,1.00,0.951),
      10000:(0.567,1.51,0.113,1.00,0.444,1.00,0.941)}
uL = [T4[E][0]/T4[E][2] for E in T4]
print("  U/Lhi light:", [f"{x:.2f}" for x in uL], " (Sec.8 claims 2.9-5.0x)")
print("  Llo/Lhi light:", [f"{T4[E][4]/T4[E][2]:.2f}" for E in T4], " SM ratio 2670/585 =", f"{2670/585:.2f}")
print("  Llo/U light:", [f"{T4[E][4]/T4[E][0]:.2f}" for E in T4], " (<1 => unlocked median SLOWER than the 'low-state' lock)")
d1_light = {1000:0.030,3000:0.106,10000:0.461}
print("  E-LOCK U vs D1 unlocked laptop (Table 1) light medians:",
      [(E, T4[E][0], d1_light[E], f"{T4[E][0]/d1_light[E]:.2f}x") for E in T4])
print("  E-LOCK U max/min vs D1 laptop max/min: E=1k 2.84 vs 1.00; E=3k 2.11 vs 4.16; E=10k 1.51 vs 1.40")
print("  Lhi vs D1 unlocked median (alt. 'understatement'):", [f"{d1_light[E]/T4[E][2]:.2f}x" for E in T4])
chk("K3: Llo(E=10k)/D1 light (0.444/0.461) deviation", abs(0.444/0.461-1), 0.04, 0.005)
print(f"  K3 against SAME-experiment U arm instead: 0.444/0.567 = {0.444/0.567:.3f} -> {100*(1-0.444/0.567):.1f}% off (K3 tolerance 25%)")
chk("heavy slowdown low lock (Table 5 LL/HH heavy E=1k: 3.615/0.947)", 3.615/0.947, 3.8, 0.05)
print(f"  Sec.7 '0.95 to 3.6 ms' = {3.6/0.95:.2f}x ; heavy Llo is NOT in Table 4 (it appears only in Table 5)")
# K2 reconstruction: best unlocked median is presumably U+heater, given heater ratios 0.57/0.37/0.40 (Sec.7)
hr = {1000:0.57,3000:0.37,10000:0.40}
print("  K2 reconstruction (assume best unlocked median = U x heater ratio, ratio-of-medians approximation):")
for E in T4:
    uh = T4[E][0]*hr[E]
    print(f"    E={E}: U+heater ~ {uh:.4f} ms; Lhi/that = {T4[E][2]/uh:.3f}; 1 - that = {1-T4[E][2]/uh:.3f}")
print("  reported: Lhi = 0.50-0.68 of best unlocked median <=> 32-50% less time (1-0.68=0.32, 1-0.50=0.50): mutually consistent")
print("  with heater-off U as reference Lhi/U would be", [f"{T4[E][2]/T4[E][0]:.3f}" for E in T4], "-> so 'best unlocked' must be heater-on")
print(f"  U+heater vs Lhi as 'understatement' factor: {[f'{T4[E][0]*hr[E]/T4[E][2]:.2f}x' for E in T4]} (vs 2.9-5.0x quoted)")

# ---------------------------------------------------------------- Table 5
hdr("TABLE 5 (E-SPLIT): memory share m = ln(tHL/tHH)/ln(tLL/tHH); SM share s = ln(tLH/tHH)/ln(tLL/tHH)")
T5 = [ # path,E, sel HH HL LH LL m, score HH HL LH LL m
 ("Light",1000,0.024,0.024,0.096,0.096,0.00,0.007,0.007,0.023,0.023,0.00),
 ("Light",3000,0.044,0.045,0.177,0.177,0.00,0.012,0.012,0.044,0.044,0.00),
 ("Light",10000,0.113,0.124,0.434,0.444,0.07,0.034,0.069,0.123,0.149,0.48),
 ("Heavy",1000,0.947,0.947,3.615,3.615,0.00,0.126,0.126,0.541,0.541,0.00),
 ("Heavy",3000,0.951,0.951,3.624,3.624,0.00,0.140,0.140,0.593,0.593,0.00),
 ("Heavy",10000,0.940,0.965,3.567,3.593,0.02,0.196,0.282,0.763,0.876,0.24)]
def share(hh,x,ll): return math.log(x/hh)/math.log(ll/hh)
def share_bounds(hh,x,ll):
    vals=[]
    for a in (hh-0.0005,hh+0.0005):
        for b in (x-0.0005,x+0.0005):
            for c in (ll-0.0005,ll+0.0005):
                if b>0 and a>0 and c>a: vals.append(share(a,b,c))
    return min(vals),max(vals)
for (p,E,sHH,sHL,sLH,sLL,sm,cHH,cHL,cLH,cLL,cm) in T5:
    m1=share(sHH,sHL,sLL); s1=share(sHH,sLH,sLL); m2=share(cHH,cHL,cLL); s2=share(cHH,cLH,cLL)
    b1=share_bounds(sHH,sHL,sLL); b2=share_bounds(cHH,cHL,cLL)
    print(f"  {p} E={E:>5} sel: m={m1:.3f} (rep {sm}) rounding-range [{b1[0]:.3f},{b1[1]:.3f}] s={s1:.3f} m+s={m1+s1:.3f} | "
          f"score: m={m2:.3f} (rep {cm}) range [{b2[0]:.3f},{b2[1]:.3f}] s={s2:.3f} m+s={m2+s2:.3f}")
chk("light scoring 0.069/0.034 ('doubled')", 0.069/0.034, 2.0, 0.05)
chk("heavy scoring 0.282/0.196 ('+44%')", 0.282/0.196, 1.44, 0.005)
chk("light selection HL/HH E=10k 0.124/0.113 ('+10%')", 0.124/0.113, 1.10, 0.005)
print("  heavy scoring HH grows with E: 0.126 -> 0.140 -> 0.196 =", f"{0.196/0.126:.2f}x", "(heavy path said to do work 'proportional to N regardless of E')")
print("  E-SPLIT corners vs E-LOCK (claimed within 0.2%):")
for E,hh,ll,hv in ((1000,0.024,0.096,0.947),(3000,0.044,0.177,0.951),(10000,0.113,0.444,0.940)):
    print(f"    E={E}: light HH {hh} vs Lhi {T4[E][2]} ({100*(hh/T4[E][2]-1):+.1f}% at display precision); "
          f"light LL {ll} vs Llo {T4[E][4]} ({100*(ll/T4[E][4]-1):+.1f}%); heavy HH {hv} vs Lhi {T4[E][6]} ({100*(hv/T4[E][6]-1):+.2f}%)")

# ---------------------------------------------------------------- Table 3
hdr("TABLE 3: achievable fractions; descriptive tests (NOT independent; pairing ignored)")
for lab,k,n,rep in (("laptop off",13,25,52),("pods off",7,25,28),("pods on",4,25,16),("H100 off",1,9,11)):
    print(f"  {lab}: {k}/{n} = {100*k/n:.1f}% (reported {rep}%)")
for lab,(a,n1),(b,n2) in (("laptop",(13,25),(0,25)),("pods",(7,25),(4,25)),("H100",(1,9),(0,9))):
    p = stats.fisher_exact([[a,n1-a],[b,n2-b]])[1]
    print(f"  Fisher exact (descriptive only) {lab} {a}/{n1} vs {b}/{n2}: p={p:.4g}")
print("  Note: Fig.1c pods heater-off hollow squares reach ~4.8x; Sec.6 'tail reaching 3.3x' is the p90 (3.29), not the max")

# ---------------------------------------------------------------- Table 7 / Table 6
hdr("TABLE 7 vs TABLE 6 vs Sec.6/Scorecard")
T7 = [ # gpu,E,n, Ldev, Hdev, Lcall, Hcall (point, lo, hi)
 ("laptop",100,5,(0.91,0.90,1.12),(1.19,1.00,1.20),(1.11,0.78,1.80),(1.09,1.02,1.12)),
 ("laptop",1000,5,(1.00,0.98,1.01),(1.18,1.18,1.19),(1.13,1.05,1.64),(1.06,1.04,1.09)),
 ("laptop",3000,5,(0.65,0.26,1.25),(1.19,1.18,1.19),(0.96,0.48,1.56),(1.07,1.02,1.09)),
 ("laptop",10000,5,(0.46,0.39,0.54),(1.19,1.19,1.20),(0.97,0.79,1.10),(1.08,1.06,1.17)),
 ("laptop",30000,5,(0.59,0.57,0.68),(1.00,1.00,1.00),(0.95,0.92,1.01),(1.07,1.02,1.11)),
 ("pods",100,5,(0.92,0.91,1.01),(1.00,1.00,1.00),(0.97,0.95,1.11),(1.07,0.95,1.08)),
 ("pods",1000,5,(0.90,0.70,0.92),(1.00,1.00,1.00),(0.97,0.94,1.03),(1.07,1.06,1.28)),
 ("pods",3000,5,(0.87,0.35,0.94),(1.00,1.00,1.00),(0.96,0.54,0.99),(1.07,1.02,1.07)),
 ("pods",10000,5,(0.82,0.34,0.91),(1.00,1.00,1.01),(0.94,0.64,1.00),(1.07,1.07,1.37)),
 ("pods",30000,5,(1.00,0.46,1.00),(1.00,1.00,1.00),(1.07,0.57,1.09),(1.07,1.03,1.16)),
 ("H100",1000,3,(0.86,0.86,0.88),(1.36,1.35,1.36),(1.24,1.21,1.25),(1.26,1.26,1.28)),
 ("H100",3000,3,(0.71,0.65,0.77),(1.35,1.35,1.35),(1.10,1.03,1.17),(1.27,1.26,1.28)),
 ("H100",10000,3,(0.81,0.80,0.88),(1.36,1.36,1.37),(1.18,1.14,1.48),(1.25,1.19,1.26))]
T6 = { # (P1, P2, P3 call, P3 dev, P4)
 ("laptop",100):("no","yes","no","yes","no"),("laptop",1000):("no","yes","no","yes","no"),
 ("laptop",3000):("no","yes","yes","yes","no"),("laptop",10000):("n/a","yes","yes","yes","no"),
 ("laptop",30000):("n/a","yes","yes","yes","yes"),("pods",100):("no","yes","yes","yes","no"),
 ("pods",1000):("no","yes","yes","yes","yes"),("pods",3000):("no","yes","yes","yes","yes"),
 ("pods",10000):("n/a","yes","yes","yes","yes"),("pods",30000):("n/a","yes","no","yes","yes"),
 ("H100",1000):("no","yes","yes","yes","no"),("H100",3000):("no","yes","yes","yes","no"),
 ("H100",10000):("n/a","yes","yes","yes","yes")}
p3c = p3d = p4 = 0
for (g,E,n,Ld,Hd,Lc,Hc) in T7:
    t6 = T6[(g,E)]
    p1 = "n/a" if E>3000 else ("yes" if (Lc[0]<0.9 and Lc[2]<1) else "no")
    p2 = "yes" if (Hd[0]>=0.95 and Hc[0]>=0.95) else "no"
    c = "yes" if Lc[0]<Hc[0] else ("TIE" if Lc[0]==Hc[0] else "no")
    d = "yes" if Ld[0]<Hd[0] else ("TIE" if Ld[0]==Hd[0] else "no")
    p3c += t6[2]=="yes"; p3d += t6[3]=="yes"; p4 += t6[4]=="yes"
    notes=[]
    if p1!=t6[0]: notes.append(f"P1 recomputed {p1} vs T6 {t6[0]}")
    if c!=t6[2]: notes.append(f"P3-call recomputed {c} vs T6 {t6[2]}")
    if d!=t6[3]: notes.append(f"P3-dev recomputed {d} vs T6 {t6[3]}")
    if t6[4]=="yes" and not Ld[0]<1: notes.append(f"P4=yes but light device ratio {Ld[0]} (no fall at shown precision)")
    if t6[4]=="no" and Ld[0]<0.95 and g=="pods": notes.append(f"P4=no although light device {Ld[0]} (host criterion must have failed)")
    print(f"  {g:6} E={E:>5}: Lcall {Lc[0]} vs Hcall {Hc[0]} -> {c}; Ldev {Ld[0]} vs Hdev {Hd[0]} -> {d}; " + ("; ".join(notes) if notes else "consistent"))
print(f"  Table 6 counts: P3 call yes={p3c}/13 (claim 10), P3 device yes={p3d}/13 (claim 13), P4 yes={p4}/13 (claim 6)")
print("  Sec.6: light call with heater 0.94-0.96 on pods at E=3k-10k (T7: 0.96, 0.94); laptop E>=3k 0.95-0.97 (T7: 0.96,0.97,0.95): OK")
print("  Sec.6: heavy calls 6-9% slower consumer (T7 1.06-1.09 laptop, 1.07 pods), 25-27% H100 (1.25-1.27): OK")
print("  Sec.6: heavy device 'uniformly slower on the laptop (1.19x)' -- but laptop E=30,000 heavy device = 1.00 [1.00,1.00]")
print("  Sec.7 heater effect unlocked (E-LOCK) 0.57/0.37/0.40 vs D1 Table 7 laptop 1.00/0.65/0.46 at E=1k/3k/10k")

hdr("TABLE 8: winners, ratio-of-medians vs Table 7 median-of-ratios, CPU per-row cost")
T8 = [("laptop",100,0.010,0.172,0.253,1.999),("laptop",1000,0.083,0.230,0.374,2.050),("laptop",3000,0.308,0.548,0.559,2.109),
      ("laptop",10000,3.332,1.682,1.685,2.204),("laptop",30000,9.187,2.265,2.138,2.791),
      ("pods",100,0.008,0.103,0.100,0.828),("pods",1000,0.048,0.123,0.123,0.835),("pods",3000,0.133,0.178,0.175,0.849),
      ("pods",10000,1.495,0.430,0.405,0.860),("pods",30000,4.907,0.779,0.833,0.894)]
t7c = {(g,E):Lc[0] for (g,E,n,Ld,Hd,Lc,Hc) in T7}
for (g,E,cpu,off,on,hv) in T8:
    w_off = "GPU" if off<cpu else "CPU"; w_on = "GPU" if on<cpu else "CPU"
    print(f"  {g:6} E={E:>5}: CPU {cpu} light-off {off} ({w_off}) light-on {on} ({w_on}) heavy {hv}; on/off={on/off:.3f} vs T7 light call {t7c[(g,E)]}; CPU us/row={1000*cpu/E:.3f}")
for g in ("laptop","pods"):
    rows={E:cpu for (gg,E,cpu,*_) in T8 if gg==g}
    fp32_rate = rows[3000]/3000
    est = fp32_rate*10000
    light10 = [off for (gg,E,cpu,off,*_) in T8 if gg==g and E==10000][0]
    print(f"  {g}: CPU per-row cost jumps {(rows[10000]/10000)/(rows[3000]/3000):.2f}x between E=3k (FP32+FP64 rescoring) and E=10k (FP64 throughout);"
          f" FP32-rate extrapolation to E=10k = {est:.3f} ms vs light GPU {light10} ms")
print("  H100 and RTX 4090 host A are absent from Table 8 -> abstract's 'at E=10,000 light faster than CPU' unverifiable for H100")
print("  Busy-fraction at LOCKED BOOST (E-LOCK Lhi sel + E-SPLIT HH scoring) / D1 laptop light call (Table 8):")
for E,sel,sc,call in ((1000,0.024,0.007,0.230),(3000,0.045,0.012,0.548),(10000,0.113,0.034,1.682)):
    print(f"    E={E}: ({sel}+{sc})/{call} = {100*(sel+sc)/call:.1f}%  (unlocked D1 top-k only: {100*d1_light[E]/call:.1f}%)")
print(f"  Laptop E=10k: heater cut light device 0.461 by 54% (~{0.461*0.54:.3f} ms) yet light call 1.682 -> 1.685 ms")

# ---------------------------------------------------------------- Appendix C, Spearman
hdr("Appendix C and Section 4 statistics")
chk("235.7% regret -> ratio", 1+2.357, 3.36, 0.005)
print("  Median regret over 16 workloads with 8 at zero = mean of 8th/9th order stats = (0 + r_min_wrong)/2 if 'median' is the usual even-n definition")
print("  => a 0.4% median implies smallest non-zero regret ~0.8% under that definition (cannot check: per-workload regrets not in PDF)")
r = -0.83; n = 10
t = r*math.sqrt((n-2)/(1-r*r)); p_t = 2*stats.t.sf(abs(t), n-2)
print(f"  Spearman rho=-0.83, n=10: t-approx t={t:.3f}, two-sided p={p_t:.4f}")
# exact permutation distribution of sum d^2
cnt = 0; tot = 0; target = (1-r)*n*(n*n-1)/6  # sum d^2 at rho=-0.83
base = np.arange(n)
perms = np.array(list(itertools.permutations(range(n))), dtype=np.int8)
d2 = ((perms - base)**2).sum(axis=1)
rho_all = 1 - 6*d2/(n*(n*n-1))
p_exact_two = np.mean(np.abs(rho_all) >= 0.83 - 1e-12)
p_exact_one = np.mean(rho_all <= -0.83 + 1e-12)
print(f"  exact permutation (no ties): P(|rho|>=0.83)={p_exact_two:.5f}, one-sided P(rho<=-0.83)={p_exact_one:.5f}")
print(f"  rho achievable values near -0.83: sum d^2 = {target:.1f} -> rho(302)={1-6*302/990:.4f}, rho(300)={1-6*300/990:.4f}")

# ---------------------------------------------------------------- bootstrap mechanics
hdr("Percentile bootstrap of the median with n = 3 or 5 blocks")
def exact_boot_median_tail(n):
    # P(bootstrap median == x_(1)) when n odd: need >= (n+1)/2 draws equal x_(1)
    k0 = (n+1)//2
    return sum(math.comb(n,k)*(1/n)**k*(1-1/n)**(n-k) for k in range(k0,n+1))
for n in (3,5):
    pt = exact_boot_median_tail(n)
    cov = 1 - 2*0.5**n
    distinct = math.comb(2*n-1, n)
    print(f"  n={n}: P(boot median = min) = {pt:.4f} (>0.025 => 2.5th pct = sample min, 97.5th = sample max);"
          f" interval = [min,max]; distribution-free coverage of population median = {cov:.4f}; distinct resamples = {distinct}")
rng = np.random.default_rng(1)
for n in (3,5):
    x = np.sort(rng.lognormal(size=n))
    bs = np.median(rng.choice(x,(20000,n)),axis=1)
    lo,hi = np.percentile(bs,[2.5,97.5])
    print(f"  simulation n={n}: sample min/max {x[0]:.4f}/{x[-1]:.4f}; bootstrap 2.5/97.5 pct {lo:.4f}/{hi:.4f}")
# coverage simulation of percentile-bootstrap CI for median under a bimodal population
def cover(n, reps=4000):
    hit=0
    for _ in range(reps):
        pop_med = 1.0
        # bimodal: 60% fast mode ~1.0, 40% slow mode ~2.5 (lognormal jitter); true median approx computed below
        z = np.where(rng.random(n)<0.6, rng.lognormal(0,0.05,n), rng.lognormal(math.log(2.5),0.1,n))
        bs = np.median(rng.choice(z,(2000,n)),axis=1)
        lo,hi = np.percentile(bs,[2.5,97.5])
        hit += lo <= TRUE_MED <= hi
    return hit/reps
big = np.where(rng.random(2_000_000)<0.6, rng.lognormal(0,0.05,2_000_000), rng.lognormal(math.log(2.5),0.1,2_000_000))
TRUE_MED = float(np.median(big))
for n in (3,5,10):
    print(f"  coverage of nominal 95% percentile-bootstrap CI for median, bimodal population, n={n}: {cover(n):.3f}")
hdr("Exact sign test: smallest attainable two-sided p with all blocks in one direction")
for n in (3,5,10): print(f"  n={n}: p_min = {2*0.5**n:.4f}")
print("  Multiplicity: D1/D1R/H100 scorecard = 13 cells x 4 predictions (+P3 split) = 52-65 tests; E-LOCK K1-K5 x 3 E; E-SPLIT S0-S3 x cells")
print(f"  Bonferroni alpha for 52 tests = {0.05/52:.5f}; unattainable with n<=5 blocks by any exact rank/sign test (min p=0.0625)")

hdr("Telemetry vs device time: does the heater's clock change (Table 2) predict its device-time change (Table 7)?")
# (gpu, E, light SM off, light SM heater, Table7 light device ratio)
TT = [("laptop",1000,2130,2130,1.00),("laptop",3000,1080,1912,0.65),("laptop",10000,585,1215,0.46),
      ("pods",1000,2520,2550,0.90),("pods",3000,2505,2550,0.87),("pods",10000,2235,2520,0.82),
      ("H100",1000,1605,1980,0.86),("H100",3000,1275,1980,0.71),("H100",10000,645,1980,0.81)]
for g,E,off,on,dr in TT:
    pred = off/on   # device ratio expected if time ~ 1/SM clock (laptop E-SPLIT: s~1 for selection)
    print(f"  {g:6} E={E:>5}: SM {off}->{on} MHz => predicted device ratio {pred:.2f}; observed {dr:.2f}; observed/predicted {dr/pred:.2f}")
print("  pods E=10k: 'boost' 2820 vs light-median 2235 => at most {:.2f}x slowdown from the median clock, vs 4.61x process spread".format(2820/2235))
print("  laptop Table 2 light E=10k 585 MHz vs Fig.1b caption 'about 500 MHz' (one process) vs E-LOCK low lock 585 MHz")
