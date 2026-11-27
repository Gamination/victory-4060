# PC Native FidelityFX 02 — native dispatch smoke test and resource mapping

Historical bring-up notes. For the opt-in in-game dispatch in revision 03, see [PC Native FidelityFX 03](pc-native-fidelityfx-03.md).

This revision rechecks PC Native FidelityFX 01 against the user-supplied Xbox DLL, GTA V Enhanced PC DLL, and official SDK 2.4.0 ZIP. The game still executes the Xbox upscaler. This is not a performance release and does not change the shader cache format or game shader bytecode.

## Static binary and header audit

- Xbox upscaler: embedded version string 4.1.1 and RTTI for `ffxProvider_FSR4_Int8` and `ffxProvider_FSR4_Common`. An FSR3 effect name in a shared enum/string table does not establish an actual Xbox FSR3 provider. GTA V Enhanced PC DLL has version string 3.1.4 and FSR3 provider RTTI. The supplied PC SDK upscaler includes version 3.1.6, plus FSR2 and FSR4 providers. The SDK's FSR3 provider source constructs its name as exactly `3.1.6`.
- The five named public exports on Xbox and SDK PC loader have matching ordinals 1–5; this does not establish object ABI compatibility.
- Xbox upscaler disassembly at RVA 0xcd48 checks **0x01000002** for the guest backend extension and reads its device at +0x10 (RVA 0xcdca). The current SDK DX12 backend tag is **0x2**, even though the current SDK also defines a separate Xbox backend namespace. Do not forward the guest backend descriptor chain unchanged. Xbox RVA 0xca85 checks 0x1000b and RVA 0xcaad reads its version at +0x10. Xbox guest API version checking is specific to that binary, and must not be used as the native SDK version.
- The common dispatch layout is 0x1b0 bytes in the current PC headers; its seven resource structures each occupy 48 bytes. An actual host resource descriptor must be rebuilt from native resource metadata, rather than forwarding a guest pointer and assuming its enum/state values.
- `scripts/check-pc-ffx-abi.py` checks the official ZIP's actual C headers with a C11 x64 compiler, including field offsets, surface format and state constants. Rust has independent compile-time layout assertions. The C check was run successfully in the development environment. It does not compile the Rust code.

## External dependency

```powershell
python scripts/install-pc-ffx.py --zip 'C:\path\FidelityFX-SDK-2.4.0.zip'
# Alternatively, explicit official tag download:
python scripts/install-pc-ffx.py --download
```

Default `%LOCALAPPDATA%\VIctory\sdk\ffx-2.4.0`. The installer verifies both pinned SHA-256 hashes and publishes a complete directory. No SDK DLL is bundled. Online download has not been executed here. Only use trusted SDK binaries; runtime loading trusts the chosen directory (hash checks are in the installer). Optional publisher verification on Windows: `Get-AuthenticodeSignature` on each DLL. No network use during cargo build or gameplay. If the loader does not enumerate the expected provider, rerun with `VICTORY_PC_FFX_ENTRY=upscaler` to explicitly test the signed effect DLL’s own five API exports. Default is `loader`; the two sets of entrypoints are never mixed in one context.

## Actual native SDK smoke test

Build diagnostics (separate from the normal release executable):

```powershell
cargo build --profile diag --locked -p victory --features diagnostics
```

Point the current launcher to `target\diag\victory.exe` and add these after environment resets:

```powershell
$env:VICTORY_PC_FFX_DIR = Join-Path $env:LOCALAPPDATA 'VIctory\sdk\ffx-2.4.0'
$env:VICTORY_PC_FFX_PROBE = '1'
$env:VICTORY_PC_FFX_CONTEXT_TEST = '1'
$env:VICTORY_PC_FFX_DISPATCH_TEST = '1'
$env:VICTORY_PC_FFX_TRACE = '1'
$env:VICTORY_LOG = 'info'
```

Look for `VIctory revision pc-native-fidelityfx-02 (diagnostics=true)`. The SDK probe selects the exact reported FSR 3.1.6 provider, checks context provider ID, queries its resource requirements and actual context memory usage, records an initialized 640x360 -> 1280x720 synthetic frame on a private native D3D12 queue, waits for its fence, reads back a center pixel and checks the expected red > green > blue > 0 ordering. The output readback elapsed time is CPU-observed; the separate `PC FFX SDK synthetic dispatch GPU interval` comes from two D3D12 timestamps around the SDK-recorded work. Neither measures actual in-game FPS. This requires a real Windows GPU to test. A failed probe before GPU submission leaves the game path unchanged if the device is still healthy. If native work cannot be retired safely, the probe retains its GPU objects, context and DLLs and refuses to initialize the normal GPU backend. Disable `VICTORY_PC_FFX_DISPATCH_TEST` if this test fails or is not wanted. `VICTORY_PC_FFX_CONTEXT_TEST` remains usable in release builds. Native DLL exceptions cannot be recovered here.

New trace evidence:

- `PC FFX SDK provider`, `PC FFX SDK native context created`, `PC FFX SDK resource requirements`, `PC FFX SDK context memory`, `PC FFX SDK synthetic dispatch recorded`, `PC FFX SDK synthetic output readback`, `PC FFX SDK synthetic dispatch passed`.
- `PC FFX guest Xbox backend extension`, `PC FFX guest API version extension`, `PC FFX guest upscale create layout`, `PC FFX guest upscale dispatch layout candidate`.
- `PC FFX guest command list kind` (thin/direct/unknown), and `PC FFX guest texture mapped` or `no host copy yet` / `not a registered texture` per non-null input/output. Snapshot fields include the actual DXGI format, dimensions, mips, resource flags, alias/tile-related information and last submitted state. Last submitted state is **not** the state at the future dispatch location. The registry inspection neither creates host textures nor changes last-use tracking. Busy locks are reported rather than waited on.

Tracing wraps named static imports and named dynamic lookups of the title loader, not PC SDK exports. Create/destroy always logged; dispatch has its own budget (first 64 and every 256th), independent of unrelated startup calls. Header chains bounded at 12 links and cycle-checked. Resource tuple order remains name, guest pointer, type, format, dimensions, mips, flags, usage, state; fields are candidate guest ABI values. No arbitrary guest pointer crosses into the PC SDK.

## Next actual in-game integration

The synthetic test establishes native dispatch capability only. Use the new trace to verify the live game inputs, whether resources have host copies, whether depth has a colour twin, actual formats/mip coverage, flags and resolution. Then introduce an explicit native upscaling command in the thin-list recording/replay path (not a call against an arbitrary guest list). Map resource lifetime/alias ownership, apply state transitions at replay, restore heap/root binding state after SDK dispatch, retire contexts only after all referenced fences, and preserve the Xbox path as a per-context fallback. The native context must match the guest's actual dimensions and reset/exposure semantics. Do not mix native and guest temporal history mid-context. No frame generation or native lighting replacement in this revision.

For normal release benchmarks disable all probe/trace flags. No performance gain is claimed; Windows compilation and execution still need testing.
