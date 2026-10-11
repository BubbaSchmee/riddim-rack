"""GTS v1 measurement audit. Renders through the real engine (crates/gts/examples/audit_render.rs)
and measures. Run from Git Bash after building the example:
  cargo build -p gts --release --example audit_render
  py -3.12 docs/audits/gts_audit.py
Prints every number used in gts-v1-audit.md and writes short listening renders to renders/.
"""
import os, subprocess, sys, tempfile
import numpy as np
from scipy import signal
from scipy.io import wavfile

SR = 48000
EXE = r"D:/cargo-target/riddim-rack/release/examples/audit_render.exe"
HERE = os.path.dirname(os.path.abspath(__file__))
RENDERS = os.path.join(HERE, "renders")
TMP = tempfile.mkdtemp(prefix="gts_audit_")
os.makedirs(RENDERS, exist_ok=True)
TABLES = ["Square Morph", "Saw Stack", "Harmonic Rise", "Vowel", "FM Morph", "Metal", "Reed", "Grit"]
NOTES = {"C1": 24, "C2": 36, "C3": 48, "C4": 60, "C5": 72, "C6": 84}


def hz(n):
    return 440.0 * 2 ** ((n - 69) / 12)


_jobs = []


def job(name, secs, **kv):
    path = os.path.join(TMP, name + ".f32")
    _jobs.append(f"{path} {secs} " + " ".join(f"{k}={v}" for k, v in kv.items()))
    return path


def impulse(name, ty, freq, res, secs=1.0):
    path = os.path.join(TMP, name + ".f32")
    _jobs.append(f"impulse {path} {ty} {freq} {res} {secs}")
    return path


def run_jobs(extra=()):
    jf = os.path.join(TMP, "jobs.txt")
    with open(jf, "w") as f:
        f.write("\n".join(list(_jobs) + list(extra)))
    out = subprocess.run([EXE, jf], capture_output=True, text=True, check=True).stdout
    _jobs.clear()
    return out


def load(p):
    return np.fromfile(p, dtype="<f4").astype(np.float64)


def wav(name, x, gain=1.0):
    y = np.clip(np.asarray(x) * gain, -1, 1)
    wavfile.write(os.path.join(RENDERS, name + ".wav"), SR, (y * 32767).round().astype(np.int16))


def db(x):
    return 10 * np.log10(np.maximum(x, 1e-30))


def spectrum(x):
    w = signal.windows.blackmanharris(len(x))
    X = np.fft.rfft(x * w)
    return np.fft.rfftfreq(len(x), 1 / SR), np.abs(X) ** 2


def harm_ratio(x, f0, extra_lines=()):
    """Non-harmonic / harmonic energy (dB). Harmonic bins: within 6 FFT bins (Blackman-Harris main
    lobe is 4) of k*f0 and of any `extra_lines` (expected inharmonic components). DC (< 15 Hz)
    ignored. The brief's +/-1 % tolerance was tried first and rejected: at C5 the aliases land
    0.27*f0 from a harmonic, inside a 1 % band once k > 27, so it reported C5 clean. GTS pitch is
    exact (f64 grain phase), so a fixed tolerance loses no harmonic energy."""
    f, P = spectrum(x)
    df = f[1]
    tol_floor = 6 * df
    mask = np.zeros_like(P, bool)
    k = np.round(f / f0)
    near = np.abs(f - k * f0) <= tol_floor
    mask |= near & (k >= 1)
    for e in extra_lines:
        mask |= np.abs(f - e) <= tol_floor
    valid = f > 15
    h = P[mask & valid].sum()
    nh = P[~mask & valid].sum()
    return db(nh / h), f, P, mask


def rms_db(x):
    return 20 * np.log10(np.sqrt(np.mean(x ** 2)) + 1e-30)


def kweight_lufs(x):
    """ITU-R BS.1770 K-weighted mono loudness (no gating; steady tones)."""
    b1, a1 = [1.53512485958697, -2.69169618940638, 1.19839281085285], [1.0, -1.69065929318241, 0.73248077421585]
    b2, a2 = [1.0, -2.0, 1.0], [1.0, -1.99004745483398, 0.99007225036621]
    y = signal.lfilter(b2, a2, signal.lfilter(b1, a1, x))
    return -0.691 + 10 * np.log10(np.mean(y ** 2) + 1e-30)


