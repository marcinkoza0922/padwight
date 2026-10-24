#!/usr/bin/env python3
"""Measures how pleasant or grating short UI cues are, without having to listen to them.

    python3 -I scripts/sound-audit.py [--out dir] [--level dB] [--sort key] path...

Each path is an audio file (anything ffmpeg reads) or a folder of them. For every cue it prints the
psychoacoustic measures behind "grating" and writes an HTML page (`<out>/index.html`) with a player,
a spectrogram and the measures for each, sortable, to listen and compare (default out:
`sound-previews/audit`, which git ignores).

The measures are simplified forms of the standard ones, each calibrated against its reference signal
when the script starts, so the units mean what they say:

- loud (sone): Zwicker loudness at its loudest moment, the cue played so full scale is 90 dB SPL.
  Cues meant for the same menu should sit close together; one that jumps out is grating however it
  sounds. --level instead plays every cue at the same peak loudness, so timbre alone is compared.
- sharp (acum, DIN 45692): how much of the loudness sits high up. 1 is a 1 kHz band of noise; past
  about 2 cues start to sound shrill.
- rough (asper): fast beating (20-200 Hz) within critical bands, heard as rasp or buzz. 1 is a 1 kHz
  tone fully modulated at 70 Hz; above about 0.2 a short cue sounds harsh.
- harsh (%): share of the loudness in 2-5 kHz, where the ear is most sensitive and where squeals live.
- fizz (%): share of the energy above 8 kHz, the hiss that small speakers and earbuds make spiky.
- attack (ms): how fast the cue reaches its peak. Under 1 ms is a hard click.
- flat: spectral flatness, 0 for a pure tone, 1 for white noise.
- annoy: Zwicker and Fastl's psychoacoustic annoyance from loudness, sharpness and roughness. Lower
  is calmer; it is the default sort.
- grate: the part of annoy that sharpness and roughness add, at a fixed loudness: the timbre alone,
  since a cue's volume can be set. Under 0.3 is smooth; over 1 grates.
"""

import argparse
import html
import json
import shutil
import subprocess
import sys
from pathlib import Path

import numpy as np
from scipy import signal

RATE = 44_100
AUDIO = {".wav", ".ogg", ".mp3", ".flac", ".opus", ".aif", ".aiff"}
# Digital full scale played back at this SPL, so levels in files map to levels at the ear.
FULL_SCALE_DB = 90.0
# Bark band edges (Zwicker's critical bands), Hz.
EDGES = np.array([20, 100, 200, 300, 400, 510, 630, 770, 920, 1080, 1270, 1480, 1720, 2000, 2320, 2700, 3150, 3700, 4400, 5300, 6400, 7700, 9500, 12000, 15500, 20000])
CENTRES = np.sqrt(EDGES[:-1] * EDGES[1:])
BARK = np.arange(len(CENTRES)) + 0.5


def decode(path):
    raw = subprocess.run(["ffmpeg", "-v", "error", "-i", str(path), "-ac", "1", "-ar", str(RATE), "-f", "f32le", "-"], capture_output=True, check=True).stdout
    return np.frombuffer(raw, dtype=np.float32).astype(np.float64)


def threshold_db(f):
    """Terhardt's threshold in quiet, dB SPL: the ear's outer and middle ear in one curve."""
    k = np.asarray(f) / 1000
    return 3.64 * k**-0.8 - 6.5 * np.exp(-0.6 * (k - 3.3) ** 2) + 1e-3 * k**4


THRESHOLD = threshold_db(CENTRES)


def frames(x, size=2048, hop=512):
    if len(x) < size:
        x = np.pad(x, (0, size - len(x)))
    n = 1 + (len(x) - size) // hop
    idx = np.arange(size)[None, :] + hop * np.arange(n)[:, None]
    return x[idx] * np.hanning(size)


