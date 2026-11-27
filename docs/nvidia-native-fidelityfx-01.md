# NVIDIA Native FidelityFX 01

Base: the supplied `victory-4060-work.zip`, archive comment `9ad18371e3b9e7935d9eb13744014a077493bab5`. The base README reports an RTX 5070 12 GB / Ryzen 9800X3D, not an RTX 4060 or RTX 3070. Those numbers are upstream's measurements, not measurements of this port. Preserve upstream attribution and license. No title assets or SDK binaries added.

## Why this base

The NVIDIA branch retains the original thin-list replay, submission/retirement, texture registry, and card-step infrastructure. It contains NVIDIA dynamic descriptor-slot sizing, Series S two-plane composition, half-view facial compute fixes, video-plane refreshes, NaN handling and ambient-light tuning. These are untouched by this port. DexNom Combined 01's import-level PC SDK replacement is not used.

The original Xbox loader still creates/configures/queries its context; the native bridge intercepts only eligible dispatches. Native code uses the official 2.4.0 SDK's FSR 3.1.6 provider, actual host resources, and dedicated ordered native command-list segments. The FFX04 resource-lifetime retention, lazy allocation, guarded pre-enqueue fallback with reset, and fence-gated recreation are retained. The guest SDK is still used when a descriptor cannot be admitted. GPU failures after acceptance remain fatal. Fallback and context recreation are not runtime-validated even in the older branch. The synthetic probe remains available separately; do not enable it during the native game test.

## Other changes

- Restore configurable DirectStorage workers, per-worker compressed-input scratch reuse (4 MiB limit), buffered fallback for unsupported unbuffered I/O, retryable overlapped event allocation and no-worker error handling. `VICTORY_DSTORAGE_STAGING_MB` is now an optional floor (0 by default; capped at 1024 MiB). This is an in-flight budget, not preallocated memory.
- Cache publication is best-effort: successful translations are returned in memory even if file creation/write/rename fails. Cache lookup/encoding and atomic publication remain content-addressed. The cache's existing build fingerprint hashes `shader_cache.rs`, so changing that file intentionally invalidates old translations once. Do not copy a different fork's translated shaders or weaken the key. Startup logs cache/data paths, shader directory, fingerprint and host settings; save failures are counted and rate-limited. Directory/config failures before shader translation still return an error.
- Descriptor initialization fast path and opt-in dirty-range batch reads (`VICTORY_HEAP_BATCH_READS=1`, off by default for the first NVIDIA test). Failed bulk reads retry original ranges. NVIDIA-specific descriptor sizing and half views unchanged.
- Per-pipeline wait notification retains the fork's existing per-pipeline wait-once budget. In-place range merging preserves the previous interval-union semantics and avoids a second vector.
- Prior FFX documentation is retained as historical reference; use this document and NVIDIA launcher for this port. Not ported: old `residency.rs`, small-GPU reclamation and buffer-priority policies, card-step scheduling changes, async GPU profiler, Agility override, or DexNom's import-level bridge. These need independent evaluation.

## First launch

Use a normal Windows PowerShell, MSVC/Rust prerequisites from upstream, and a writable cache/data directory outside the source checkout. Keep caches/data across repeats; never use the protected DexNom cache. Compile the diagnostics profile because the NVIDIA fork's RT skip-list requires it.

```powershell
cargo build --locked --profile diag -p victory --features diagnostics
$work = 'D:\VIctory-workspace'  # choose a writable directory outside the repo
$env:VICTORY_CACHE_DIR = Join-Path $work 'nvidia-native-01-cache'
$env:VICTORY_DATA_DIR = Join-Path $work 'nvidia-native-01-data'
$exe = 'D:\path\game_scarlett_master.exe'  # your own title executable
$settings = @{
    VICTORY_CARD_BUFFERS_MB = '768'
    VICTORY_DSTORAGE_WORKERS = '6'
    VICTORY_DSTORAGE_STAGING_MB = '128'
    VICTORY_PIPELINE_WORKERS = '8'
    VICTORY_LOG = 'info'
}
# A: original NVIDIA fork's title upscaler at 720p internal.
.\scripts\play-nvidia.ps1 -Exe $exe -Quality -NoSmoothMotion -Resolution 720 -More $settings
# B: close A normally first. Retain the same cache and data paths.
.\scripts\play-nvidia.ps1 -Exe $exe -Quality -NoSmoothMotion -NativeFSR3 -More $settings
```

The native option sets the PC SDK path (override with `-FfxDir`), keeps `VICTORY_OWN_UPSCALER=0` and removes the render-scale patch. Do not pass `-Resolution` or `-SeriesX` for this first native test. The title previously selected 720p -> 1080p on Series S; confirm in this fork's actual context log. This is a conservative launcher restriction, not a hardcoded SDK resolution. `-Quality` disables the inherited speculative texture updates, relay stage 2 and card steps (known upstream visual problems); keep it for initial validation. `-NoSmoothMotion` avoids an unrelated driver frame-generation experiment. The 768 MiB GPU compare cap is a starting point for the RTX 3070's 8 GB, not a measured optimum. `-More` is applied after launcher defaults; never enable both own-upscaler and native FFX switches. The launcher clears stale VICTORY variables other than configured paths before setting the mode. It is not necessary to reinstall SDK 2.4.0 if already installed.

Expected banner `VIctory revision nvidia-native-fidelityfx-01`. In A, confirm the intro/interface and gameplay. In B, confirm `PC FFX native game context created`, `... accepted`, `... recorded` and `... fence completed`. Report any `PC FFX resource rejected`, `PC FFX recovery` and native SDK errors. Use a matching scene and resolution for performance comparisons. Don't interpret a loading/menu loop as gameplay throughput. Compare actual visible quality, dropped draws, preparation/relay and GPU waits, not just presentation FPS.

## Offline validation

The port was made by three-way merging the FFX-only delta from GPU Execution through FFX04, not replacing the NVIDIA renderer. The host conflict was resolved by retaining NVIDIA device selection and adding only the native probe/retirement logic; omitted earlier GPU-profiler blocks. Shader cache and storage/descriptor/notification changes are independent. Header ABI assertions and Python descriptor/recovery models are portable checks; added Rust unit tests require the Windows build. No Rust toolchain, Windows GPU, D3D12 debug layer, or PowerShell runtime was available in this environment. No claim of successful Windows compilation, gameplay, quality or speed for this build.
