param([switch]$Launch, [ValidateRange(10,3600)][int]$TimeoutSeconds = 600)
$ErrorActionPreference = 'Stop'
$caseRoot = $PSScriptRoot
$logs = Join-Path $caseRoot 'logs'
if (!(Test-Path -LiteralPath (Join-Path $logs 'preparation.json'))) { throw 'Not a prepared case' }
if (!$Launch) { throw 'Use -Launch only inside the prepared Windows Sandbox or a disposable VM. No process started.' }
if (Test-Path -LiteralPath (Join-Path $logs 'run.json')) { throw 'This case was already run; prepare a new case' }
function Get-CaseSnapshot {
    @(Get-ChildItem -LiteralPath $caseRoot -File -Recurse | Where-Object { !$_.FullName.StartsWith($logs + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) } | Sort-Object FullName | ForEach-Object {
        [pscustomobject]@{ Path=$_.FullName.Substring($caseRoot.Length+1); Bytes=$_.Length; SHA256=(Get-FileHash -LiteralPath $_.FullName).Hash }
    })
}
# Rewrite paths for the actual guest location; never use the supplied drive mappings.
$iniLines = @(
    ('INPUT_PATH= ' + (Join-Path $caseRoot 'input')),
    ('OUTPUT_PATH= ' + (Join-Path $caseRoot 'output')),
    ('INI_PATH= ' + (Join-Path $caseRoot 'ini')),
    'RBS_HEADING= RBS.INI','NRA_HEADING= NRA.INI','ERD_HEADING= ERD.INI','EXPERIMENT_HEADING= EXP.INI'
)
if ($iniLines | Where-Object { $_.Length -gt 255 }) { throw 'Case paths exceed the application buffer width' }
[IO.File]::WriteAllText((Join-Path $caseRoot 'moulinette.ini'), (($iniLines | ForEach-Object { $_.PadRight(255) }) -join "`r`n") + "`r`n", [Text.Encoding]::ASCII)
$before = Get-CaseSnapshot
$before | ConvertTo-Json -Depth 4 | Set-Content -Encoding UTF8 (Join-Path $logs 'before.json')
$started = [DateTime]::UtcNow
$process = $null
$timedOut = $false
$exitCode = $null
$launchError = $null
try {
    $process = Start-Process -FilePath (Join-Path $caseRoot 'moulinetteb.exe') -WorkingDirectory $caseRoot -PassThru -WindowStyle Normal -RedirectStandardOutput (Join-Path $logs 'stdout.txt') -RedirectStandardError (Join-Path $logs 'stderr.txt')
    Write-Output 'Interact with the GUI now. Capture settings/screenshots in logs. Exit the entire application when finished.'
    if (!$process.WaitForExit($TimeoutSeconds * 1000)) {
        $timedOut = $true
        Stop-Process -Id $process.Id -Force
    }
    $process.WaitForExit()
    $exitCode = $process.ExitCode
} catch { $launchError = $_.Exception.Message }
$after = Get-CaseSnapshot
$after | ConvertTo-Json -Depth 4 | Set-Content -Encoding UTF8 (Join-Path $logs 'after.json')
$beforeIndex = @{}
foreach ($entry in $before) { $beforeIndex[$entry.Path] = $entry.SHA256 }
$changed = @($after | Where-Object { !$beforeIndex.ContainsKey($_.Path) -or $beforeIndex[$_.Path] -ne $_.SHA256 })
$afterPaths = @($after | ForEach-Object { $_.Path })
$deleted = @($before | Where-Object { $_.Path -notin $afterPaths })
[ordered]@{ StartedAtUtc=$started.ToString('o'); EndedAtUtc=[DateTime]::UtcNow.ToString('o'); PID=$(if($process){$process.Id}else{$null}); ExitCode=$exitCode; TimedOut=$timedOut; LaunchError=$launchError; ChangedOrCreated=$changed; Deleted=$deleted; ConsoleCapture='QuickWin window text may not appear on stdout/stderr; record it manually.' } | ConvertTo-Json -Depth 6 | Set-Content -Encoding UTF8 (Join-Path $logs 'run.json')
if ($launchError) { throw $launchError }
Write-Output ('Captured run: ' + (Join-Path $logs 'run.json'))
