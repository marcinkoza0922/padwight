#!/usr/bin/env python3
"""Synthesizes the made-for-this-app cues in assets/sounds (see the README there).

Each is a short mono 16-bit 44.1 kHz WAV, peak-normalized to -1 dBFS. Run it from the
repository root to rewrite them: python3 scripts/synth-sounds.py [name ...]
"""

import sys
import wave

import numpy as np

RATE = 44_100
rng = np.random.default_rng(7)


def t(seconds):
    return np.arange(int(RATE * seconds)) / RATE


def env(n, attack, decay):
    """An envelope over n samples: a linear attack, then an exponential decay (seconds)."""
    x = np.arange(n) / RATE
    rise = np.clip(x / max(attack, 1e-4), 0, 1)
    return rise * np.exp(-np.maximum(x - attack, 0) / decay)


def bandpass(signal, low, high):
    spectrum = np.fft.rfft(signal)
    freqs = np.fft.rfftfreq(len(signal), 1 / RATE)
    spectrum[(freqs < low) | (freqs > high)] = 0
    return np.fft.irfft(spectrum, len(signal))


def place(total, *parts):
    """Mixes (offset seconds, samples) parts into one buffer `total` seconds long."""
    out = np.zeros(int(RATE * total))
    for at, part in parts:
        i = int(RATE * at)
        n = min(len(part), len(out) - i)
        out[i : i + n] += part[:n]
    return out


def sand():
    """A soft hiss of grains pouring, brightening and fading: an hourglass turned."""
    x = t(0.55)
    grains = rng.standard_normal(len(x)) * (rng.random(len(x)) < 0.25)
    hiss = bandpass(grains, 2500, 9000)
    sweep = np.sin(2 * np.pi * (900 * x + 900 * x**2)) * 0.08
    return (hiss + sweep) * env(len(x), 0.12, 0.18)


def boing():
    """A cartoon spring: a wobbling tone that bends up and settles."""
    x = t(0.4)
    pitch = 180 + 220 * (1 - np.exp(-x / 0.05)) + 30 * np.sin(2 * np.pi * 14 * x) * np.exp(-x / 0.15)
    phase = 2 * np.pi * np.cumsum(pitch) / RATE
    return (np.sin(phase) + 0.3 * np.sin(2 * phase)) * env(len(x), 0.004, 0.12)


def shutter():
    """A camera shutter: two crisp clicks, the second a touch lower."""
    def click(freq):
        x = t(0.03)
        return bandpass(rng.standard_normal(len(x)), freq, freq * 3) * env(len(x), 0.0005, 0.006)

    return place(0.12, (0, click(2500)), (0.055, click(1800) * 0.8))


def zap():
    """An electric zap: a buzzing tone diving down, with crackle."""
    x = t(0.18)
    pitch = 2400 * np.exp(-x / 0.05) + 120
    phase = 2 * np.pi * np.cumsum(pitch) / RATE
    buzz = np.sign(np.sin(phase)) * 0.5 + np.sin(3 * phase) * 0.3
    crackle = bandpass(rng.standard_normal(len(x)), 3000, 12000) * 0.4
    return (buzz + crackle) * env(len(x), 0.002, 0.05)


def purr():
    """A cat's purr: a low, soft rumble pulsing about twenty-five times a second."""
    x = t(0.6)
    rumble = bandpass(rng.standard_normal(len(x)), 40, 400)
    pulse = 0.5 + 0.5 * np.sin(2 * np.pi * 25 * x) ** 2
    shape = np.sin(np.pi * np.clip(x / 0.6, 0, 1)) ** 0.7
    return rumble * pulse * shape + 0.15 * np.sin(2 * np.pi * 50 * x) * pulse * shape


def clank():
    """A machine part landing on steel: a dull thump with a short metal ring."""
    x = t(0.35)
    thump = np.sin(2 * np.pi * 95 * x) * env(len(x), 0.001, 0.03)
    ring = sum(np.sin(2 * np.pi * f * x) * a for f, a in [(523, 0.5), (1347, 0.35), (2211, 0.2), (3120, 0.12)])
    hit = bandpass(rng.standard_normal(len(x)), 1500, 6000) * env(len(x), 0.0005, 0.008)
    return thump + ring * env(len(x), 0.001, 0.07) + hit * 0.6


def card():
    """A playing card flicked down on a table: a papery snap with a short slide."""
    x = t(0.16)
    snap = bandpass(rng.standard_normal(len(x)), 1800, 7000) * env(len(x), 0.0008, 0.012)
    slide = bandpass(rng.standard_normal(len(x)), 3000, 9000) * env(len(x), 0.02, 0.04) * 0.25
    return snap + place(0.16, (0.015, slide))[: len(x)]


SOUNDS = {f.__name__: f for f in [sand, boing, shutter, zap, purr, clank, card]}


def write(name, samples):
    samples = samples / np.max(np.abs(samples)) * 10 ** (-1 / 20)
    fade = min(len(samples), int(RATE * 0.01))
    samples[-fade:] *= np.linspace(1, 0, fade)
    data = (samples * 32767).astype("<i2").tobytes()
    with wave.open(f"assets/sounds/{name}.wav", "wb") as out:
        out.setnchannels(1)
        out.setsampwidth(2)
        out.setframerate(RATE)
        out.writeframes(data)


if __name__ == "__main__":
    for name in sys.argv[1:] or SOUNDS:
        write(name, SOUNDS[name]())
        print("wrote", name)