def band_levels(x):
    """Each frame's level in each Bark band, dB SPL."""
    spec = np.abs(np.fft.rfft(frames(x), axis=1)) ** 2
    freqs = np.fft.rfftfreq(2048, 1 / RATE)
    # A Hann window's power gain, so a full-scale sine reads 0 dBFS.
    spec *= 2 / (np.sum(np.hanning(2048)) ** 2 * 0.5) * 0.5
    bands = np.stack([spec[:, (freqs >= lo) & (freqs < hi)].sum(axis=1) for lo, hi in zip(EDGES[:-1], EDGES[1:])], axis=1)
    return 10 * np.log10(bands + 1e-20) + FULL_SCALE_DB


def specific_loudness(levels):
    """Zwicker's specific loudness N'(z) per frame and band, with masking spread to the bands above."""
    excitation = levels.copy()
    # Upward spread of masking: about 27 dB per Bark below loud bands, less above.
    for i in range(1, excitation.shape[1]):
        excitation[:, i] = np.maximum(excitation[:, i], excitation[:, i - 1] - 15)
    e = 10 ** ((excitation - THRESHOLD) / 10)
    n = 0.08 * (np.maximum(0.0, (0.5 + 0.5 * e) ** 0.23 - 1))
    return n * LOUDNESS_GAIN


def sharpness(n):
    """DIN 45692 sharpness of one N'(z), acum."""
    g = np.where(BARK < 15.8, 1.0, 0.15 * np.exp(0.42 * (BARK - 15.8)) + 0.85)
    total = n.sum()
    return 0.0 if total <= 0 else SHARP_GAIN * (n * g * BARK).sum() / total


