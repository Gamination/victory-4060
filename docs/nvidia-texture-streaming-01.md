# NVIDIA Texture Streaming 01 — hybrid mip residency and diagnostics

Base: NVIDIA Native FidelityFX 01, unchanged SDK interception, shader translator,
DirectStorage and launcher. This revision is a standalone candidate; use the
previous executable while it is being evaluated. No shader/asset payloads added.

## Changes

- Keep `VICTORY_HELD_LEVELS` enabled independently of `VICTORY_LEVELS_ON_DEMAND`.
  When a placed sampled texture cannot be made reserved (size, layout, allocation,
  or other eligibility checks), it can use the existing reduced host allocation
  if eligible. Sparse copies always keep base zero. Exclude known GPU aliases and
  multi-plane layouts from new sparse allocations. Render targets, UAVs, depth
  and native FFX GPU-owned resources are not sparse candidates.
- Keep full placement/notification spans in sparse mode, including the hybrid
  fallback. This deliberately sacrifices some potential CPU savings to avoid
  missing finer mip arrivals before backing type is known. Force full staging
  chains on actual sparse copies and pin the copy named by each upload's tile
  mapping step. Non-sparse uploads retain the original reduced-level behavior.
- Publish a complete atomic bitmask of backed valid mips instead of just the
  finest available index. An SRV uses the coarsest contiguous run inside its
  requested interval; if none is present, use a single available coarser mip
  (never a finer mip outside the interval). If none is usable, publish a typed
  null while retaining the backing object in the descriptor's lifetime tracking.
  Remake tracked placed SRVs when any validity bit changes, even with no change
  in the finest mip. Preserve the existing view-relative min-LOD adjustment.
- Failed sparse detiles zero their staging region and do not enter the title-held
  mask. A full-chain upload overwrites previously GPU-written levels; their old
  card-written validity bits are cleared rather than advertising overwritten data.
  Existing allocation ownership and packed-tail rules are retained.
- Report cumulative allocation attempts (sparse, reduced, full), total leading
  mips omitted by reduced copies, visibility-mask changes, gapped/empty masks,
  successfully detiled and failed sparse mips. Existing tile byte, heap, wait,
  mapping cost and promotion counters are retained. The new counters are not
  unique asset counts or per-frame latency measurements.

## First tests

Compile exactly as NVIDIA Native FidelityFX 01:

```powershell
cargo build --locked --profile diag -p victory --features diagnostics
```

Keep the same writable cache and runtime data paths and the same upscaler mode
that passed the current test. See `docs/nvidia-native-fidelityfx-01.md` for launcher
syntax. `-More` is applied after launcher defaults; add settings to the existing
hashtable, not before calling a launcher that clears VICTORY variables.

```powershell
# A: baseline (new telemetry, no sparse allocation)
$settings['VICTORY_HELD_LEVELS'] = '1'
$settings['VICTORY_LEVELS_ON_DEMAND'] = '0'
# launch with your existing -More $settings, close normally afterwards
# B: hybrid sparse experiment, same scene and all other settings unchanged
$settings['VICTORY_LEVELS_ON_DEMAND'] = '1'
$settings['VICTORY_LEVELS_MIN_KB'] = '1024'
# launch again with the same -More $settings
```

Expected banner: `VIctory revision nvidia-texture-streaming-01`.
The shader cache source and translator are not changed; existing fingerprint
validation still determines compatibility. Do not copy caches between forks.
Compare `texture streaming: cumulative allocation attempts and mip events`,
`textures with levels on demand`, the existing texture queue and memory reports,
frame time, GPU wait, native FFX dispatches and visible texture pop-in. Take a
matching screenshot if there is a new blur/black texture or UI regression.

## Validation and limits

`python scripts/check-texture-residency.py` checks the range-selection model
against an independently enumerated oracle. Actual Rust tests include explicit
hole/empty/range cases, 2D and array SRV adjustment, and existing sparse GPU
integration tests updated to use the new range logic. On Windows run:

```powershell
cargo test --locked --profile diag -p victory-gpu --features diagnostics resident_range_keeps_only_valid_contiguous_levels
cargo test --locked --profile diag -p victory-gpu --features diagnostics show_resident_clamps_2d_and_2d_array_and_adjusts_min_lod
$env:VICTORY_TILES_TEST_CARD = '1'
cargo test --locked --profile diag -p victory-gpu --features diagnostics levels_on_demand_on_the_card -- --ignored --nocapture
```

No Windows toolchain or GPU was available for this port. In particular, mask
publication follows the existing submission ordering; actual GPU execution,
packed-tail correctness, dynamic/table SRVs (which may need a later recording to
see newly available mips), delayed descriptors and partial source reads need
runtime validation. A valid bit is not proof of correct guest texel content.
No live tile eviction, fence-safe individual tile reclamation, incremental mip
upload, sampler feedback, automatic LOD bias or adaptive quality is implemented.
Sparse uploads still record the whole staging chain and are not upload-ahead
eligible. Packed tail allocations and old mapped tiles are kept until the copy
is retired. The new diagnostics do not prove VRAM savings or visual correctness.
Do not enable sparse mode by default until the A/B results are available.
