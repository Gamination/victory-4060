#!/usr/bin/env python3
"""Portable model of tiles.rs resident_range; does not execute Rust or D3D12."""
import random

MAX = (1 << 32) - 1

def selected(mask, first, count, levels):
    if not 0 < levels <= 16 or first >= levels or count == 0:
        return None
    end = min(first + count, levels)
    below = lambda n: (1 << n) - 1
    valid = mask & below(levels)
    requested = valid & (below(end) & ~below(first))
    if not requested:
        coarser = valid & ~below(end)
        return ((coarser & -coarser).bit_length() - 1, 1) if coarser else None
    last = requested.bit_length() - 1
    start = last
    while start > first and requested & (1 << (start - 1)):
        start -= 1
    return start, last - start + 1

def reference(mask, first, count, levels):
    if not 0 < levels <= 16 or first >= levels or count == 0:
        return None
    end = min(first + count, levels)
    valid = [i for i in range(levels) if mask & (1 << i)]
    runs = []
    for i in valid:
        if first <= i < end:
            if runs and i == runs[-1][-1] + 1:
                runs[-1].append(i)
            else:
                runs.append([i])
    if runs:
        return runs[-1][0], len(runs[-1])
    return next(((i, 1) for i in valid if i >= end), None)

def check(mask, first, count, levels):
    got = selected(mask, first, count, levels)
    assert got == reference(mask, first, count, levels), (mask, first, count, levels, got)
    if got:
        start, length = got
        assert first <= start < start + length <= levels
        assert all(mask & (1 << i) for i in range(start, start + length))

cases = 0
for levels in range(1, 9):
    for mask in range(1 << levels):
        for first in range(levels + 1):
            for count in (*range(levels + 2), MAX):
                check(mask, first, count, levels)
                cases += 1
rng = random.Random(4060)
for _ in range(20000):
    levels = rng.randint(0, 17)
    check(rng.getrandbits(32), rng.randint(0, 18), rng.choice([MAX, rng.randint(0, 18)]), levels)
    cases += 1
print(f'{cases:,} residency range model cases passed (not Rust/GPU execution)')
