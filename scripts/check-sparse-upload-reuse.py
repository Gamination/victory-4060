#!/usr/bin/env python3
"""Independent portable model of Streaming 02 reuse gates and staging stamps.
Does not execute the Rust, D3D12, GPU fences or source layout implementation.
"""
import itertools
import random


def suffix_first(mask, levels):
    if not 1 <= levels <= 16 or not mask:
        return None
    first = (mask & -mask).bit_length() - 1
    return first if first < levels and mask == ((1 << levels) - 1) & (-1 << first) else None


cases = 0
for levels in range(1, 17):
    for mask in range(1 << levels):
        bits = [i for i in range(levels) if mask & (1 << i)]
        expected = bits[0] if bits and bits == list(range(bits[0], levels)) else None
        assert suffix_first(mask, levels) == expected
        cases += 1
    assert suffix_first(1 << levels, levels) is None
print(f'{cases:,} exhaustive suffix cases passed')

# Gate inputs: complete tile backing, same snapshot generation and identity,
# no recorded upload left, no card writes, successful whole prefix read,
# same bytes, same judgement, same current mask, eligible requested detail.
# Exactly one combination of these ten booleans is allowed.
for gates in itertools.product((False, True), repeat=10):
    (visible, version, no_pending, no_card, read_complete, same_bytes,
     same_judgement, same_held, same_snapshot, view_covered) = gates
    can_reuse = (visible and version and no_pending and no_card and read_complete
                 and same_bytes and same_judgement and same_held and same_snapshot and view_covered)
    assert can_reuse == (sum(gates) == len(gates))
print('1,024 exhaustive reuse gate combinations passed')

# Compare the per-mip stamp policy to always detiling and zeroing. The
# oracle has independently retained staging bytes; source bytes stand in
# for a detiled mip. Shared tail bytes invalidate every affected mip.
rng = random.Random(406002)
updates = skipped = 0
for _ in range(5000):
    levels = rng.randint(1, 16)
    tail = rng.randint(0, levels)
    source = [rng.randbytes(16) for _ in range(levels)]
    stamps = [None] * levels
    staged = [b'\xff' * 16 for _ in range(levels)]
    for _ in range(20):
        if rng.randrange(8) == 0: # new staging allocation: no old stamps
            stamps = [None] * levels
            staged = [b'\xff' * 16 for _ in range(levels)]
        for i in range(levels):
            if rng.randrange(6) == 0:
                source[i] = rng.randbytes(16)
        mask = rng.randrange(1 << levels)
        failures = rng.randrange(1 << levels) if rng.randrange(12) == 0 else 0
        expected = [bytes(16)] * levels
        for i in range(levels):
            held = bool(mask & (1 << i))
            stamp = ('data', source[i] if i < tail else tuple(source[tail:])) if held else ('zero',)
            if held:
                expected[i] = source[i]
            # A matching stamp skips detiling; an artificial failure only
            # applies to a detile that actually runs.
            if stamps[i] == stamp:
                skipped += 1
            elif held and failures & (1 << i):
                staged[i] = expected[i] = bytes(16)
                stamps[i] = None
            else:
                staged[i] = expected[i]
                stamps[i] = stamp
            updates += 1
        assert staged == expected
print(f'{updates:,} randomized mip transitions passed ({skipped:,} matching stamps)')

# Retirement / generation examples, including a recorded list discarded
# without execution, a later refresh, and an allocation refusal.
class Copy:
    def __init__(self):
        self.pending = 0
        self.version = 0
        self.snapshot = None
        self.visible = 0
        self.card = False
    def record(self): self.pending += 1
    def retire(self):
        assert self.pending > 0
        self.pending -= 1
    def refresh(self): self.version += 1
    def upload(self, held, visible):
        self.visible = visible
        self.card = False
        self.snapshot = (self.version, held) if held == visible and held else None
    def reuse(self):
        return self.pending == 0 and not self.card and self.snapshot == (self.version, self.visible) and bool(self.visible)

c = Copy(); c.record(); c.refresh(); c.upload(14, 14)
assert not c.reuse(); c.retire(); assert c.reuse()
c.record(); assert not c.reuse(); c.retire(); assert c.reuse()
c.refresh(); assert not c.reuse()
c.record(); c.upload(14, 12); c.retire(); assert not c.reuse()
c.record(); c.upload(14, 14); c.retire(); assert c.reuse()
c.card = True; assert not c.reuse()
print('Retirement, mutation, discarded list, partial backing and card-write examples passed')
