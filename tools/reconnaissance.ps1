param([string]$OutputDirectory = 'docs/evidence/phase1')
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$destination = Join-Path $root $OutputDirectory
New-Item -ItemType Directory -Force -Path $destination | Out-Null
$binary = Join-Path $root 'res/moulinetteb.exe'
$sample = Join-Path $root 'res/15032501.mpa'
Get-FileHash -Algorithm SHA256 -LiteralPath $binary,$sample,(Join-Path $root 'res/moulinette.ini') |
    Select-Object Path,Hash | ConvertTo-Json | Set-Content -Encoding UTF8 (Join-Path $destination 'hashes.json')
& llvm-objdump -p $binary | Set-Content -Encoding UTF8 (Join-Path $destination 'pe-headers-imports.txt')
if ($LASTEXITCODE -ne 0) { throw 'llvm-objdump header inspection failed' }
& llvm-objdump -h $binary | Set-Content -Encoding UTF8 (Join-Path $destination 'sections.txt')
if ($LASTEXITCODE -ne 0) { throw 'llvm-objdump section inspection failed' }
& llvm-objdump -d --start-address=0x4481b0 --stop-address=0x448380 $binary |
    Set-Content -Encoding UTF8 (Join-Path $destination 'entry-disassembly.txt')
if ($LASTEXITCODE -ne 0) { throw 'llvm-objdump disassembly failed' }
$bytes = [IO.File]::ReadAllBytes($binary)
# Latin-1 maps bytes one-to-one: non-ASCII bytes cannot create false ASCII runs.
$text = [Text.Encoding]::GetEncoding(28591).GetString($bytes)
[regex]::Matches($text,'[\x20-\x7e]{4,}') | ForEach-Object {
    '0x{0:X8} {1}' -f $_.Index,$_.Value
} | Set-Content -Encoding UTF8 (Join-Path $destination 'ascii-strings.txt')
# Payload ranges are from REA inspect_pe_resources, not inferred control layouts.
$dialogs = foreach ($block in @(@(101,712048,1422),@(102,713472,1214),@(103,714688,732),@(106,715420,182),@(108,715604,206),@(109,715812,1368),@(110,717180,962))) {
    "Dialog $($block[0]): raw UTF-16 printable runs (may include incidental data)"
    $decoded = [Text.Encoding]::Unicode.GetString($bytes,$block[1],$block[2])
    [regex]::Matches($decoded,'[\x20-\x7e\u00a0-\u00ff]{3,}') | ForEach-Object { $_.Value }
}
$dialogs | Set-Content -Encoding UTF8 (Join-Path $destination 'dialog-text.txt')
$sampleBytes = [IO.File]::ReadAllBytes($sample)
$sampleText = [Text.Encoding]::GetEncoding(28591).GetString($sampleBytes)
$lines = $sampleText -split '\r\n'
$spectra = for ($i=0; $i -lt $lines.Length; $i++) {
    if ($lines[$i] -match '^\[DATA(\d+),\s*(\d+)\s*\]$') {
        $id = [int]$Matches[1]
        $declared = [int]$Matches[2]
        $counts = [Collections.Generic.List[long]]::new()
        for ($j=$i+1; $j -lt $lines.Length -and $lines[$j] -notmatch '^\['; $j++) {
            if ($j -eq $lines.Length-1 -and $lines[$j] -eq '') { break }
            $value = [long]0
            if (-not [long]::TryParse($lines[$j].Trim(),[ref]$value)) { throw "Noninteger data at line $($j+1)" }
            $counts.Add($value)
        }
        $stats = $counts | Measure-Object -Sum -Minimum -Maximum
        [pscustomobject]@{ Section="DATA$id"; HeaderLine=$i+1; Declared=$declared; Observed=$counts.Count; Sum=[long]$stats.Sum; Minimum=$stats.Minimum; Maximum=$stats.Maximum; First=$counts[0]; Last=$counts[$counts.Count-1] }
    }
}
[pscustomobject]@{
    Bytes=$sampleBytes.Length
    CRLFPairs=([regex]::Matches($sampleText,'\r\n')).Count
    HighBytes=@($sampleBytes | Where-Object { $_ -ge 128 } | Group-Object | Select-Object Name,Count)
    Spectra=@($spectra)
} | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 (Join-Path $destination 'sample-summary.json')
Write-Output "Static reconnaissance saved to $destination"