def roughness(x):
    """Beating at 20-200 Hz in each critical band, weighted to peak at 70 Hz like the ear's, and
    counted only as far as it repeats: an onset, or noise, moves the envelope as fast as a beat
    does, but once and at random, and neither sounds rough."""
    env_rate = 2000
    bands = []
    for lo, hi in zip(EDGES[1:-2], EDGES[2:-1]):
        sos = signal.butter(4, [lo, min(hi, RATE / 2 - 100)], btype="band", fs=RATE, output="sos")
        band = signal.sosfilt(sos, x)
        bands.append((band, 10 * np.log10(np.max(np.convolve(band**2, np.ones(441) / 441, mode="same")) + 1e-20) + FULL_SCALE_DB))
    loudest = max(level for _, level in bands)
    total = 0.0
    for band, level in bands:
        # A band well below the loudest is masked by it, and its faint edge (the filter ringing on
        # against a tone just outside) always looks modulated.
        weight = max(0.0, level - max(loudest - 20, 20))
        if weight == 0:
            continue
        env = signal.resample_poly(np.abs(signal.hilbert(band)), env_rate, RATE)
        if len(env) < 64:
            continue
        # The slow shape of the cue (its attack and decay) isn't roughness: take it out.
        slow = signal.sosfiltfilt(signal.butter(2, 15, fs=env_rate, output="sos"), env)
        fast = signal.sosfiltfilt(signal.butter(2, [20, 250], btype="band", fs=env_rate, output="sos"), env)
        loud = slow > 0.1 * slow.max()
        if loud.sum() < env_rate * 0.01:
            continue
        f = fast[loud]
        spec = np.abs(np.fft.rfft(f * np.hanning(len(f)))) ** 2
        fm = np.fft.rfftfreq(len(f), 1 / env_rate)
        curve = np.exp(-0.5 * (np.log2(np.maximum(fm, 1) / 70) / 0.9) ** 2)
        depth = np.sqrt((spec * curve).sum() / max(spec.sum(), 1e-20)) * f.std() / slow[loud].mean()
        ac = np.correlate(f, f, mode="full")[len(f) - 1 :]
        lags = slice(int(0.004 * env_rate), min(int(0.05 * env_rate), len(f) // 2) + 1)
        # A beat brings the envelope back round, so it peaks again a period on; a single bump, however
        # fast, only falls away.
        a = ac[lags.start - 1 : lags.stop + 1] / ac[0] if ac[0] > 0 and lags.start < lags.stop else np.zeros(3)
        peaks = a[1:-1][(a[1:-1] > a[:-2]) & (a[1:-1] >= a[2:])]
        periodic = max(0.0, float(peaks.max())) if len(peaks) else 0.0
        total += depth * weight * periodic
    return ROUGH_GAIN * total / 20


def measure(x, level=None):
    x = x[np.argmax(np.abs(x) > 1e-4) :] if np.any(np.abs(x) > 1e-4) else x
    if level is not None:
        x = x * 10 ** ((level - FULL_SCALE_DB - momentary_dbfs(x)) / 20)
    levels = band_levels(x)
    n = specific_loudness(levels)
    per_frame = n.sum(axis=1)
    # N5: loudness exceeded 5% of the time while the cue sounds, as Zwicker's annoyance asks.
    sounding = per_frame[per_frame > 0.05 * per_frame.max()] if per_frame.max() > 0 else per_frame
    n5 = float(np.percentile(sounding, 95))
    weighted = (n * per_frame[:, None]).sum(axis=0)
    s = sharpness(weighted)
    r = roughness(x)
    harsh = weighted[(CENTRES >= 2000) & (CENTRES <= 5000)].sum() / max(weighted.sum(), 1e-12)
    spec = np.abs(np.fft.rfft(x)) ** 2
    freqs = np.fft.rfftfreq(len(x), 1 / RATE)
    fizz = spec[freqs > 8000].sum() / max(spec.sum(), 1e-20)
    centroid = (spec * freqs).sum() / max(spec.sum(), 1e-20)
    audible = spec[(freqs > 50) & (freqs < 16000)] + 1e-20
    flat = float(np.exp(np.mean(np.log(audible))) / np.mean(audible))
    env = np.abs(signal.hilbert(x))
    env = np.convolve(env, np.ones(44) / 44, mode="same")
    top = int(np.argmax(env))
    attack = (int(np.argmax(env > 0.9 * env[top])) - int(np.argmax(env > 0.1 * env[top]))) / RATE * 1000
    tail = np.nonzero(env > env[top] * 10 ** (-50 / 20))[0]
    duration = (tail[-1] + 1) / RATE * 1000 if len(tail) else 0
    # Zwicker & Fastl psychoacoustic annoyance (fluctuation strength left out: cues are too short for it).
    ws = (s - 1.75) * 0.25 * np.log10(n5 + 10) if s > 1.75 else 0.0
    wr = 2.18 / max(n5, 1e-3) ** 0.4 * 0.6 * r
    annoy = n5 * (1 + np.hypot(ws, wr))
    # The same at a fixed 5 sone: the timbre alone, as a cue's volume can be set to anything.
    grate = np.hypot((s - 1.75) * 0.25 * np.log10(15) if s > 1.75 else 0.0, 2.18 / 5**0.4 * 0.6 * r)
    return {"grate": grate, "loud": n5, "sharp": s, "rough": r, "harsh": harsh * 100, "fizz": fizz * 100, "centroid": centroid, "attack": attack, "ms": duration, "flat": flat, "annoy": annoy}


def momentary_dbfs(x):
    """The loudest 50 ms of the cue, as K-weighted dBFS (BS.1770's filters)."""
    b1, a1 = [1.53512485958697, -2.69169618940638, 1.19839281085285], [1.0, -1.69065929318241, 0.73248077421585]
    b2, a2 = [1.0, -2.0, 1.0], [1.0, -1.99004745483398, 0.99007225036621]
    k = signal.lfilter(b2, a2, signal.lfilter(b1, a1, x))
    win = int(0.05 * RATE)
    power = np.convolve(k**2, np.ones(win) / win, mode="valid") if len(k) > win else [np.mean(k**2)]
    return 10 * np.log10(np.max(power) + 1e-20)


def tone(f, seconds, db):
    t = np.arange(int(RATE * seconds)) / RATE
    return np.sin(2 * np.pi * f * t) * np.sqrt(2) * 10 ** ((db - FULL_SCALE_DB) / 20)


LOUDNESS_GAIN = SHARP_GAIN = ROUGH_GAIN = 1.0


def calibrate():
    """Scales each measure so its reference signal reads 1."""
    global LOUDNESS_GAIN, SHARP_GAIN, ROUGH_GAIN
    LOUDNESS_GAIN = 1 / specific_loudness(band_levels(tone(1000, 0.5, 40)))[2].sum()
    rng = np.random.default_rng(0)
    noise = signal.sosfilt(signal.butter(6, [920, 1080], btype="band", fs=RATE, output="sos"), rng.standard_normal(RATE))
    noise *= 10 ** ((60 - FULL_SCALE_DB) / 20) / np.sqrt(np.mean(noise**2))
    n = specific_loudness(band_levels(noise))
    SHARP_GAIN = 1 / sharpness(n.sum(axis=0))
    t = np.arange(RATE) / RATE
    am = tone(1000, 1, 60) * (1 + np.sin(2 * np.pi * 70 * t)) / np.sqrt(1.5)
    ROUGH_GAIN = 1 / roughness(am)


def spectrogram(x, path):
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    fig, ax = plt.subplots(figsize=(3.2, 1.6), dpi=100)
    nper = 512 if len(x) > 512 else max(64, len(x))
    f, t, s = signal.spectrogram(np.pad(x, (0, max(0, 4096 - len(x)))), RATE, nperseg=nper, noverlap=nper * 3 // 4)
    ax.pcolormesh(t * 1000, f / 1000, 10 * np.log10(s + 1e-12), vmin=-110, vmax=-30, cmap="magma", shading="auto")
    ax.set_ylim(0, 16)
    ax.set_xlim(0, max(100, len(x) / RATE * 1000))
    ax.tick_params(labelsize=6)
    ax.set_xlabel("ms", fontsize=6, labelpad=0)
    ax.set_ylabel("kHz", fontsize=6, labelpad=0)
    fig.tight_layout(pad=0.2)
    fig.savefig(path)
    plt.close(fig)


def gather(paths):
    for p in map(Path, paths):
        if p.is_dir():
            yield from sorted(f for f in p.rglob("*") if f.suffix.lower() in AUDIO)
        elif p.suffix.lower() in AUDIO:
            yield p


COLUMNS = ["annoy", "grate", "loud", "sharp", "rough", "harsh", "fizz", "attack", "ms", "centroid", "flat"]


def write_page(out, rows):
    data = json.dumps(rows)
    page = PAGE.replace("/*DATA*/", data).replace("/*COLUMNS*/", json.dumps(COLUMNS))
    (out / "index.html").write_text(page)


PAGE = """<!doctype html><meta charset=utf-8><title>Sound audit</title>
<style>
:root{--bg:#fafafa;--fg:#222;--line:#ddd;--hi:#fff3c4}
@media (prefers-color-scheme:dark){:root{--bg:#18181b;--fg:#e4e4e7;--line:#3f3f46;--hi:#3f3a1a}}
body{background:var(--bg);color:var(--fg);font:13px system-ui;margin:16px}
table{border-collapse:collapse}td,th{border-bottom:1px solid var(--line);padding:3px 6px;text-align:right}
th{cursor:pointer;position:sticky;top:0;background:var(--bg)}td.name{text-align:left;max-width:260px;overflow-wrap:anywhere}
tr.playing{background:var(--hi)}img{display:block}input{width:300px}
</style>
<p>Click a row to play it (Space plays the selection again, ↑/↓ move and play). Click a column to sort.
<input id=filter placeholder="filter by name"></p>
<table><thead><tr><th>name</th><th>spectrogram</th></tr></thead><tbody></tbody></table>
<script>
const rows=/*DATA*/,cols=/*COLUMNS*/;let key='annoy',dir=1,sel=0,shown=[];const audio=new Audio();
const head=document.querySelector('thead tr');cols.forEach(c=>{const th=document.createElement('th');th.textContent=c;th.onclick=()=>{dir=key==c?-dir:1;key=c;draw()};head.append(th)});
function play(i){sel=i;audio.src=shown[i].file;audio.currentTime=0;audio.play();document.querySelectorAll('tbody tr').forEach((tr,j)=>tr.classList.toggle('playing',j==i))}
function draw(){const f=document.getElementById('filter').value.toLowerCase();shown=rows.filter(r=>r.name.toLowerCase().includes(f)).sort((a,b)=>(a[key]>b[key]?1:-1)*dir);
const body=document.querySelector('tbody');body.innerHTML='';shown.forEach((r,i)=>{const tr=document.createElement('tr');tr.innerHTML=`<td class=name>${r.name}</td><td><img src="${r.png}" width=192></td>`+cols.map(c=>`<td>${r[c].toFixed(c=='centroid'||c=='ms'?0:2)}</td>`).join('');tr.onclick=()=>play(i);body.append(tr)})}
document.getElementById('filter').oninput=draw;
addEventListener('keydown',e=>{if(e.target.tagName=='INPUT')return;if(e.key==' '){e.preventDefault();play(sel)}if(e.key=='ArrowDown'){e.preventDefault();play(Math.min(sel+1,shown.length-1))}if(e.key=='ArrowUp'){e.preventDefault();play(Math.max(sel-1,0))}});
draw();
</script>"""


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("paths", nargs="+")
    ap.add_argument("--out", default="sound-previews/audit")
    ap.add_argument("--level", type=float, help="play every cue at this peak loudness (dB SPL, e.g. 60) to compare timbre alone")
    ap.add_argument("--sort", default="annoy", choices=["name", *COLUMNS])
    ap.add_argument("--no-page", action="store_true", help="print the table only")
    ap.add_argument("--json", help="also write the measures to this file")
    ap.add_argument("--root", help="names are shown relative to this folder")
    args = ap.parse_args()
    calibrate()
    out = Path(args.out)
    files = list(gather(args.paths))
    if not files:
        sys.exit("no audio files given")
    if not args.no_page:
        (out / "media").mkdir(parents=True, exist_ok=True)
    rows = []
    for i, f in enumerate(files):
        try:
            x = decode(f)
        except subprocess.CalledProcessError:
            print(f"skipped {f}: ffmpeg can't read it", file=sys.stderr)
            continue
        if len(x) < 64 or np.abs(x).max() < 1e-5:
            continue
        name = str(f.relative_to(args.root)) if args.root and f.is_relative_to(args.root) else str(f)
        row = {"name": name, **{k: float(v) for k, v in measure(x, args.level).items()}}
        if not args.no_page:
            stem = f"{i:04d}"
            shutil.copy(f, out / "media" / f"{stem}{f.suffix.lower()}")
            spectrogram(x, out / "media" / f"{stem}.png")
            row |= {"file": f"media/{stem}{f.suffix.lower()}", "png": f"media/{stem}.png"}
        rows.append(row)
    rows.sort(key=lambda r: r[args.sort])
    print(f"{'name':<52}" + "".join(f"{c:>9}" for c in COLUMNS))
    for r in rows:
        print(f"{r['name'][-52:]:<52}" + "".join(f"{r[c]:>9.0f}" if c in ("centroid", "ms") else f"{r[c]:>9.2f}" for c in COLUMNS))
    if args.json:
        Path(args.json).write_text(json.dumps(rows, indent=1))
    if not args.no_page:
        write_page(out, rows)
        print(f"\n{out / 'index.html'}", file=sys.stderr)


if __name__ == "__main__":
    main()
