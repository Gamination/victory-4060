param([hashtable]$Settings, [string]$FfxDir)

# Do not let the inherited own-upscaler patch bypass the FFX dispatch.
# Start with the title's natural resolution rather than patching its FSR
# context sizes. The SDK bridge itself still supports changing dimensions.
if (-not $FfxDir) { $FfxDir = Join-Path $env:LOCALAPPDATA 'VIctory\sdk\ffx-2.4.0' }
foreach ($dll in @('amd_fidelityfx_loader_dx12.dll', 'amd_fidelityfx_upscaler_dx12.dll')) {
    if (-not (Test-Path -LiteralPath (Join-Path $FfxDir $dll) -PathType Leaf)) { throw "PC FidelityFX SDK missing $dll in $FfxDir" }
}
$settings.Remove('VICTORY_RENDER_SCALE')
$settings.VICTORY_OWN_UPSCALER = '0'
$settings.VICTORY_PC_FFX_DIR = (Resolve-Path -LiteralPath $FfxDir).Path
$settings.VICTORY_PC_FFX_NATIVE = '1'
$settings.VICTORY_PC_FFX_ENTRY = 'loader'
$settings.VICTORY_PC_FFX_LAZY = '1'
$settings.VICTORY_PC_FFX_RECOVER = '1'
$settings.VICTORY_PC_FFX_TRACE = '1'
$settings.VICTORY_PC_FFX_PROBE = '0'
$settings.VICTORY_PC_FFX_CONTEXT_TEST = '0'
$settings.VICTORY_PC_FFX_DISPATCH_TEST = '0'
