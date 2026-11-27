# PC Native FidelityFX 03 — experimental live FSR 3.1.6 dispatch

Based on 02 and its successful native smoke test (0.223232 ms for 640×360 -> 1280×720, pixel [127,64,32,255]). The user's actual guest trace shows a 0x29 context, thin lists, 1280×720 -> 1920×1080, RGBA16F color/output, RG16F motion, R32G8X24_TYPELESS depth, no optional textures, reversed near/far (10000/0.15), view-space metres factor zero. These are observations, not fixed renderer resolutions. Native resource eligibility is checked on every interception.

## Run

Reuse the pinned SDK installed by `scripts/install-pc-ffx.py` (no SDK binaries bundled). Build `cargo build --release --locked -p victory`, or `cargo build --profile diag --locked -p victory --features diagnostics` for timing. After the launcher clears its variables:

```powershell
$env:VICTORY_PC_FFX_DIR = Join-Path $env:LOCALAPPDATA 'VIctory\sdk\ffx-2.4.0'
$env:VICTORY_PC_FFX_NATIVE = '1'
$env:VICTORY_PC_FFX_ENTRY = 'loader'
$env:VICTORY_PC_FFX_TRACE = '1'
$env:VICTORY_PC_FFX_PROBE = '0'
$env:VICTORY_PC_FFX_CONTEXT_TEST = '0'
$env:VICTORY_PC_FFX_DISPATCH_TEST = '0'
$env:VICTORY_LOG = 'info'
```

Keep the same prepared shader cache and thin/async submission settings. For the diagnostic executable, optionally set `VICTORY_PC_GPU_PROFILE=1` (do not turn on the older blocking GPU profiler). `VICTORY_PC_FFX_NATIVE=0` restores the original path on a fresh process. Do not toggle mid-run. The old probe is still usable separately with native replacement disabled. Revision banner: `pc-native-fidelityfx-03`.

## Implementation

1. The loader records successful guest upscale context creation and a generation number; it recognizes the observed Xbox backend/API extension chain. Only the opt-in, known upscale dispatch is eligible. Other effects remain guest calls. A successful enqueue replaces the guest dispatch call; the original guest context is still created and queried. Neither guest COM pointers nor raw guest descriptors are passed into the PC DLL.
2. Validate finite temporal values, context limits, flag set, required handles, native format, dimensions, no aliases and no optional textures. Validate native texture copies are GPU-owned, non-tiled, not streamed, no depth twin, no placed-view copy, and reject CPU-readback output. Holds Arc references across recording, submit and completion. The depth resource uses the SDK depth/stencil usage and R32_FLOAT surface format, backed by its actual R32G8X24 native texture. Its DX12 backend fixes the SRV to R32_FLOAT_X8X24_TYPELESS. Output uses the actual RGBA16F native format, not the guest packed format. Preserve jitter, motion scale, reversed-depth camera parameters, exposure and delta; the observed zero metres conversion factor is interpreted as 1.0 (assumption to validate).
3. Record a dedicated thin-stream operation with an owned job. During replay cut the list before and after a native segment, preserving the existing guest draw state in the tail. No native SDK commands are recorded on arbitrary worker threads. The state prelude registers input compute-read and output UAV states in normal submission order. Global UAV barriers bracket the SDK recording.
4. Reuse one verified SDK context across submitted native dispatches. The native segment is initially closed without effect commands; under the submission lock it is reopened and filled immediately before the normal state prelude and submit. Prior flights are settled and the host fence checked before reusing context history/descriptors. The first submitted frame resets history; skipped sequence numbers also reset. Out-of-order or duplicate execution and multiple native segments in a submission are rejected. This conservative synchronization can limit performance. No extra frame generation, lower-resolution forcing, or rendering-pass skipping outside the intercepted effect.
5. Fallback is permitted before native activation, e.g. early resources not yet made or SDK initialization failure. Once native history starts, incompatible tracked dispatches or SDK errors stop the run (unknown active descriptor returns an API error); there is no transparent GPU-history transfer back to the Xbox provider. Restart with native disabled. The initial bridge keeps one context and its modules alive until process exit; a new context generation or changed declared limits requires restart. Changes in dispatch dimensions *within* those limits are supported by the validation path; actual SDK behavior is untested. Other resource formats, masks and context configuration translation are follow-up work.

## Evidence to collect

- `PC FFX native game context created` (real context limits and memory).
- `PC FFX native game dispatch accepted` (guest call replaced in recording).
- `PC FFX native game dispatch recorded` (SDK returned success at submit, actual render/output/reset).
- `PC FFX native game dispatch fence completed` (host completion path, not pixel correctness).
- `PC FFX guest fallback before native activation` (reason).
- Diagnostic `native FFX FSR 3.1.6` command timing and the presence/absence of the previous nine Xbox FSR4 hashes.

First test for a visible, non-black, stable frame. Capture a screenshot and log, then compare warm release intervals against 02 with native disabled. Native and Xbox shader caches are not interchangeable; the existing game shader cache format is unchanged. No runtime speedup or visual parity is claimed. C SDK header ABI checks and Python models are offline checks; Windows Rust compilation and real GPU/game execution were not possible in the development environment.

For the resource-recovery revision, see [PC Native FidelityFX 04](pc-native-fidelityfx-04.md).
