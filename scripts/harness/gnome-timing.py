#!/usr/bin/env python3
"""Summarizes the GNOME overlay's `gnome` timing lines (PADWIGHT_OVERLAY_TIMING): each stage's cost per frame and the gap between frames.
Usage: python3 scripts/harness/gnome-timing.py overlay.log"""
import sys, re, statistics as st
rows=[l.split() for l in open(sys.argv[1]) if l.startswith('gnome ')]
un=sum(1 for l in open(sys.argv[1]) if l.startswith('gnome-unchanged'))
def col(name):
    return [int(r[r.index(name)+1])/1e6 for r in rows]
def show(n,v):
    v=sorted(v); print(f"{n:8} mean {st.mean(v):6.2f}  p50 {v[len(v)//2]:6.2f}  p95 {v[int(len(v)*.95)-1]:6.2f}  max {v[-1]:6.2f} ms")
print(len(rows),'frames sent,',un,'unchanged')
for n in ('monitor','paint','write','call'): show(n,col(n))
tot=[int(r[2])/1e6 for r in rows]; show('total',tot)
ts=[int(r[1]) for r in rows]; gaps=[(b-a)/1e6 for a,b in zip(ts,ts[1:])]
show('gap',gaps)
big=[r for r in rows if int(r[r.index('damage')+2])>0]
px=[int(r[r.index('damage')+2]) for r in rows]; w,h=rows[0][3].split('x'); full=int(w)*int(h)
print('damage as share of screen: mean %.0f%%  p50 %.0f%%' % (100*st.mean(px)/full, 100*sorted(px)[len(px)//2]/full))
