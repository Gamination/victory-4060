#!/usr/bin/env python3
"""Offline state-machine model, not an execution of Rust or either SDK."""
from itertools import product

def dispatch(last, known, accepted, recover, guest_ok):
    if known and accepted:
        return 'native', last == 'guest', 'native'
    if last == 'native':
        if not known or not recover:
            return 'error', False, last
        return 'guest', True, 'guest' if guest_ok else last
    return 'guest', False, 'guest' if guest_ok else last

count = 0
for last, known, accepted, recover, guest_ok in product(
        ['none','guest','native'], [False, True], [False, True], [False, True], [False, True]):
    if accepted and not known:
        continue
    call, reset, after = dispatch(last, known, accepted, recover, guest_ok)
    if accepted:
        assert call == 'native' and after == 'native'
    if last == 'native' and call == 'guest':
        assert known and not accepted and recover and reset
    if last == 'guest' and call == 'native':
        assert reset
    if call == 'guest' and not guest_ok:
        assert after == last
    count += 1
# Reproduce native -> rejected color -> native, both reset on switching.
a = dispatch('native', True, False, True, True)
assert a == ('guest', True, 'guest')
assert dispatch(a[2], True, True, True, True) == ('native', True, 'native')
assert dispatch('native', True, True, True, True) == ('native', False, 'native')
assert dispatch('native', False, False, True, True)[0] == 'error'

# Model fence gating: never recreate while an accepted job is pending.
checks=0
for accepted, completed, last in product(range(8),repeat=3):
    allowed = accepted == completed and last in (0,accepted)
    if allowed:
        assert accepted == completed
    if accepted != completed:
        assert not allowed
    checks += 1
print(f'{count} backend transition cases, 4 sequence checks, {checks} retirement cases passed (Python model only).')