def polyblep_osc(f0, n, kind, pw=0.5):
    t = np.arange(n) * f0 / SR % 1.0
    dt = f0 / SR

    def blep(t):
        r = np.zeros_like(t)
        m = t < dt
        u = t[m] / dt
        r[m] = 2 * u - u * u - 1
        m = t > 1 - dt
        u = (t[m] - 1) / dt
        r[m] = u * u + 2 * u + 1
        return r

    if kind == "saw":
        return 2 * t - 1 - blep(t)
    y = np.where(t < pw, 1.0, -1.0) + blep(t) - blep((t - pw) % 1.0)
    return y - y.mean()


def ideal_osc(f0, n, kind, pw=0.5):
    """Additive band-limited (every harmonic below 20 kHz): the 'premium' reference."""
    t = np.arange(n) / SR
    y = np.zeros(n)
    for k in range(1, int(20000 / f0) + 1):
        if kind == "saw":
            y += np.sin(2 * np.pi * k * f0 * t) / k
        else:
            y += np.sin(np.pi * k * pw) / k * np.cos(2 * np.pi * k * f0 * t)
    return y


def band_db(f, P, lo, hi, ref):
    return db(P[(f >= lo) & (f < hi)].sum() / ref)


ss = slice(int(0.25 * SR), int(1.0 * SR))  # steady-state analysis window
out = {}

# ---------------------------------------------------------------- (1) aliasing map + (2) loudness
for t in range(8):
    for idx in (0, 64, 127):
        for nn, n in NOTES.items():
            for sh in (0, 12):
                out[("al", t, idx, nn, sh)] = job(f"al_{t}_{idx}_{nn}_{sh}", 2.0, base="raw", table=t, index=idx, note=n, shift=sh)
for t in (0, 1):
    for idx in range(0, 128, 8):
        out[("ix", t, idx)] = job(f"ix_{t}_{idx}", 1.0, base="raw", table=t, index=idx, note=36)
    out[("ix", t, 127)] = job(f"ix_{t}_127", 1.0, base="raw", table=t, index=127, note=36)
# ---------------------------------------------------------------- (3) attack / click
for att, rel in ((0.001, 0.001), (0.005, 0.06)):
    out[("clk", att)] = job(f"clk_{att}", 0.8, base="raw", table=0, index=64, note=36, att=att, rel=rel, off=0.5)
    out[("clkd", att)] = job(f"clkd_{att}", 0.8, table=0, note=29, att=att, rel=rel, off=0.5)
# filter tail truncation at voice end: Comb- res 1.0 and LP12 res 1.0, default rel 60 ms
out["tail_comb"] = job("tail_comb", 1.2, note=29, fa_res=1.0, off=0.5)
out["tail_lp"] = job("tail_lp", 1.2, note=29, fa_type=0, fa_freq=400, fa_res=1.0, fa_env=0, off=0.5)
# ---------------------------------------------------------------- (3b/4) grain boundary, AM ripple
SHIFTS = (-24, -12, -7, -5, 0, 5, 7, 12, 19, 24)
for sh in SHIFTS:
    out[("gs", sh)] = job(f"gs_{sh}", 1.0, base="raw", table=0, index=64, note=36, shift=sh)
    out[("gsin", sh)] = job(f"gsin_{sh}", 1.0, base="raw", table=2, index=0, note=36, shift=sh)
# ---------------------------------------------------------------- (5) shaper
for mode, mname in ((1, "Saturate"), (2, "Clip"), (3, "Quant")):
    for amt in (0.5, 1.0):
        out[("sh", mname, amt)] = job(f"sh_{mname}_{amt}", 1.0, base="raw", table=2, index=127, note=48, shaper_on=1, shaper_mode=mode, shaper_amt=amt)
        out[("shs", mname, amt)] = job(f"shs_{mname}_{amt}", 1.0, base="raw", table=2, index=0, note=48, shaper_on=1, shaper_mode=mode, shaper_amt=amt)
        out[("shhi", mname, amt)] = job(f"shhi_{mname}_{amt}", 1.0, base="raw", table=2, index=127, note=72, shaper_on=1, shaper_mode=mode, shaper_amt=amt)
