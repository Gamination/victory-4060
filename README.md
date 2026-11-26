# VIctory for NVIDIA and AMD

This fork adds dedicated GPU memory fixes to VIctory for NVIDIA and AMD.
The memory changes were checked on an AMD Radeon RX 7600, including texture
uploads and GPU readback, and on Windows' software renderer. Gameplay FPS and
total RAM or VRAM savings from these changes still need measurement.

The NVIDIA gameplay results below come from the earlier fork's reported runs.
They predate these memory changes. NVIDIA Smooth Motion applies to the NVIDIA
launcher; AMD users can use the existing AMD launcher.

## Dedicated GPU memory changes

- Page-aligned graphics allocations use demand-zero, write-watched system
  memory at console-addressable addresses. Creating a 16 MiB allocation in
  the allocation test touched zero pages.
- Sampled textures retain CPU-writable console storage and a GPU-local host
  texture. CPU writes are detiled into upload buffers and copied with
  Direct3D 12 texture copies and resource transitions.
- On supported discrete adapters, up to two 64 MiB texture staging blocks can
  use GPU upload heaps while at least 20% of the reported local VRAM budget
  remains free. System-RAM upload buffers handle the fallback.
- CPU-visible buffer heaps use one mapped host mirror unless the explicit
  two-sided synchronization override is enabled.
- The periodic sweep releases returned staging blocks after their submissions
  finish and the reuse delay passes. Empty staging retention is capped at
  128 MiB and falls to zero below 2 GiB of available physical RAM.
- Persistent CPU mappings of supported sampled DEFAULT textures are refreshed
  for the next submission. Pending lists retain separate staging storage
  when needed.

These changes adapt console memory behavior to separate PC memory pools.
The title still receives the stable CPU addresses it expects. Live resources,
pending work, and the title's own allocations can exceed the cache limits.

Validation covered 42 targeted tests across resources, allocation translation,
physical page aliases, staging reclamation, texture detiling, and GPU readback.
Three successive writes through one persistent texture mapping were read back
correctly, including writes made after command recording. Mixed CPU/GPU writes
to the same texture and unsupported Xbox swizzle modes remain limitations.
RTX 4060 gameplay performance has yet to be measured.

## Built from many projects

This repository builds on work from several VIctory projects. The NVIDIA
fork's authors combined and measured those projects on their NVIDIA machine.
Their credits, reported performance, and explanations are retained below.
This fork adds the dedicated GPU memory changes described above.

Code that is in this repository:

- [avatarmanz/VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX):
  Its threading, write relay, card fills, copy queue and texture
  read-back are all kept.
