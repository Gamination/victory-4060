# Run one launcher with a live console and a separate, immediately readable log.
param(
    [Parameter(Mandatory)][string]$Launcher,
    [Parameter(Mandatory)][scriptblock]$Launch
)

$directory = Join-Path $env:LOCALAPPDATA 'victory\logs'
[IO.Directory]::CreateDirectory($directory) | Out-Null
$name = 'run-{0}-{1}.log' -f (Get-Date -Format 'yyyyMMdd-HHmmss-fff'), [guid]::NewGuid().ToString('N')
$path = Join-Path $directory $name
$file = [IO.File]::Open($path, 'CreateNew', 'Write', 'Read')
$log = [IO.StreamWriter]::new($file, [Text.UTF8Encoding]::new($false))
$log.AutoFlush = $true
$status = @{ ExitCode = 0 }
$finished = $false
try {
    Write-Host "Run log: $path"
    $log.WriteLine("Started $(Get-Date -Format o) with $Launcher")
    & $Launch $status *>&1 | ForEach-Object {
        $line = [string]$_
        $log.WriteLine($line)
        if ($_ -is [System.Management.Automation.InformationRecord]) {
            Write-Host $line
        } else {
            Write-Output $line
        }
    }
    $finished = $true
} catch {
    $status.ExitCode = 1
    $finished = $true
    $log.WriteLine("Launcher error: $_")
    throw
} finally {
    try {
        if ($finished) {
            $log.WriteLine("Ended $(Get-Date -Format o); exit code $($status.ExitCode)")
        } else {
            $log.WriteLine("Interrupted $(Get-Date -Format o)")
        }
    } finally {
        $log.Dispose()
    }
}
exit $status.ExitCode