out["sh_off"] = job("sh_off", 1.0, base="raw", table=2, index=127, note=48)
out["sh_off_hi"] = job("sh_off_hi", 1.0, base="raw", table=2, index=127, note=72)
out["sh_def_on"] = job("sh_def_on", 1.0, base="raw", table=0, index=64, note=36, shaper_on=1)
out["sh_def_off"] = job("sh_def_off", 1.0, base="raw", table=0, index=64, note=36)
# ---------------------------------------------------------------- (6) filters
for ty, fr, res in ((3, 880, 0.6), (3, 880, 1.0), (2, 880, 0.6), (2, 880, 1.0), (0, 1000, 1.0), (0, 1000, 0.0), (1, 1000, 1.0)):
    out[("imp", ty, res)] = impulse(f"imp_{ty}_{res}", ty, fr, res, 2.0)
# default patch, filter vs bypass, sustain-portion snapshot at F1 and C2 (moda off so the index is static)
for n in (29, 36):
    out[("fd", n)] = job(f"fd_{n}", 1.5, note=n, moda_index=0)
    out[("fd_byp", n)] = job(f"fd_byp_{n}", 1.5, note=n, moda_index=0, routing=2)
    out[("fd_r1", n)] = job(f"fd_r1_{n}", 1.5, note=n, moda_index=0, fa_res=1.0)
    out[("fdp", n)] = job(f"fdp_{n}", 1.5, note=n, moda_index=0, fa_type=2)
    out[("fdp_r1", n)] = job(f"fdp_r1_{n}", 1.5, note=n, moda_index=0, fa_type=2, fa_res=1.0)
# LP12 res 1.0 in the engine: cutoff on the 4th harmonic of C2 (261.6 Hz), level vs res 0
out["lp_r1"] = job("lp_r1", 1.0, note=36, moda_index=0, fa_type=0, fa_freq=261.63, fa_kbd=0, fa_env=0, fa_res=1.0)
out["lp_r0"] = job("lp_r0", 1.0, note=36, moda_index=0, fa_type=0, fa_freq=261.63, fa_kbd=0, fa_env=0, fa_res=0.0)
# AM: saw C3, wet, carrier 5 kHz / 15 kHz
for fm in (5000, 15000):
    out[("am", fm)] = job(f"am_{fm}", 1.0, base="raw", routing=0, table=2, index=127, note=48, fa_type=4, fa_freq=fm, fa_res=1.0, fa_kbd=0, fa_env=0, volume=-12)
# ---------------------------------------------------------------- (7) envelope shape
out["env"] = job("env", 1.5, base="raw", table=2, index=0, note=60, att=0.1, dec=0.3, sus=0.5, rel=0.3, off=0.8)
# ---------------------------------------------------------------- (8) porta
out["porta"] = job("porta", 1.0, base="raw", table=2, index=0, note=36, note2="48@0.3", off=0.8)
out["porta_def"] = job("porta_def", 1.0, note=36, note2="48@0.3", off=0.8)
# ---------------------------------------------------------------- (9) default patch snapshot
out["def_F1"] = job("def_F1", 1.0, note=29)
out["def_C2"] = job("def_C2", 1.0, note=36)
out["def_F1_static"] = job("def_F1_static", 1.0, note=29, moda_index=0)
# ---------------------------------------------------------------- renders-only
out["r_default_F1_notes"] = job("r_def_notes", 2.0, note=29, off=0.18)

bench = run_jobs(["bench 1 4", "bench 8 4"])
print("== (10) CPU\n" + bench)

# ================================================================ analysis
print("== (1) aliasing map: non-harmonic/harmonic dB (flag > -40)")
flags = []
amap = {}
for t in range(8):
    for idx in (0, 64, 127):
        row = []
        for sh in (0, 12):
            for nn, n in NOTES.items():
                x = load(out[("al", t, idx, nn, sh)])[int(0.25 * SR):]
                r, *_ = harm_ratio(x, hz(n))
                amap[(t, idx, nn, sh)] = r
                row.append(f"{r:6.1f}")
                if r > -40:
                    flags.append((r, TABLES[t], idx, nn, sh))
        print(f"{TABLES[t]:13s} idx{idx:3d} sh0: " + " ".join(row[:6]) + " | sh+12: " + " ".join(row[6:]))
flags.sort(reverse=True)
print(f"flagged cells: {len(flags)} / {8*3*6*2}")
for f_ in flags[:12]:
    print("  ", f_)
