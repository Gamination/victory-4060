# Starts the title with the settings this fork is played with (README.md, "How to run it").
#   scripts\play.ps1 -Exe <path of your own copy's executable>
# -Safe         without the newer, faster ways (the first thing to try when something is wrong)
# -RayTracing   the title's own ray-traced indirect light: lit interiors, about a third slower
# -Build        target\diag (the default, the build all measurements are of) or target\release
#               A launch translates every shader of the title and makes the host pipelines earlier runs drew with
#               before starting it, and puts back what -SaveShaders wrote: the first time a place is seen is then
#               not slow while they are made. The first run only records the pipelines (VICTORY_PIPELINE_BOOK, on
#               here), and only what is new is done.
# -Precompile   walk the title even if nothing changed since the last precompile (a launch walks it
#               only when the build, the settings, a file of the title or the cache changed).
# -NoPrecompile start it without precompiling at all (the quick launch, when the caches are made).
# -SaveShaders  after precompiling, write every translated shader into one file (a wiped cache need not translate
#               them again). A launch puts the saved file back when it is there.
# -ShadersFile  the file -SaveShaders writes and a launch loads. Default: precompiled.vpc, beside the cache.
# -More         other settings: -More @{VICTORY_PANEL='1'}
# Run it from a normal window, not as administrator: the driver then keeps its shader cache elsewhere.
param(
    [Parameter(Mandatory)][string]$Exe,
    [switch]$Safe,
    [switch]$RayTracing,
    [switch]$NativeFSR3,
    [switch]$TextureStreaming,
    [string]$FfxDir,
    [string]$Adapter,
    [ValidateSet('diag', 'release')][string]$Build = 'diag',
    [switch]$Precompile,
    [switch]$NoPrecompile,
    [switch]$SaveShaders,
    [switch]$LoadShaders,
    [string]$ShadersFile,
    [hashtable]$More = @{}
)
& "$PSScriptRoot\run-logged.ps1" -Launcher play.ps1 -Launch {
    param($status)

    $settings = @{
        VICTORY_TEXTURE_NO_WAIT = '1'; VICTORY_RELAY = '2'; VICTORY_WIDE_BACK = '1'; VICTORY_CARD_REHASH_RUNS = '1'
        VICTORY_JOIN_COPIES = '1'; VICTORY_INDIRECT_WINDOW = '1'; VICTORY_RELAY_COPY = '1'; VICTORY_CARD_STEPS = '2'
        VICTORY_INDIRECT_BATCH = '2'; VICTORY_MAP_THREADS = '1'; VICTORY_TITLE_PRIORITIES = '0'; VICTORY_CARD_AWAKE = '1'
        VICTORY_SHOW_AFTER = '1'; VICTORY_AMBIENT = '0.8'; VICTORY_D3D12_DRED = 'light'; VICTORY_SKIP_PIPELINES = 'none'
        # What a run creates is written down for -Precompile to make ahead of the runs after it (pipeline/known.rs).
        VICTORY_PIPELINE_BOOK = '1'
    }
    if ($Adapter) { $settings.VICTORY_ADAPTER = $Adapter }
    if ($Safe) {
        $settings.VICTORY_UNDER_WIDE = '0'; $settings.VICTORY_LAND_ECHO = '0'
    } else {
        $settings.VICTORY_THIN = '1'; $settings.VICTORY_JOIN_PARTS = '8'; $settings.VICTORY_UPLOAD_AHEAD = '1'
    }
    if ($RayTracing) { $settings.VICTORY_BVH_BUILDER = '1' }
    if ($TextureStreaming) {
        $settings.VICTORY_HELD_LEVELS = '1'
        $settings.VICTORY_LEVELS_ON_DEMAND = '1'
        $settings.VICTORY_LEVELS_MIN_KB = '1024'
    }
    if ($NativeFSR3) {
        if ($Safe) { throw 'NativeFSR3 requires thin lists and the Series S profile at title-selected resolution.' }
        & "$PSScriptRoot\native-fsr3.ps1" -Settings $settings -FfxDir $FfxDir
    }
    foreach ($k in $More.Keys) { $settings[$k] = $More[$k] }
    if ($NativeFSR3 -and ($settings.VICTORY_OWN_UPSCALER -ne '0' -or $settings.ContainsKey('VICTORY_RENDER_SCALE') -or $settings.VICTORY_THIN -ne '1' -or $settings.VICTORY_PC_FFX_NATIVE -ne '1')) { throw 'Conflicting native FSR3 settings.' }
    if ($NativeFSR3) { Write-Host 'Series S, native FSR3 at title-selected resolution' }
    if ($TextureStreaming) { Write-Host 'Radeon profile with texture streaming requested' }

    $victory = Join-Path $PSScriptRoot "..\target\$Build\victory.exe"
    if (-not (Test-Path -LiteralPath $victory)) { $status.ExitCode = 2; Write-Host "No $victory. Build it first (README.md)."; return }
    if (-not (Test-Path -LiteralPath $Exe)) { $status.ExitCode = 2; Write-Host "No $Exe."; return }
    # (Other settings left over in this window go; where your files are stays.)
    $paths = 'VICTORY_CONFIG', 'VICTORY_CARD_BAR', 'VICTORY_READBACK_BAR', 'VICTORY_GAME_DIR', 'VICTORY_CACHE_DIR', 'VICTORY_DATA_DIR', 'VICTORY_LOW_RAM',
        'VICTORY_ARENA_MB', 'VICTORY_MAX_PAGES', 'VICTORY_FAULTS_PER_FRAME', 'VICTORY_FAULTS_ALL', 'VICTORY_UPLOAD_ARENA_MB', 'VICTORY_TEXTURE_SPARE_KEPT', 'VICTORY_STAGING_KEPT', 'VICTORY_RIPE_BLOCKS', 'VICTORY_CARD_BUFFERS_MIN_KB', 'VICTORY_CARD_BUFFERS_MB'
    Get-ChildItem Env:VICTORY_* | Where-Object { $_.Name -notin $paths } | ForEach-Object { Remove-Item "Env:$($_.Name)" }
    foreach ($k in $settings.Keys) { Set-Item "Env:$k" $settings[$k] }
    $Exe = (Resolve-Path -LiteralPath $Exe).Path
    Push-Location (Join-Path $PSScriptRoot '..')   # victory.toml is looked for here
    try {
        # What every launch does first (the default): the shaders and pipelines this run would make, made before it
        # starts, and what -SaveShaders wrote, put back first. -NoPrecompile is the quick launch.
        $restore = (-not $NoPrecompile) -or $LoadShaders
        if ($restore -or $SaveShaders) {
            Write-Host 'Precompiling the title''s shaders and pipelines (the cache makes the next times quicker; -NoPrecompile skips this)...'
            $preArgs = @((Split-Path $Exe))
            if ($restore) {
                if ($ShadersFile) {
                    # Only there after a -SaveShaders run: the first launch has none.
                    if (Test-Path $ShadersFile) { $preArgs += "--load=$ShadersFile" }
                    else { Write-Host "No $ShadersFile yet: nothing to load." }
                } else { $preArgs += '--load' }
                # A launch walks the title only when the last precompile saw something else (its files,
                # the build, the settings, the cache); -Precompile walks it always.
                if (-not $Precompile) { $preArgs += '--if-needed' }
            }
            if ($SaveShaders) {
                if ($ShadersFile) { $preArgs += "--save=$ShadersFile" } else { $preArgs += '--save' }
            }
            & $victory precompile @preArgs 2>&1
            if ($LASTEXITCODE -ne 0) { $status.ExitCode = $LASTEXITCODE; Write-Host "Precompile ended with exit code $LASTEXITCODE."; return }
        }
        $runArgs = @('run', $Exe, '--execute')
        if ($NativeFSR3) { $runArgs += @('--console', 'series-s') }
        & $victory @runArgs 2>&1
        $status.ExitCode = $LASTEXITCODE
    } finally { Pop-Location }
}
exit $LASTEXITCODE
