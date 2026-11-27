# PC Native FidelityFX 04 — resource recovery

Based on the tested 03 build. The full 03 log ended at frame 4286 with `PC FFX native stream became incompatible: resource 0 has no eligible committed GPU copy`. The last sampled native completion was 2040; the log did not distinguish a missing copy from the other rejection conditions. This is a guarded termination, not a confirmed SDK fault. The original source did not use the normal lazy allocator during FFX interception.

## Changes

* Look up live texture objects under the registry lock and retain a nonzero reference before releasing that lock. Never resurrect an object in destruction. For eligible GPU-owned textures, call the same `hosted(this,true)` allocator and lock that other GPU commands use. Keep the existing Arc-based ownership of the native copy; do not make a separate scratch image or copy stale title memory. The lazy path is optional (`VICTORY_PC_FFX_LAZY=0` disables only missing-copy allocation). This does not guarantee that the producer has recorded a valid write; existing command order is still required.
* Report resource index, role, address, exact rejection reason and available metadata. Keep restrictions on title-filled/streamed textures, placed descriptor views, tiles, nonzero base mip, depth twins, allocation refusals, formats, extents and CPU readback. Guest heap-placed render resources already eligible in 03 are not the same as the `texture_at` placed descriptor views; do not relax the latter.
* Pre-enqueue refusal no longer calls `host::fail`. For a known extension-free guest upscale dispatch (type 0x10001, zero flags, valid reset byte), the loader can run the original Xbox call. If its previous accepted backend was native, pass an aligned copy of the guest descriptor with only reset byte 0x198 set. The supplied Xbox upscaler disassembly consumes this byte at 0x1800121ac; the PC SDK header defines the same field as camera discontinuity/reset. Resume native with reset on the next compatible call. Track last successful backend by guest context generation, not a single global active generation. No transfer or blending of temporal histories. Reset behavior of the original Xbox provider after an extended inactive period still requires runtime validation. Unrecognized dispatch layouts after native activation and errors after native enqueue remain guarded errors; no double execution or blind retry. `VICTORY_PC_FFX_RECOVER=0` disables the new native-to-guest recovery (returns an API error for a declined active dispatch).
* Context recreation on changed generation/limits is admitted only when accepted and completed native counts match and the old context has recorded its last accepted job (or has recorded none). Reuse the already loaded SDK exports, destroy the retired context before creating the next one; if pending, decline the new dispatch before enqueue. Reset the newly created native history. SDK create/destroy failures disable further native admission; retain failed live contexts/modules rather than unload them. No forced drain from the title recording thread.
* Existing native FSR3.1.6 command segmentation, resource formats and queue synchronization retained. No shader-cache format change and no changes to resolution or quality settings.

## Test

Same SDK installation, prepared shader cache and release build as 03. After launcher resets:

```powershell
$env:VICTORY_PC_FFX_NATIVE = '1'
$env:VICTORY_PC_FFX_LAZY = '1'
$env:VICTORY_PC_FFX_RECOVER = '1'
$env:VICTORY_PC_FFX_TRACE = '1'
$env:VICTORY_LOG = 'info'
```

Keep the SDK directory and loader entrypoint from 03. The banner is `pc-native-fidelityfx-04`. Try the previous failure scene and transitions. Look for `PC FFX resolving lazy GPU-owned texture`, `PC FFX lazy GPU-owned texture resolved`, `PC FFX resource rejected`, `PC FFX native dispatch declined before enqueue`, `PC FFX recovery: native to guest; reset history`, `PC FFX recovery guest dispatch result`, `PC FFX recovery: guest to native; reset history`, and `PC FFX retired context for recreation`. Short Xbox fallback periods may cause stutter or transient image changes. If a repeat failure, send the full log and note any visible regression. Restart with native disabled to return to the original path.

Offline validation: SDK ABI header check, Python eligibility and recovery models, previous dependency and parser regression scripts, patch application and ZIP integrity. Rust Windows tests and native in-game execution must be performed on the user's PC. This revision is not a guarantee of scene transition stability; actual root rejection condition remains unknown until new logging is exercised.