by_note = {nn: [amap[(t, i, nn, s)] for t in range(8) for i in (0, 64, 127) for s in (0, 12)] for nn in NOTES}
for nn, v in by_note.items():
    print(f"  {nn}: median {np.median(v):6.1f} dB  worst {max(v):6.1f} dB  flagged {sum(np.array(v) > -40)}/48")

# top harmonic present vs what a full-band oscillator would carry
print("\n== bandwidth: highest frequency carried by GTS grains (harmonic 64 x f0) vs 20 kHz")
for nn, n in NOTES.items():
    f0 = hz(n)
    print(f"  {nn} f0={f0:7.1f} Hz  GTS top={64*f0:8.0f} Hz  full-band harmonics needed={int(20000/f0)}")

print("\n== (2) loudness")
lv = []
for t in range(8):
    x = load(out[("al", t, 64, "C2", 0)])[ss]
    lv.append((rms_db(x), kweight_lufs(x)))
    print(f"  {TABLES[t]:13s} C2 idx64 RMS {lv[-1][0]:6.1f} dBFS  K-wtd {lv[-1][1]:6.1f} LUFS")
r = np.array(lv)
print(f"  spread RMS {r[:,0].max()-r[:,0].min():.1f} dB, K-weighted {r[:,1].max()-r[:,1].min():.1f} dB")
for t in (0, 1):
    vals = [(i, rms_db(load(out[("ix", t, i)])[ss]), kweight_lufs(load(out[("ix", t, i)])[ss])) for i in list(range(0, 128, 8)) + [127]]
    a = np.array([v[1] for v in vals]); k = np.array([v[2] for v in vals])
    print(f"  {TABLES[t]} index 0..127 RMS range {a.min():.1f}..{a.max():.1f} ({a.max()-a.min():.1f} dB), K-wtd range {k.max()-k.min():.1f} dB; K-wtd at 0/64/127: {k[0]:.1f} {k[8]:.1f} {k[-1]:.1f}")

print("\n== (3) clicks: max |x[n]-x[n-1]|")
for att in (0.001, 0.005):
    for key, lab in ((("clk", att), "raw osc C2"), (("clkd", att), "default patch F1")):
        x = load(out[key])
        on = np.abs(np.diff(x[: int(0.01 * SR)])).max()
        st = np.abs(np.diff(x[int(0.2 * SR): int(0.45 * SR)])).max()
        o0 = int(0.5 * SR)
        offs = np.abs(np.diff(x[o0 - 10: o0 + int(0.02 * SR)])).max()
        nz = np.nonzero(np.abs(x) > 0)[0]
        last = nz[-1] if len(nz) else 0
        endstep = abs(x[last]) if len(nz) else 0
        print(f"  att/rel {att*1000:.0f}/{(1 if att==0.001 else 60)} ms {lab:17s}: note-on {on:.4f}, steady {st:.4f}, note-off {offs:.4f}, final-sample jump {20*np.log10(endstep+1e-12):.1f} dBFS at {last/SR*1000:.1f} ms")
for key in ("tail_comb", "tail_lp"):
    x = load(out[key])
    nz = np.nonzero(np.abs(x) > 0)[0]
    last = nz[-1]
    pre = np.sqrt(np.mean(x[last - 480: last] ** 2))
    print(f"  {key}: voice cut at {last/SR*1000:.1f} ms, last sample {x[last]:+.5f} ({20*np.log10(abs(x[last])+1e-12):.1f} dBFS), RMS of last 10 ms {20*np.log10(pre+1e-12):.1f} dBFS")

print("\n== (3b/4) grain scheme at Shift != 0 (Square Morph idx64 C2, and pure sine = Harmonic Rise idx0 C2)")
f0 = hz(36)
for sh in SHIFTS:
    x = load(out[("gs", sh)])[ss]
    r, f, P, mask = harm_ratio(x, f0)
    fund = P[np.abs(f - f0) < 3].sum()
    sub = P[np.abs(f - f0 / 2) < 3].sum()
    s = load(out[("gsin", sh)])[ss]
    env = np.abs(signal.hilbert(s))[2000:-2000]
    depth = (env.max() - env.min()) / (env.max() + env.min())
    rs, fs, Ps, _ = harm_ratio(s, f0)
    # where the sine's energy went: share in the two harmonics nearest 2^(sh/12)*f0
    tgt = f0 * 2 ** (sh / 12)
    hk = np.argsort(np.abs(np.arange(1, 400) * f0 - tgt))[:2] + 1
    near2 = sum(Ps[np.abs(fs - k * f0) < 3].sum() for k in hk) / Ps[fs > 15].sum()
    print(f"  shift {sh:+3d}: square RMS {rms_db(x):5.1f} dBFS, non-harm {r:6.1f} dB, f0/2 {db(sub/fund):6.1f} dBc | sine: AM depth {100*depth:5.1f} %, non-harm {rs:6.1f} dB, energy in 2 nearest harmonics {100*near2:5.1f} %, RMS {rms_db(s):5.1f} dBFS")