- [Marvisak/VIctory](https://github.com/Marvisak/VIctory): the original
  compatibility layer under everything else (loader, runtime services,
  Direct3D 12 back end, shader recompiler).
- [fxceless/victory-fork](https://github.com/fxceless/victory-fork) (formerly
  victory-gta6): the fork the 7900 XTX one grew from, the Series S interface
  plane (pull request 1189, Tissma95), which fixed the black menus, and the face
  network fix (pull request 927, tippythetaidum), which made faces move.
- victory-perf, carrying patches from
  [ZigZagBoi1/ScarlettPort](https://github.com/ZigZagBoi1/ScarlettPort): the
  title patches for render scale, the title's own upscaler and the watchdog.
- Through the 7900 XTX fork:
  [DexNom/VIctory](https://github.com/DexNom/VIctory),
  [Belobug/VIctory-Belobug-AMD](https://github.com/Belobug/VIctory-Belobug-AMD),
  [FQY55kai/ZJOU-VICTORY](https://github.com/FQY55kai/ZJOU-VICTORY),
  [myokex/victory-amd](https://github.com/myokex/victory-amd), Geckomacabre and
  cxy0072019-sketch.

Projects we ran, tested or studied along the way, whose lessons shaped the
settings and fixes here (none of their code is in this repository unless listed
above):

- ProjectLeonida/Leonida (no longer public): the first attempt, where the
  retail executable's boot was worked through.
- Our own builds of Marvisak/VIctory and fxceless/victory-fork, where the
  ray-walk skip, the half render scale and most of the measurements in the
  table below came from.
- [Belobug/VIctory-Belobug-AMD](https://github.com/Belobug/VIctory-Belobug-AMD)
  builds with its fixes combined.
- [zm0kiemedia/Neptune_VI](https://github.com/zm0kiemedia/Neptune_VI) (formerly
  vicecitybaby).
- [xcl-dev/xcl](https://github.com/xcl-dev/xcl), another layer for the same
  title, read for how the title behaves there.

Full credits, with what each person contributed, are at the end of this README.

## What this fork changes, and how it gets its frame rate

Everything below is what `scripts\play-nvidia.ps1` turns on. Measured on an
RTX 5070 12 GB with a Ryzen 7 9800X3D, in the prologue:

| Setup | fps | GPU time a frame |
|---|---|---|
| The 7900 XTX fork's own settings, Series X | high, but thousands of draws a frame skipped (the picture jumps) | - |
| With the descriptor fix, Series X, full scale | 6.0 | 110 ms |
| The same, half render scale | 7.2 | 82-88 ms |
| Series S, half scale, own upscaler, ray walks skipped | 11.0 | 33 ms |
| Plus memory compares on the card | 21.8 | about 6 ms |

### Settings that lower the picture a bit

1. **Series S profile** (`--console series-s`). The title believes it runs on
   the smaller console and picks that console's settings. The biggest change
   is resolution: the game image is 1920x1080 instead of the Series X's
   4K-class target. Menus, the HUD and subtitles are still drawn at 4K on a
   separate plane, so they stay sharp.
2. **Half render scale** (`VICTORY_RENDER_SCALE=0.5`). A patch to the title's
   1.0.2.1 executable pins its internal render scale: the 3D world is drawn at
   half width and half height (960x540 on Series S), a quarter of the pixels,
   then stretched back to 1080p. This is why the world is softer than the menus.
   Other resolutions: see [Picking a resolution](#picking-a-resolution).
3. **The title's own upscaler** (`VICTORY_OWN_UPSCALER=1`). On Series S the
   title upscales with AMD FidelityFX (FSR), whose compute programs cannot be
   translated here, so that step drew nothing. A patch makes the title use its
   own temporal upscaler on every profile. It is not a quality setting in
   itself, but Series S needs it for a scale below 1.
4. **Ray walks end at the root** (`VICTORY_BVH_MISS=1`). The part of the title
   that builds its ray-tracing structures is off, so their nodes are stale.
   Instead of walking stale nodes, every box and triangle test is a miss. That
   avoids a hang and saves about 10 ms a frame, but light bounced off nearby
   surfaces is lost, so some dim rooms look darker.

### Settings that make it faster without changing the picture

5. **Memory compares on the card** (`VICTORY_CARD_BUFFERS_MIN_MB=1`,
   `VICTORY_CARD_BUFFERS_MB=1536`). Once the settings above had cut the GPU's
   work, the CPU was the limit: every frame it compared about 190 MB of title
   memory to find what the title had changed. These move the compares of copies
   of 1 MiB or more to the graphics card. Copying back fell from about 27 to
   10 ms a frame, and the frame rate doubled (11 to 21.8 fps).
6. **Wait for new shaders** (`VICTORY_PIPELINE_WAIT_MS=10000`). Draws wait for
   a pipeline still being made instead of being skipped: nothing is missing,
   with a short hitch the first time a place is seen. A launch precompiles
   first (see
   [Precompiling shaders and pipelines](#precompiling-shaders-and-pipelines)),
   which moves that hitch before the run.
7. **NVIDIA Smooth Motion** (by default in `play-nvidia.ps1`). The script
   starts a copy of the build named `victory_sm.exe`; with Smooth Motion turned
   on for that exe in the NVIDIA App, the driver inserts a generated frame
   between real ones, so motion looks about twice as smooth. The title still
   runs at the real rate underneath, so controls respond at about 20 fps.

### Code changes (what makes it work on NVIDIA at all)

8. **No more skipped draws on NVIDIA** (`VICTORY_DYNAMIC_SLOTS=524288`,
   `crates/victory-gpu/src/descriptor/host.rs`). NVIDIA caps the shader-visible
   view heap at 1,000,000 views, below the 1,245,184 the title asks for. The
   pool of dynamic descriptor slots (2^18) then ran out, and 1,100-4,800
   descriptor tables a frame were refused, each one a dropped draw: the screen
   rarely updated and the character jumped from place to place. The pool can
   now take up to half the heap: 0-33 refused a frame.
9. **Series S interface plane** (`VICTORY_COMPOSE=1`,
   `crates/victory-gpu/src/compose.rs`, `host.rs`, `queue.rs`). A Series S
   title presents two planes: the game (1080p) and the interface (4K). Only the
   first was shown, so menus, loading screens and subtitles were black until
   gameplay. One compute pass now lays the interface over the game before it
   is shown. Ported from fxceless/victory-gta6 pull request 1189 (Tissma95),
   without its FSR stand-in (the own upscaler is used instead).
10. **Title patches** (`crates/victory-loader/src/hooks.rs`, `lib.rs`,
    `windows.rs`): render scale, own upscaler, and `VICTORY_NO_WATCHDOG`
    (keeps the title's watchdog from ending a slow frame), taken from the
    victory-perf patches.
11. **Ray walks end at the root** (`crates/victory-shader/src/emit/intersect.rs`,
    `crates/victory-gpu/src/shader_cache.rs`): the `VICTORY_BVH_MISS` switch, made
    part of the shader cache key so translated shaders with and without it are
    kept apart.
12. **`scripts/play-nvidia.ps1`**: the launcher with all of this list.
13. **Faces move** (`crates/victory-shader/src/emit/halves.rs`, `lower.rs`,
    `crates/victory-gpu/src/descriptor/host.rs`, `draw.rs`). Faces are driven by
    a network of compute programs whose numbers travel as 16-bit values through
    integer views; they were read as floats and came back as zeros on NVIDIA, so
    no face moved. They now move as integers. Ported from fxceless/victory-fork
    pull request 927 (tippythetaidum).

14. **Ray tracing mostly off** (`scripts/rt_off_hashes.txt`, used by
    `play-nvidia.ps1` through `VICTORY_SKIP_PIPELINES`; `-RayTracing` keeps the
    passes). The title's ray-tracing structure builder cannot run on NVIDIA (its
    64-lane programs are refused; Marvisak/VIctory pull request 2311), so every
    ray misses. The passes that turn those misses into light made rooms go white
    and hazy depending on the view angle. The 1,000 `CS_RaytracingResolve` and
    34 traced-shadow shaders are skipped. The screen-probe and visibility traces
    keep running: skipping them too (all 1,052 shaders of fxceless pull request
    465's rule) left their outputs as garbage, rainbow glitter everywhere. Needs
    the `diag` build (the skip list is a diagnostics switch).
15. **The intro video no longer flickers** (`crates/victory-gpu/src/texture.rs`).
    The movie's planes (linear, one-level R8 textures in a ring of three) were
    written by the decoder through another view of the same memory, judged
    clean when the frame was recorded, and copied a list late: frames from the
    ring showed out of order. Video planes now always record their copy.

### Settings

The default set of `play-nvidia.ps1` is the speed set (about 18-22 fps on the
RTX 5070). `-Quality` trades about 7 fps for fewer of the first two faults
below.

| Fault | Cause | Fix |
|---|---|---|
| Stretched bodies that jump | The relay's second stage and card steps hand skinned meshes back before they are done (avatarmanz issue 13; fxceless issue 89 on an RTX 5070) | `-Quality`: `VICTORY_RELAY=1`, `VICTORY_CARD_STEPS=0` |
| Rainbow, pixelated skin | A texture refresh that does not wait for the runtime's remapping fills its upload from pages not there yet | `-Quality`: `VICTORY_TEXTURE_NO_WAIT=0` |
| Some views far too bright | Without rays the light probes keep the sky's light indoors, and the fog takes it too | Always: `VICTORY_AMBIENT_AIR=1.5` (fog at most 1.5 times the light around), `VICTORY_AMBIENT_MOST=2` (new, `ambient.rs`; was a fixed 4) |
| The picture stops until you look away | A pipeline that takes longer than the wait was waited for again in every frame it was in view | Always: each pipeline is waited for once at most (`VICTORY_PIPELINE_WAIT_ONCE`, on; `pipeline/make.rs`; the problem of fxceless issue 1181) |
| Black blobs on faces and in the air | A NaN that a shader writes is spread by blur and the temporal upscaler | Always: `VICTORY_NAN_SCRUB=1` (new, `emit/lower.rs`): colours and image stores with NaN become 0; NaN in the fog volume is cleared too |

Known faults left: the common room by its sunlit windows still goes beige and
hazy (sky light in the fog); smearing when the camera moves fast at half scale
(the title's own temporal upscaler); two orange glows on some corridor walls.
Render scale 1.0 on Series S has not been tested.

Also in, off by default:

- `VICTORY_CONSTANT_MARKS=1`, a port of fxceless pull request 1216
  (GamingRipperDoc): a slot picked from the heap is read as a constant buffer
  only if it holds one (else a texture's descriptor was read as one, in the
  meshlet culling passes).
- `VICTORY_TARGET_VARIANTS=1`, a port of fxceless a35c557393: a pipeline is made
  again for the render target formats bound at the draw.
- F9 frame capture (`capture.rs`): with `VICTORY_CAPTURE=<folder>` and
  `VICTORY_DUMP_TARGETS=1`, F9 saves the next frame's picture, its commands and
  every 2D target.
- `VICTORY_ADAPTER=<part of a card's name, or #index>` (`host.rs`): the card to
  run on when there is more than one (`play-nvidia.ps1 -Adapter`).

---

A fork of [avatarmanz/VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX),
itself a fork of [VIctory](https://github.com/Marvisak/VIctory), a compatibility
layer that runs an Xbox Series title natively on Windows from files the user
supplies. That fork is tuned for an AMD RX 7900 XTX. The NVIDIA fork adds its Series S
settings and NVIDIA fixes. This fork carries those changes and adds memory
handling improvements shared by NVIDIA and AMD adapters.

**This repository contains no game files, code or assets.** You need your own
copy of the game. It is not affiliated with or endorsed by the game's developer
or publisher.

## Earlier NVIDIA gameplay results (2026-11-25)

The earlier NVIDIA fork reported gameplay measurements on Windows 11,
AMD Ryzen 7 9800X3D, 64 GB RAM, and GeForce RTX 5070 12 GB, using the Series S
profile at half render scale (`scripts\play-nvidia.ps1`).
The RX 7600 memory validation described above covers targeted tests.
An AMD gameplay benchmark for these memory changes is pending.

| | |
|---|---|
| Prologue, indoors | about 20-22 fps on average, 24-31 fps in lighter places |
| Device losses | none in the measured runs |
| Menus, loading screens, subtitles | shown (they were black on Series S before) |
| First time a place is seen | slow for a moment while its shaders are made |

On the same card the fork's own settings skipped thousands of draws a frame
(the picture jumped and rarely updated), and the Series X profile at full scale
ran at about 6 fps.

What is known to be wrong:

- The picture is softer than Series X: half render scale, upscaled by the
  title's own upscaler.
- Rooms with little direct light can be dark: ray walks end at the root while
  the title's structure builder is off (`VICTORY_BVH_MISS`).
- Everything listed in the 7900 XTX fork's README still applies (flicker of far
  plants outdoors, textures appearing late while the title streams, a stutter on
  the first run with a new driver or build).

## How to run it

### Dedicated GPU memory

The title keeps CPU-writable console addresses. Sampled textures use a GPU-local
host texture and a separate upload buffer. On a discrete adapter supporting
GPU upload heaps, up to two 64 MiB texture upload blocks can use CPU-visible
VRAM while at least 20% of the reported local budget remains free. The fallback
uses system-RAM upload buffers. To compare the two paths on the same scene,
pass `-More @{VICTORY_TEXTURE_BAR='0'}` to `scripts/play-nvidia.ps1`.

The periodic sweep can release empty staging blocks after their submissions
finish. It keeps up to 128 MiB of empty staging blocks and drops that allowance
to zero below 2 GiB of available physical RAM. Live textures, buffers, and
pending copies have separate lifetimes and can exceed those cache limits.

### Build and launch

You need Windows 10/11 x64, a CPU with AVX2, a Direct3D 12 GPU with shader
model 6.6 and mesh shaders, Rust (the toolchain is pinned), the MSVC build
tools, and your own dump of the title.

```powershell
git clone --branch work https://github.com/Gamination/victory-4060.git
cd victory-4060
copy victory.example.toml victory.toml      # then set your paths, outside this folder
cargo build --locked --profile diag -p victory --features diagnostics
scripts\play-nvidia.ps1 -Exe <path of your copy's executable>
```

`scripts\play-nvidia.ps1` starts a copy of the build named `victory_sm.exe`, so
NVIDIA Smooth Motion applies every time once it is turned on for that exe in the
NVIDIA App. `-NoSmoothMotion` runs `victory.exe` itself; `-SeriesX` uses the
Series X profile (sharper, much slower); `-Quality` gives fewer stretched bodies
and less rainbow skin for about 7 fps; `-NoPrecompile` starts the title at once
instead of translating its shaders and making its pipelines first (see
[Precompiling shaders and pipelines](#precompiling-shaders-and-pipelines)). The
fork's own `scripts\play.ps1` is still there for AMD cards.

For AMD, launch the same build with the existing AMD settings:

```powershell
scripts\play.ps1 -Exe <path of your copy's executable>
```

To select an AMD adapter on a PC with multiple GPUs:

```powershell
scripts\play.ps1 -Exe <path of your copy's executable> -More @{VICTORY_ADAPTER='RX 7600'}
```

To compare system-RAM texture staging on AMD:

```powershell
scripts\play.ps1 -Exe <path of your copy's executable> -More @{VICTORY_TEXTURE_BAR='0'}
```

The AMD launcher uses its own profile and settings. The resolution wrappers
below call the NVIDIA launcher and apply its settings.

### Precompiling shaders and pipelines

The first time a place is seen, its shaders are translated and its host
pipelines compiled, and the picture waits while that happens. A launch does
that work before the title starts, when it has anything to do:

```powershell
scripts\play-nvidia.ps1 -Exe <path of your copy's executable>
```

That is `victory precompile --if-needed` run with the launcher's own settings,
then the title as usual. It translates every shader found in the title's files
and every shader earlier runs recorded into the shader cache, then makes the
host pipelines those runs drew with, so the driver's cache on disk has them.
It goes on where it stopped: both caches keep what was made, so only what is
new is done. Reading the title's files to find its shaders takes a minute on a
large install, so that is done only when the last precompile saw something
else: another build, other switches, a file of the title that changed or
appeared, a cache that lost entries. Otherwise the launch goes straight to the
pipelines, which are warmed every time (it skips what it has made).
`-Precompile` walks the title regardless; `-NoPrecompile` (or `run.bat skip`)
starts the title at once; `scripts\play.ps1` does the same for AMD cards, and
`victory precompile --shaders-only` does the shaders alone.

Two things to know:

- A translation depends on the switches a run uses (`VICTORY_NAN_SCRUB`,
  `VICTORY_BVH_MISS` and the other shader switches are part of its cache key),
  so precompile with the launcher: a cache made with other settings is not used
  by the run.
- Pipelines are known from earlier runs, and the launchers record them by
  default (`VICTORY_PIPELINE_BOOK=1`, the title's own shaders in the cache
  folder). So the first launch has no pipelines to make and says so; the
  second makes them.

The translations are on disk from the moment they are made, in
`<cache folder>\shaders` (the report names the cache folder). To keep them as
one file — for a cache that is wiped, or a second machine with the same build
and settings — `--save` writes every one of them and `--load` puts them back:

```powershell
victory precompile --save=<file>   # --save alone uses precompiled.vpc, beside the cache folder
victory precompile --load=<file>   # --load alone reads it
```

The launchers take `-SaveShaders` (and `-ShadersFile <path>`) and put the saved
file back at the start of every launch; `run.bat save` / `run.bat load` use the
default file. What is written holds the title's shaders, translated, and is
about the size of the cache folder (a few GB for this title): like the cache, it
stays outside the repository and is not shared.

Every launch does it, so a change of the build, of the launcher's settings, of
a file of the title or of the graphics driver is picked up by the next one: it
walks the title when there is anything new to find there, and makes the
pipelines whose own key changed. `-NoPrecompile` skips the whole step. Making
the 2,000-odd pipelines earlier runs drew with is the part that removes the
hitch when a place is first seen; `--all` also makes the tens of thousands the
title creates while it loads and never draws with, which takes much longer.

### Picking a resolution

There is one script per console and resolution in `scripts\`. Each one runs
`play-nvidia.ps1` with `-Resolution` set, and takes the same `-Exe` path and
other switches:

```powershell
scripts\play-S-720p.ps1 -Exe <path of your copy's executable>
scripts\play-X-1080p.ps1 -Exe <path of your copy's executable> -Quality
```

The resolution is the height the 3D world is drawn at. The title's own
upscaler then stretches it to the console's output, and menus, the HUD and
subtitles stay sharp either way. Series S outputs 1920x1080 and Series X
3840x2160, so the same height is a different render scale on each:

| Script | Console | World drawn at | Render scale | Tested on the RTX 5070 |
|---|---|---|---|---|
| `play-S-1080p.ps1` | Series S | 1920x1080 | 1.0 | **crashes at start** (the title stops at a breakpoint; also reported on an RTX 2080) |
| `play-S-720p.ps1` | Series S | 1280x720 | 0.667 | not yet |
| `play-S-540p.ps1` | Series S | 960x540 | 0.5 | yes: the default, about 18-22 fps |
| `play-S-480p.ps1` | Series S | 853x480 | 0.444 | not yet |
| `play-X-2160p.ps1` | Series X | 3840x2160 | 1.0 | yes, about 6 fps |
| `play-X-1440p.ps1` | Series X | 2560x1440 | 0.667 | not yet |
| `play-X-1080p.ps1` | Series X | 1920x1080 | 0.5 | yes: the `-SeriesX` default, about 7 fps |
| `play-X-720p.ps1` | Series X | 1280x720 | 0.333 | not yet |
| `play-X-540p.ps1` | Series X | 960x540 | 0.25 | not yet |
| `play-X-480p.ps1` | Series X | 853x480 | 0.222 | not yet |

What to expect:

- **Lower is faster, up to a point.** On this machine the frame time is mostly
  the CPU preparing the GPU's work (about 33 ms a frame on Series S), not the
  GPU drawing pixels. Going below 540p saves GPU time, but the frame rate stops
  rising once the CPU is the limit.
- **Series S is much faster than Series X at the same resolution.** The console
  profile changes far more than resolution: the title picks the smaller
  console's level of detail, effects and memory budgets. `play-S-540p` and
  `play-X-540p` draw the same number of pixels, but the Series X one does
  more work for each frame.
- **Lower is softer and smears more.** The title's temporal upscaler rebuilds
  detail from earlier frames. At low resolutions and low frame rates, fast
  camera moves (driving, turning quickly) leave visible trails.
- **Series S at 1080p (render scale 1.0) does not work yet.** The title stops
  at a breakpoint as soon as it starts, on a 12 GB card too, so it is not video
  memory. Use 720p or lower on Series S.
- Series S cannot go above 1080p: its output is 1080p, and the script says so
  and stops if asked for more.

### More than one card, and other cards

On a PC with two cards, `-Adapter` picks one by part of its name or its index
(`-Adapter 5060`, `-Adapter '#1'`); it sets `VICTORY_ADAPTER`, and the log says
which card the game is on. Without it the game uses Windows' default card, the
one the main display is on.

Measured on the same PC (Ryzen 7 9800X3D), same build and settings
(`play-S-540p`, the default), one run each in the prison prologue:

| Card | Slot it sits in | fps | Preparing a frame (CPU) | On the GPU |
|---|---|---|---|---|
| RTX 5070 12 GB | PCIe 5.0 x16, drives the display | 19.3 | 34 ms | 4.6 ms |
| RTX 5060 Ti 16 GB | PCIe 4.0 x4, no display attached | 14.9 | 50 ms | 5.4 ms |

The GPU work itself is close; the gap is in preparing frames, mostly copies of
title memory to and from the card. This layer moves tens of MB a frame, so the
link to the card matters: the 5060 Ti here has about an eighth of the 5070's
bandwidth, and its finished frames are also copied to the 5070 to be shown.
In a full-speed slot with the display on it, a 5060 Ti should land closer to
the 5070. Neither run lost the device; both refused the same 7 programs.

`-Resolution` works on `play-nvidia.ps1` itself too
(`-Resolution 720 -SeriesX`), and `-More @{VICTORY_RENDER_SCALE='0.6'}` still
sets any scale from 0.2 to 1.0.

Before the first run, ahead of time (both can be stopped and go on later):

```powershell
target\diag\victory.exe shaders <the shader archives>   # every shader translated, minutes
target\diag\victory.exe warm --used                     # pipelines earlier runs drew with, for a new driver or build
```

Commands, debugging switches and upstream's own description:
[docs/README-fxceless.md](docs/README-fxceless.md). Every `VICTORY_...` switch is
described in a comment where it is read.

## What is different here

This fork's own changes are listed at the top of this README. Everything the
7900 XTX fork added is kept: translation of list calls on threads
of ours, submissions prepared and joined on a thread of ours, the write relay,
fills done on the card, textures on their own copy queue, texture read-back
(boats float), and frame record and replay. Its
[docs/findings.md](docs/findings.md) has the reasons and the measurements.

## Credits

This is a tree of other people's work with more on top. The history is kept, and
commits taken from elsewhere name the commit and its author.

- **[avatarmanz/VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX)**:
  the fork this one started from, and everything in it.
- **[VIctory](https://github.com/Marvisak/VIctory)** by Marvisak and its
  contributors: the loader, the runtime services, the Direct3D 12 back end and
  the shader recompiler.
- **[fxceless/victory-fork](https://github.com/fxceless/victory-fork)**: the fork
  the 7900 XTX one started from, the people whose pull requests it carries,
  pull request 1189 (Tissma95) for the Series S interface plane, 927
  (tippythetaidum) for the face network and 1216 (GamingRipperDoc) for the
  marked constant buffer slots.
- **[DexNom/VIctory](https://github.com/DexNom/VIctory)**: ray tracing fixes,
  cube gradients, sparse textures.
- **[Belobug/VIctory-Belobug-AMD](https://github.com/Belobug/VIctory-Belobug-AMD)**:
  the ambient light idea, pipeline workers, and work by romainhedouin.
- **[FQY55kai/ZJOU-VICTORY](https://github.com/FQY55kai/ZJOU-VICTORY)**.
- **[myokex/victory-amd](https://github.com/myokex/victory-amd)**: the guard over
  every view of a page (with Spexivus, joroyt and tightfall), and Joshua Odom's
  reads into title memory.
- **Geckomacabre** and **cxy0072019-sketch**: the texture read-back.
- **[XCL](https://github.com/xcl-dev/xcl)** (GPL-2.0-only) was read for how the same
  title behaves under another layer; no code of it is here.
- The earlier NVIDIA fork's changes were made with
  [Claude Code](https://claude.com/claude-code).
- This fork's dedicated GPU memory changes were developed and tested with
  OpenAI Codex.

## License

GPL-3.0-or-later, as VIctory. See [LICENSE](LICENSE); the one file that is not
covered is named in [docs/asset-policy.md](docs/asset-policy.md).
