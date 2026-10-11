"""Reference comparator: measures a REF render and a TEST render segment by segment and prints
the distance between them. Analysis only; nothing from either file is copied anywhere.

    py -3.12 tools/compare.py REF.wav TEST.wav [--segments auto|N|t0,t1,...]
        [--test-segments auto|N|t0,t1,...] [--pair i,j,...] [--f0 130.81] [--match-level]
        [--plots DIR] [--json OUT.json]

Note: Ableton names MIDI 60 'C3' (261.63 Hz); pass that --f0 for an Ableton 'C3'.

Segments: `auto` = spectral-change detection (log-mel novelty between consecutive 46 ms frames,
peaks at least 1 s apart and above 6x the median novelty) inside the active span; `N` = N equal
parts of the active span; `t0,t1,...` = boundaries in seconds. TEST uses --test-segments (default:
the same spec as REF). --pair maps TEST segment k to REF segment pair[k] (default identity).
--match-level scales each TEST segment to its REF segment's RMS before the distance metrics and
prints the gain.

Per segment: peak / RMS dBFS, BS.1770 gated LUFS; f0 (autocorrelation, refined on the spectrum);
harmonics 1..40 dB re the fundamental (Blackman-Harris, steady part = segment minus 0.3 s each
edge); tilt (dB/oct least-squares over harmonics 200 Hz..10 kHz above -100 dBc, plus odd-only and
even-only fits); -60 dBc bandwidth; non-harmonic floor (power outside +-1 % of k*f0 (at least +-5 bins), 20 Hz..Nyquist,
dB re total); DC; attack (onset to 90 % of steady amplitude) and release (last sample >= steady -3
dB to steady -60 dB) at the segment edges, on a one-period sliding RMS (resolution ~ half a period);
L/R correlation.
Distances per pair: mean |dB diff| over harmonics 1..40 (each clipped at -100 dBc), log-mel RMS dB
distance (64 mels 30 Hz..Nyquist, 2048-sample frames at 44.1 kHz = 46 ms, hop 1024, floor -100 dB
re the pair's max), RMS difference of the momentary-loudness (400 ms) envelopes in LU.
"""
import argparse
import json
import sys

import numpy as np
import soundfile as sf
from scipy.signal import bilinear, find_peaks, lfilter
from scipy.signal.windows import blackmanharris

NH = 40
FLOOR = -100.0
EDGE = 0.3


def db(p):
    return 10 * np.log10(np.maximum(p, 1e-30))


def load(path):
    x, sr = sf.read(path, always_2d=True, dtype="float64")
    if x.shape[1] == 1:
        x = np.repeat(x, 2, axis=1)
    return x[:, :2], sr


def k_filter(sr):
    """BS.1770 K-weighting at any rate (pre-filter shelf + RLB high-pass, analog prototypes)."""
    out = []
    for kind, fc, q, g in (("shelf", 1681.974450955533, 0.7071752369554196, 3.999843853973347), ("hp", 38.13547087602444, 0.5003270373238773, 0.0)):
        k = np.tan(np.pi * fc / sr)
        if kind == "shelf":
            vh = 10 ** (g / 20)
            vb = vh ** 0.4996667741545416
            a0 = 1 + k / q + k * k
            b = [(vh + vb * k / q + k * k) / a0, 2 * (k * k - vh) / a0, (vh - vb * k / q + k * k) / a0]
            a = [1, 2 * (k * k - 1) / a0, (1 - k / q + k * k) / a0]
        else:
            a0 = 1 + k / q + k * k
            b = [1, -2, 1]
            a = [1, 2 * (k * k - 1) / a0, (1 - k / q + k * k) / a0]
        out.append((b, a))
    return out


def k_weight(x, sr):
    for b, a in k_filter(sr):
        x = lfilter(b, a, x, axis=0)
    return x


def momentary(x, sr, hop=0.1):
    """400 ms block mean-square of the K-weighted stereo signal, every `hop` s."""
    k = k_weight(x, sr)
    p = (k ** 2).sum(axis=1)
    blk, h = int(0.4 * sr), int(hop * sr)
    c = np.concatenate([[0], np.cumsum(p)])
    starts = np.arange(0, max(len(p) - blk, 0) + 1, h)
    return (c[starts + blk] - c[starts]) / blk


def lufs(x, sr):
    z = momentary(x, sr)
    if len(z) == 0:
        return float("nan")
    loud = -0.691 + db(z)
    z = z[loud > -70]
    if len(z) == 0:
        return float("-inf")
    rel = -0.691 + db(z.mean()) - 10
    z = z[-0.691 + db(z) > rel]
    return float(-0.691 + db(z.mean()))