print("\n== (5) shaper (input: Harmonic Rise idx127 = 64-harmonic saw; sine = idx0) ")
x0 = load(out["sh_off"])[ss]
r0, *_ = harm_ratio(x0, hz(48))
x0h = load(out["sh_off_hi"])[ss]
r0h, *_ = harm_ratio(x0h, hz(72))
print(f"  shaper off: C3 non-harm {r0:.1f} dB, C5 {r0h:.1f} dB, C3 RMS {rms_db(x0):.1f}")
for mname in ("Saturate", "Clip", "Quant"):
    for amt in (0.5, 1.0):
        x = load(out[("sh", mname, amt)])[ss]
        r, f, P, mask = harm_ratio(x, hz(48))
        xh = load(out[("shhi", mname, amt)])[ss]
        rh, *_ = harm_ratio(xh, hz(72))
        s = load(out[("shs", mname, amt)])[ss]
        fs, Ps = spectrum(s)
        f0s = hz(48)
        h = [Ps[np.abs(fs - k * f0s) < 3].sum() for k in range(1, int(23000 / f0s))]
        thd = 10 * np.log10(sum(h[1:]) / h[0])
        rs, *_ = harm_ratio(s, f0s)
        print(f"  {mname:8s} amt {amt}: sine THD {thd:6.1f} dB ({100*10**(thd/20):5.1f} %), sine alias {rs:6.1f} dB | saw C3 alias {r:6.1f} dB, saw C5 alias {rh:6.1f} dB, saw RMS {rms_db(x):5.1f} (dry {rms_db(x0):.1f})")
print(f"  default shaper (Saturate 0.3) toggle on Square Morph C2: {rms_db(load(out['sh_def_on'])[ss]) - rms_db(load(out['sh_def_off'])[ss]):+.1f} dB")

print("\n== (6) filters")
names = {0: "LP12", 1: "BP12", 2: "Comb+", 3: "Comb-"}
for (k, v) in [(kk, vv) for kk, vv in out.items() if isinstance(kk, tuple) and kk[0] == "imp"]:
    _, ty, res = k
    h = load(v)
    H = np.abs(np.fft.rfft(h, 1 << 18))
    fr = np.fft.rfftfreq(1 << 18, 1 / SR)
    g = lambda f: 20 * np.log10(H[np.argmin(np.abs(fr - f))] + 1e-12)
    if ty in (2, 3):
        fc = 880.0
        print(f"  {names[ty]} {fc:.0f} Hz res {res}: peak(880) {g(880):+.1f} dB, between teeth(1320) {g(1320):+.1f} dB, DC {g(5):+.1f} dB, 440 {g(440):+.1f} dB")
    else:
        e = np.abs(h)
        pk = 20 * np.log10(H.max())
        idx = np.nonzero(e > 1e-3 * e.max())[0][-1]
        print(f"  {names[ty]} 1 kHz res {res}: peak gain {pk:+.1f} dB at {fr[np.argmax(H)]:.0f} Hz, gain@1k {g(1000):+.1f} dB, @4k {g(4000):+.1f}, ring to -60 dB {idx/SR*1000:.1f} ms (no self-oscillation if finite)")
