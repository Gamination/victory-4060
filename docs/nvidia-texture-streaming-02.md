# NVIDIA Texture Streaming 02 — sparse upload and detile reuse

Base: the exact NVIDIA Texture Streaming 01 source archive. The latest run
reported 9,566 sparse allocation attempts, 344,519 successful mip detiles,
6,141 retries and 7,262 uploads leaving held levels without tiles. Its texture
queue reported 126,921 MB cumulatively; source inspection shows this counter
counts scheduled title source spans (`draw.rs` / `texture::staged`), not actual
PCIe traffic. Do not interpret that number as hardware transfer telemetry.

## Changes

* Reuse an existing sparse host copy for indexed texture requests when its
  last upload is fully backed and a complete coarse suffix, the current
  descriptor judgement, held mask and fully readable source prefix match, and
  the same staging generation is still current. No skip on remapping lock
  contention, missing tiles, failed detiles, holes, finer requests not covered,
  host-written copies, changed source, or outstanding recorded uploads.
* The exact sparse host copy has a ticket held by the recorded upload's
  deferred closure until the list is reset/dropped. This relies on the existing
  host-list retirement rule (reset only after all of its submissions finish).
  A generation shared with the staging allocation invalidates a published
  snapshot on later detiling, including another refresh in the same submission.
  A card write or whole-chain upload clears the previous snapshot. No new fence
  wait or SDK resource path is introduced. Existing title-memory read and
  noncryptographic 128-bit hash semantics are used; hash equality is not a
  cryptographic guarantee. Reuse applies to the indexed-texture path only.
* On persistent sparse table staging allocations, hash a mip's source region
  before detiling; reuse the existing linear bytes if the stamp matches. Tail
  mips hash the whole shared tail, with a single tail hash per refresh. Never
  reuse stamps across staging allocations. A mip that becomes unheld is zeroed;
  repeated known-zero fills are omitted. Failed detiles clear their stamp and
  validity bit. One-shot indexed staging allocations avoid this extra per-mip
  hash work. Existing non-sparse, video, RT, depth and FFX paths remain unchanged.
* When a copy is required, **record the entire sparse chain as before**: final
  tile allocation happens at submission time, after command recording, so a
  record-time partial copy mask could expose uninitialized newly mapped tiles.
  This revision skips whole unchanged uploads and incremental CPU detiling,
  not individual GPU copy commands within necessary uploads. No live mip
  eviction, adaptive LOD, pressure-policy change or pool trimming in this build.
* Add separate cumulative counts of skipped uploads, logical full-chain source
  bytes avoided, prefix bytes checked, reused detiles and their linear footprint
  bytes, skipped zero fills, sparse copy calls recorded and recorded footprint
  bytes. These last two count recording, not executed copies or hardware bytes.
  `detiled_mips` now counts actual successful detile calls, not cached stamps.

## Test

Use the same diag build, same native FSR3 settings and cache/data as Streaming 01.
The shader translator and every file included in its build fingerprint are
unchanged. Set these via the NVIDIA launcher's `-More $settings`:

```powershell
cargo build --locked --profile diag -p victory --features diagnostics
$settings['VICTORY_HELD_LEVELS'] = '1'
$settings['VICTORY_LEVELS_ON_DEMAND'] = '1'
$settings['VICTORY_LEVELS_MIN_KB'] = '1024'
$settings['VICTORY_SPARSE_REUSE'] = '1'
$settings['VICTORY_SPARSE_DETILE_REUSE'] = '1'
```

Expected banner: `VIctory revision nvidia-texture-streaming-02`.
Both reuse switches default on for eligible sparse textures; sparse mode itself
is still opt-in. Set both reuse switches to `0` to disable the new optimizations
without disabling Streaming 01's hybrid residency. To isolate changes, enable
one at a time after a rollback test if necessary. `VICTORY_TEXTURE_SAME=0`
continues disabling the unchanged-upload check for both texture paths.

Compare the new `texture streaming: sparse upload reuse` lines alongside
original streaming counters at matching frames/scenes, shader cache hit rate,
frame times, private commitment, physical working set, available RAM, local and
shared GPU memory and visible assets. A low skip count is possible when lists
remain recorded for a long time. Prefix hashing can also outweigh the savings
in some scenes: measure the total time, not just skip counts. Keep previous
working binary if visual regressions appear. Memory goal remains 16 GB private
commitment, then 18 GB combined provisional accounting; no guaranteed saving.

## Validation and limitations

Portable models: `python scripts/check-sparse-upload-reuse.py` and
`python scripts/check-texture-residency.py`. The added Rust unit tests check
complete suffix detection, source-range coverage for addressable 2D layouts,
and staging generation invalidation. Run on Windows:

```powershell
cargo test --locked --profile diag -p victory-gpu --features diagnostics sparse_reuse_tests -- --nocapture
```

The Rust compiler and Windows D3D12 GPU were not available during this edit.
Source review and Python models do not validate actual GPU timing, D3D12
ordering, Rust compilation, image correctness or memory savings. Continue
using opt-in sparse mode and controlled tests. Future work: submission-time
per-mip copy recording with explicit lifetime/fence ordering, empty texture
pool block reclamation and system-memory/command-list retention.