def mel_fb(sr, n_fft, n_mels=64, fmin=30.0):
    mel = lambda f: 2595 * np.log10(1 + f / 700)
    imel = lambda m: 700 * (10 ** (m / 2595) - 1)
    pts = imel(np.linspace(mel(fmin), mel(sr / 2), n_mels + 2))
    f = np.fft.rfftfreq(n_fft, 1 / sr)
    fb = np.zeros((n_mels, len(f)))
    for i in range(n_mels):
        lo, c, hi = pts[i : i + 3]
        fb[i] = np.clip(np.minimum((f - lo) / (c - lo), (hi - f) / (hi - c)), 0, None)
    return fb


def logmel(m, sr):
    n = 1 << int(round(np.log2(0.046 * sr)))  # 2048 at 44.1/48 kHz
    hop = n // 2
    w = np.hanning(n)
    frames = np.lib.stride_tricks.sliding_window_view(m, n)[::hop] * w
    p = np.abs(np.fft.rfft(frames, axis=1)) ** 2
    return mel_fb(sr, n) @ p.T  # (64, frames) power


def active_span(m, sr):
    nz = np.nonzero(np.abs(m) > 10 ** (-80 / 20))[0]
    return (nz[0] / sr, (nz[-1] + 1) / sr) if len(nz) else (0, len(m) / sr)