sl = slice(int(0.8 * SR), int(1.4 * SR))
for n in (29, 36):
    f0n = hz(n)
    fs, Pb = spectrum(load(out[("fd_byp", n)])[sl])
    for key, lab in ((("fd", n), "Comb- 880 res .6 (default)"), (("fd_r1", n), "Comb- res 1.0"), (("fdp", n), "Comb+ res .6"), (("fdp_r1", n), "Comb+ res 1.0")):
        xf = load(out[key])
        _, Pf = spectrum(xf[sl])
        hg = []
        for k in range(1, 40):
            m = np.abs(fs - k * f0n) < 2.5
            hg.append(db(Pf[m].sum() / Pb[m].sum()))
        hg = np.array(hg)
        pk = np.abs(xf).max()
        print(f"  F{'1' if n==29 else '_C2'} {lab:26s}: fundamental {hg[0]:+.1f} dB, harmonics 2-39 min {hg[1:].min():+.1f} max {hg[1:].max():+.1f} dB, overall {rms_db(xf[sl]) - rms_db(load(out[('fd_byp', n)])[sl]):+.1f} dB, peak {pk:.3f}")
xr1, xr0 = load(out["lp_r1"])[ss], load(out["lp_r0"])[ss]
print(f"  LP12 at 4th harmonic of C2, res 1.0 vs 0: {rms_db(xr1)-rms_db(xr0):+.1f} dB, peak {np.abs(load(out['lp_r1'])).max():.3f} (limiter knee 0.95)")
for fm in (5000, 15000):
    x = load(out[("am", fm)])[ss]
    f0a = hz(48)
    lines = [abs(k * f0a + s * fm) for k in range(1, 65) for s in (-1, 1)]
    r, f, P, mask = harm_ratio(x, f0a, extra_lines=lines)
    print(f"  AM carrier {fm} Hz, C3 64-harm saw: energy outside expected k*f0 +/- fc lines {r:.1f} dB")

print("\n== (7) envelope (A 100 ms, D 300 ms, S 0.5, R 300 ms; sine C4)")
x = load(out["env"])
env = np.abs(signal.hilbert(x))
env = signal.medfilt(env, 1)
smooth = np.convolve(env, np.ones(48) / 48, "same")
pk = smooth[int(0.15 * SR): int(0.2 * SR)].max() if False else smooth[int(0.09 * SR): int(0.12 * SR)].max()
for frac in (0.25, 0.5, 0.75, 1.0):
    print(f"  attack at {frac*100:.0f}% of A: {smooth[int(frac*0.1*SR)]/pk:.3f} (linear would be {frac:.2f})")
susl = smooth[int(0.7 * SR)] / pk
print(f"  sustain level {susl:.3f} (set 0.5); decay at 0.1+0.3 s: {smooth[int(0.4*SR)]/pk:.3f}")
o = int(0.8 * SR)
for ms in (30, 60, 150, 270, 300):
    print(f"  release +{ms} ms: {20*np.log10(smooth[o+int(ms*SR/1000)]/pk+1e-12):6.1f} dB")
nz = np.nonzero(np.abs(x) > 0)[0]
print(f"  last nonzero sample at {(nz[-1]-o)/SR*1000:.1f} ms after note-off, value {20*np.log10(abs(x[nz[-1]])+1e-12):.1f} dBFS")

print("\n== (8) porta (C2 -> C3 legato at 0.3 s, porta 80 ms)")
x = load(out["porta"])
an = signal.hilbert(x)
inst = np.diff(np.unwrap(np.angle(an))) * SR / (2 * np.pi)
inst = np.convolve(inst, np.ones(256) / 256, "same")
st = 12 * np.log2(np.maximum(inst, 1) / hz(36))
t0 = int(0.3 * SR)
for target in (0.1, 0.5, 0.9, 0.98):
    i = np.argmax(st[t0:] >= 12 * target)
    print(f"  {target*100:.0f}% of the 12 st glide reached at {i/SR*1000:.1f} ms")
print(f"  pitch at +2 ms {st[t0+96]:.2f} st, +10 ms {st[t0+480]:.2f} st (exponential = fastest at the start)")

print("\n== (9) default patch vs references")
f0 = hz(36)
n = len(range(ss.start, ss.stop))
refs = {
    "PolyBLEP square C2": polyblep_osc(f0, n, "sq"), "PolyBLEP saw C2": polyblep_osc(f0, n, "saw"),
    "full-band square C2": ideal_osc(f0, n, "sq"), "full-band saw C2": ideal_osc(f0, n, "saw"),
}
cmp = {"GTS Square Morph idx0 (50% sq) C2": load(out[("al", 0, 0, "C2", 0)])[ss],
       "GTS Saw Stack idx0 (1 saw) C2": load(out[("al", 1, 0, "C2", 0)])[ss]}
cmp.update(refs)
for lab, x in cmp.items():
    x = x / np.sqrt(np.mean(x ** 2))
    f, P = spectrum(x)
    tot = P[f > 15].sum()
    r, *_ = harm_ratio(x, f0)
    print(f"  {lab:34s}: >4.2k {band_db(f,P,4200,24000,tot):6.1f} dB, 5-10k {band_db(f,P,5000,10000,tot):6.1f}, 10-20k {band_db(f,P,10000,20000,tot):6.1f}, non-harm {r:6.1f} dB, centroid {np.sum(f*P)/np.sum(P):6.0f} Hz")
for key, nn in (("def_F1", 29), ("def_C2", 36), ("def_F1_static", 29)):
    x = load(out[key])
    xs = x[int(0.05 * SR): int(0.95 * SR)]
    r, f, P, _ = harm_ratio(xs, hz(nn))
    tot = P[f > 15].sum()
    print(f"  default {key:14s}: RMS {rms_db(xs):5.1f} dBFS, K-wtd {kweight_lufs(xs):5.1f} LUFS, peak {np.abs(x).max():.3f}, non-harm {r:6.1f} dB, >5k {band_db(f,P,5000,24000,tot):6.1f} dB, centroid {np.sum(f*P)/np.sum(P):5.0f} Hz")
fA, PA = spectrum(load(out[("al", 1, 0, "C6", 12)])[ss])
print()

# ================================================================ listening renders
def cat(*xs, gap=0.15):
    g = np.zeros(int(gap * SR))
    return np.concatenate([np.concatenate([x, g]) for x in xs])

def norm(x, to=-18.0):
    return x * 10 ** ((to - rms_db(x)) / 20)

seg = lambda key: load(out[key])[: int(0.8 * SR)]
wav("01_default_patch_F1_one_note", load(out["def_F1"]))
wav("02_bandwidth_GTS_square_vs_fullband_square_C1_C2", cat(norm(seg(("al", 0, 0, "C1", 0))), norm(ideal_osc(hz(24), int(0.8 * SR), "sq")), norm(seg(("al", 0, 0, "C2", 0))), norm(ideal_osc(hz(36), int(0.8 * SR), "sq"))))
wav("03_aliasing_worst_cells_C5_C6", cat(*[norm(seg(("al", t, i, nn, s))) for (r_, tn, i, nn, s) in flags[:4] for t in [TABLES.index(tn)]]))
wav("04_aliasing_saw_stack_C6_shift12_vs_polyblep", cat(norm(seg(("al", 1, 0, "C6", 12))), norm(polyblep_osc(hz(96), int(0.8 * SR), "saw"))))
wav("05_click_default_patch_att1ms_rel1ms_vs_5_60ms", cat(load(out[("clkd", 0.001)]), load(out[("clkd", 0.005)])), 2.0)
wav("06_voice_end_cuts_ringing_filter_LP12res1_then_CombMinus_res1", cat(load(out["tail_lp"]), load(out["tail_comb"])))
wav("07_shift_sweep_steps_sine_minus24_to_plus24", cat(*[norm(seg(("gsin", s))) for s in SHIFTS], gap=0.05))
wav("08_shift_steps_square_minus24_to_plus24", cat(*[norm(seg(("gs", s))) for s in SHIFTS], gap=0.05))
wav("09_shaper_dry_sat_clip_quant_amt1_C5", cat(norm(seg("sh_off_hi")), *[norm(seg(("shhi", m, 1.0))) for m in ("Saturate", "Clip", "Quant")]))
wav("10_comb_minus_vs_plus_res1_F1", cat(load(out[("fd_r1", 29)]), load(out[("fdp_r1", 29)])))
wav("11_lp12_res1_vs_res0", cat(load(out["lp_r1"]), load(out["lp_r0"])))
wav("12_porta_C2_to_C3_80ms", load(out["porta_def"]))
wav("13_table_tour_C2_idx64", cat(*[norm(seg(("al", t, 64, "C2", 0))) for t in range(8)], gap=0.1))
wav("14_am_15k_carrier_C3_saw", norm(load(out[("am", 15000)])))
tot = sum(os.path.getsize(os.path.join(RENDERS, f)) for f in os.listdir(RENDERS))
print(f"renders total {tot/1e6:.2f} MB")