def refine_cut(m, sr, t, f0):
    """Moves a coarse cut (frame resolution) to the sample where the waveform stops repeating:
    (x[i] - x[i - P])^2 smoothed over one period P peaks half a period after the change."""
    P = int(round(sr / f0))
    a, b = max(P, int((t - 0.1) * sr)), min(len(m), int((t + 0.1) * sr))
    d = (m[a:b] - m[a - P : b - P]) ** 2
    sm = np.convolve(d, np.ones(P) / P, "same")
    return (a + int(np.argmax(sm)) - P // 2) / sr


def segment(m, sr, spec, hint=130.81):
    t0, t1 = active_span(m, sr)
    if spec == "auto":
        mp = db(logmel(m, sr))
        hop = (1 << int(round(np.log2(0.046 * sr)))) // 2
        mp = np.maximum(mp, mp.max() - 100)
        nov = np.sqrt(((np.diff(mp, axis=1)) ** 2).mean(axis=0))
        fr_t = (np.arange(len(nov)) + 1) * hop / sr + hop / sr
        inside = (fr_t > t0 + 0.5) & (fr_t < t1 - 0.5)
        nov = np.where(inside, nov, 0)
        thr = 6 * np.median(nov[inside]) if inside.any() else np.inf
        pk, _ = find_peaks(nov, height=thr, distance=max(1, int(1.0 * sr / hop)))
        return [t0] + [refine_cut(m, sr, t, hint) for t in fr_t[pk]] + [t1]
    if spec.isdigit():
        return list(np.linspace(t0, t1, int(spec) + 1))
    return [float(v) for v in spec.split(",")]


def f0_autocorr(m, sr, hint):
    x = m - m.mean()
    n = 1 << int(np.ceil(np.log2(2 * len(x))))
    r = np.fft.irfft(np.abs(np.fft.rfft(x, n)) ** 2)[: len(x)]
    lo, hi = int(sr / (hint * 1.1)), int(sr / (hint / 1.1)) + 1
    k = lo + int(np.argmax(r[lo:hi]))
    a, b, c = r[k - 1], r[k], r[k + 1]
    d = 0.5 * (a - c) / (a - 2 * b + c) if (a - 2 * b + c) != 0 else 0
    return sr / (k + d)


def env_rms(m, sr, f0):
    w = max(1, int(round(sr / f0)))
    c = np.concatenate([[0], np.cumsum(m * m)])
    e = np.zeros(len(m))
    h = w // 2
    i = np.arange(len(m))
    a, b = np.clip(i - h, 0, len(m)), np.clip(i - h + w, 0, len(m))
    e = np.sqrt((c[b] - c[a]) / w)
    return e


def analyse(x, sr, a, b, hint):
    m = x.mean(axis=1)
    seg = x[int(a * sr) : int(b * sr)]
    sm = m[int(a * sr) : int(b * sr)]
    st = m[int((a + EDGE) * sr) : int((b - EDGE) * sr)]
    n = 1 << int(np.floor(np.log2(len(st))))
    st = st[(len(st) - n) // 2 :][:n]
    f0 = f0_autocorr(st[: min(n, 1 << 15)], sr, hint)
    w = blackmanharris(n, sym=False)
    p = np.abs(np.fft.rfft((st - st.mean()) * w)) ** 2
    bw = sr / n
    # refine f0 on harmonics 1..10 (parabolic peak of each, least squares on k)
    est = []
    for k in range(1, 11):
        c = int(round(k * f0 / bw))
        lo, hi = max(1, c - 8), c + 9
        j = lo + int(np.argmax(p[lo:hi]))
        if j <= 0 or j >= len(p) - 1:
            continue
        la, lb, lc = db(p[j - 1 : j + 2])
        dd = 0.5 * (la - lc) / (la - 2 * lb + lc) if (la - 2 * lb + lc) != 0 else 0
        est.append(((j + dd) * bw, k, lb))
    if est:
        top = max(e[2] for e in est)
        good = [(f, k) for f, k, l in est if l > top - 60]
        f0 = sum(f for f, k in good) / sum(k for f, k in good)
    if est and max(l for f, k, l in est if k % 2 == 0) - max(l for f, k, l in est if k % 2 == 1) > 40:
        print(f"  WARNING: odd multiples of {f0:.2f} Hz are 40 dB under the even ones: f0 is probably {2 * f0:.2f} Hz (pass --f0)", file=sys.stderr)
    nh_max = int((sr / 2) / f0)
    amps = np.full(nh_max, FLOOR - 50.0)
    for k in range(1, nh_max + 1):
        c = int(round(k * f0 / bw))
        # strongest bin within +-3 of k*f0, then the power of its +-4-bin main lobe (sum: no
        # scalloping loss with Blackman-Harris)
        lo, hi = max(1, c - 3), min(len(p) - 1, c + 4)
        if lo >= hi:
            continue
        j = lo + int(np.argmax(p[lo:hi]))
        amps[k - 1] = db(p[max(1, j - 4) : j + 5].sum())
    rel = amps - amps[0]
    hdb = np.maximum(rel, FLOOR)
    freqs = f0 * np.arange(1, nh_max + 1)

    def fit(mask):
        sel = mask & (freqs >= 200) & (freqs <= 10000) & (rel > FLOOR)
        if sel.sum() < 2:
            return float("nan")
        return float(np.polyfit(np.log2(freqs[sel]), rel[sel], 1)[0])

    allm = np.ones(nh_max, bool)
    odd = (np.arange(1, nh_max + 1) % 2) == 1
    above = np.nonzero(rel > -60)[0]
    # non-harmonic floor
    f = np.arange(len(p)) * bw
    harm = np.zeros(len(p), bool)
    for k in range(1, nh_max + 2):
        harm |= np.abs(f - k * f0) <= max(0.01 * k * f0, 5 * bw)  # never narrower than the window's main lobe
    band = f >= 20
    tot = p[band].sum()
    nonh = p[band & ~harm].sum()
    # envelope edges
    e = env_rms(m, sr, f0)
    steady = np.sqrt((st ** 2).mean())
    ia, ib = int(a * sr), int(b * sr)
    look = int(0.25 * sr)
    pre = e[max(0, ia - look) : ia + look]
    on = np.nonzero(pre > steady * 10 ** (-60 / 20))[0]
    attack = None
    if len(on) and on[0] == 0 and ia - look > 0:
        on = []  # already sounding before the segment: a switch, not a note-on (attack '-')
    if len(on):
        i_on = on[0]
        reach = np.nonzero(pre[i_on:] >= 0.9 * steady)[0]
        attack = float(reach[0] / sr) if len(reach) else None
        onset_t = (max(0, ia - look) + i_on) / sr
    else:
        onset_t = None
    post = e[max(0, ib - look) : ib + look]
    hold = np.nonzero(post >= steady * 10 ** (-3 / 20))[0]
    release = None
    off_t = None
    if len(hold):
        i_off = hold[-1]
        off_t = (max(0, ib - look) + i_off) / sr
        drop = np.nonzero(post[i_off:] <= steady * 10 ** (-60 / 20))[0]
        release = float(drop[0] / sr) if len(drop) else None
    l, r = seg[:, 0], seg[:, 1]
    corr = float(np.corrcoef(l, r)[0, 1]) if l.std() > 0 and r.std() > 0 else 1.0
    return {
        "t": [a, b],
        "peak_db": float(20 * np.log10(np.abs(seg).max() + 1e-30)),
        "rms_db": float(db((seg ** 2).mean())),
        "lufs": lufs(seg, sr),
        "f0": float(f0),
        "harm_db": [float(v) for v in hdb[:NH]],
        "harm_all_db": [float(v) for v in hdb],
        "tilt": fit(allm),
        "tilt_odd": fit(odd),
        "tilt_even": fit(~odd),
        "bw_harm": int(above[-1] + 1) if len(above) else 0,
        "bw_hz": float((above[-1] + 1) * f0) if len(above) else 0.0,
        "nonharm_db": float(db(nonh / tot)),
        "dc": float(sm.mean()),
        "attack_s": attack,
        "onset_s": onset_t,
        "release_s": release,
        "off_s": off_t,
        "corr": corr,
        "steady_rms": float(steady),
    }


def mel_dist(rm, tm, sr):
    a, b = logmel(rm, sr), logmel(tm, sr)
    n = min(a.shape[1], b.shape[1])
    a, b = db(a[:, :n]), db(b[:, :n])
    top = max(a.max(), b.max())
    a, b = np.maximum(a, top + FLOOR), np.maximum(b, top + FLOOR)
    return float(np.sqrt(((a - b) ** 2).mean()))


def lufs_env_dist(rx, tx, sr):
    a, b = momentary(rx, sr), momentary(tx, sr)
    n = min(len(a), len(b))
    if n == 0:
        return float("nan")
    a, b = -0.691 + db(a[:n]), -0.691 + db(b[:n])
    return float(np.sqrt(((a - b) ** 2).mean()))


def fmt(v, f="{:.1f}"):
    return "-" if v is None or (isinstance(v, float) and np.isnan(v)) else f.format(v)


def report(tag, s):
    print(f"  {tag}: t {s['t'][0]:.3f}-{s['t'][1]:.3f} s  peak {s['peak_db']:.2f} dBFS  RMS {s['rms_db']:.2f} dBFS  LUFS {s['lufs']:.2f}  f0 {s['f0']:.3f} Hz")
    print(f"    tilt {fmt(s['tilt'], '{:.2f}')} dB/oct (odd {fmt(s['tilt_odd'], '{:.2f}')}, even {fmt(s['tilt_even'], '{:.2f}')})  -60 dBc bw: h{s['bw_harm']} = {s['bw_hz']:.0f} Hz  non-harm {s['nonharm_db']:.1f} dB  DC {s['dc']:.2e}  corr {s['corr']:.4f}")
    print(f"    attack {fmt(s['attack_s'] and s['attack_s'] * 1e3, '{:.1f}')} ms (onset {fmt(s['onset_s'], '{:.4f}')} s)  release {fmt(s['release_s'] and s['release_s'] * 1e3, '{:.1f}')} ms (off {fmt(s['off_s'], '{:.4f}')} s)")
    print("    h1-40 dBc: " + " ".join(f"{v:.1f}" for v in s["harm_db"]))


def plot(pairs, outdir, stem):
    import os

    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    os.makedirs(outdir, exist_ok=True)
    paths = []
    for k, (ri, r, t) in enumerate(pairs):
        fig, ax = plt.subplots(figsize=(9, 4), dpi=110)
        nmax = max(len(r["harm_all_db"]), len(t["harm_all_db"]))
        for s, lab, c, off in ((r, "REF", "#1f5fbf", 0.985), (t, "TEST", "#d0602a", 1.015)):
            h = np.array(s["harm_all_db"])
            ax.vlines(np.arange(1, len(h) + 1) * off, FLOOR, h, color=c, lw=1.2, label=lab)
        n = np.arange(1, nmax + 1)
        ax.plot(n, -20 * np.log10(n), ":", color="0.5", lw=0.8, label="1/n")
        ax.plot(n, -40 * np.log10(n), "--", color="0.7", lw=0.8, label="1/n²")
        ax.set_xscale("log")
        ax.set_ylim(FLOOR, 5)
        ax.set_xlabel("harmonic number")
        ax.set_ylabel("dB re fundamental")
        ax.set_title(f"{stem}: REF segment {ri + 1} vs TEST segment {k + 1}")
        ax.legend(loc="lower left", fontsize=8)
        ax.grid(alpha=0.3, which="both")
        p = os.path.join(outdir, f"{stem}-seg{ri + 1}.png")
        fig.tight_layout()
        fig.savefig(p)
        plt.close(fig)
        paths.append(p)
    return paths


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("ref")
    ap.add_argument("test")
    ap.add_argument("--segments", default="auto")
    ap.add_argument("--test-segments", default=None)
    ap.add_argument("--pair", default=None)
    ap.add_argument("--f0", type=float, default=130.81)
    ap.add_argument("--match-level", action="store_true")
    ap.add_argument("--plots", default=None)
    ap.add_argument("--json", default=None)
    o = ap.parse_args(argv)
    rx, sr = load(o.ref)
    tx, srt = load(o.test)
    if sr != srt:
        sys.exit(f"sample rates differ ({sr} vs {srt}); render TEST at {sr}")
    rb = segment(rx.mean(axis=1), sr, o.segments, o.f0)
    tb = segment(tx.mean(axis=1), sr, o.test_segments or o.segments, o.f0)
    print(f"REF {o.ref}: {len(rb) - 1} segments, boundaries " + ", ".join(f"{v:.3f}" for v in rb))
    print(f"TEST {o.test}: {len(tb) - 1} segments, boundaries " + ", ".join(f"{v:.3f}" for v in tb))
    pair = [int(v) - 1 for v in o.pair.split(",")] if o.pair else list(range(len(tb) - 1))
    out, pairs = [], []
    for k, ri in enumerate(pair):
        if k >= len(tb) - 1 or ri >= len(rb) - 1:
            break
        ra, rbnd = rb[ri], rb[ri + 1]
        ta, tbnd = tb[k], tb[k + 1]
        r = analyse(rx, sr, ra, rbnd, o.f0)
        rseg = rx[int(ra * sr) : int(rbnd * sr)]
        tseg = tx[int(ta * sr) : int(tbnd * sr)]
        gain_db = 0.0
        if o.match_level:
            t0 = analyse(tx, sr, ta, tbnd, o.f0)
            gain_db = 20 * np.log10(r["steady_rms"] / t0["steady_rms"])
        g = 10 ** (gain_db / 20)
        txg = tx * g
        t = analyse(txg, sr, ta, tbnd, o.f0)
        tseg = tseg * g
        d_h = float(np.mean(np.abs(np.array(r["harm_db"]) - np.array(t["harm_db"]))))
        d_mel = mel_dist(rseg.mean(axis=1), tseg.mean(axis=1), sr)
        d_lu = lufs_env_dist(rseg, tseg, sr)
        print(f"\nPair REF seg {ri + 1} <-> TEST seg {k + 1}" + (f"  (TEST gain {gain_db:+.2f} dB applied)" if o.match_level else ""))
        report("REF ", r)
        report("TEST", t)
        print(f"  DIST: mean|dH| h1-40 {d_h:.2f} dB   log-mel {d_mel:.2f} dB   LUFS env {d_lu:.2f} LU")
        pairs.append((ri, r, t))
        out.append({"ref_seg": ri + 1, "test_seg": k + 1, "gain_db": gain_db, "ref": r, "test": t, "dist": {"harm_db": d_h, "mel_db": d_mel, "lufs_env_lu": d_lu}})
    if o.plots:
        import os

        stem = os.path.splitext(os.path.basename(o.test))[0]
        for p in plot(pairs, o.plots, stem):
            print("plot", p)
    if o.json:
        with open(o.json, "w") as f:
            json.dump({"ref_bounds": rb, "test_bounds": tb, "pairs": out}, f, indent=1)


def _selfcheck():
    # ponytail: one synthetic check (band-limited saw at C3 vs itself): harmonics follow -20log10(n),
    # tilt ~ -6 dB/oct, distances 0. Run: py -3.12 tools/compare.py --selfcheck
    sr, f0 = 44100, 130.81
    t = np.arange(int(3 * sr)) / sr
    saw = sum(np.sin(2 * np.pi * k * f0 * t) / k for k in range(1, int(sr / 2 / f0) + 1)) * 0.2
    x = np.stack([saw, saw], 1)
    s = analyse(x, sr, 0.0, 3.0, f0)
    assert abs(s["f0"] - f0) < 0.01, s["f0"]
    assert all(abs(s["harm_db"][k - 1] + 20 * np.log10(k)) < 0.2 for k in range(1, 41)), s["harm_db"][:8]
    assert abs(s["tilt"] + 6.02) < 0.1, s["tilt"]
    assert s["nonharm_db"] < -80, s["nonharm_db"]
    assert mel_dist(saw, saw, sr) == 0
    print("selfcheck ok", round(s["tilt"], 3), round(s["nonharm_db"], 1))


if __name__ == "__main__":
    if sys.argv[1:] == ["--selfcheck"]:
        _selfcheck()
    else:
        main()
