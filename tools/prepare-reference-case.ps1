param(
    [ValidateSet('Baseline','LabelsExact','Sentinel9','Calibration','TimeDistinct','MissingCaloff','Truncated')]
    [string]$Variant = 'Baseline',
    [string]$CaseName
)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
if (!$CaseName) { $CaseName = $Variant.ToLowerInvariant() + '-' + [Guid]::NewGuid().ToString('N').Substring(0,8) }
if ($CaseName -notmatch '^[a-zA-Z0-9_-]+$') { throw 'CaseName must contain only letters, numbers, hyphens and underscores' }
$caseRoot = Join-Path $root ('experiments/cases/' + $CaseName)
if (Test-Path -LiteralPath $caseRoot) { throw 'Case already exists; use a fresh name' }
[void](New-Item -ItemType Directory -Path $caseRoot)
foreach ($dir in @('input','output','ini','logs')) { [void](New-Item -ItemType Directory -Path (Join-Path $caseRoot $dir)) }
Copy-Item -LiteralPath (Join-Path $root 'res/moulinetteb.exe') -Destination (Join-Path $caseRoot 'moulinetteb.exe')
$encoding = [Text.Encoding]::GetEncoding(28591)
$sample = [IO.File]::ReadAllBytes((Join-Path $root 'res/15032501.mpa'))
$text = $encoding.GetString($sample)
switch ($Variant) {
    'LabelsExact' {
        $text = $text.Replace('cmline1=RBS135'+[char]176, 'cmline1=RBS').Replace('cmline1=RBS165'+[char]176,'cmline1=RBS').Replace('cmline1=RD30'+[char]176,'cmline1=ERD').Replace('cmline1=1D','cmline1=MON')
    }
    'Sentinel9' {
        $text = [regex]::Replace($text, '(?m)^range=(8192|256)\r?$', 'range=9')
        $text = [regex]::Replace($text, '(?ms)^\[DATA(\d+),[^\]]+\]\r\n.*?(?=^\[DATA|\z)', {
            param($match)
            $id = [int]$match.Groups[1].Value
            $values = 0..8 | ForEach-Object { [string](1000 * ($id + 1) + $_) }
            '[DATA'+$id+',9 ]'+"`r`n"+($values -join "`r`n")+"`r`n"
        })
    }
    'Calibration' {
        $text = [regex]::Replace($text,'(?s)(\[ADC1\].*?)(?=\[ADC2\])', {
            param($match)
            $match.Value.Replace('caloff=0.000000','caloff=1.250000').Replace('calfact=1.000000','calfact=2.500000').Replace('calfact2=0','calfact2=9.5').Replace('calfact3=0','calfact3=11').Replace('caluse=0','caluse=1')
        })
    }
    'TimeDistinct' {
        $text = [regex]::Replace($text,'(?s)(\[ADC1\].*?)(?=\[ADC2\])', {
            param($match)
            [regex]::Replace([regex]::Replace([regex]::Replace($match.Value,'(?m)^runtime=.*?\r?$', 'runtime=12.345'),'(?m)^livetime=.*?\r?$', 'livetime=4.567'),'(?m)^realtime=.*?\r?$', 'realtime=8.901')
        })
        # Normalize CRLF after regex substitutions that consumed the CR.
        $text = $text.Replace("`r`n","`n").Replace("`n","`r`n")
    }
    'MissingCaloff' { $text = [regex]::Replace($text,'(?m)^caloff=.*?\r\n','') }
    'Truncated' { $text = $text.Substring(0, $text.LastIndexOf("`r`n", $text.Length-3)+2) }
}
$inputPath = Join-Path $caseRoot 'input/15032501.mpa'
if ($Variant -eq 'Baseline') { [IO.File]::WriteAllBytes($inputPath, $sample) }
else { [IO.File]::WriteAllBytes($inputPath, $encoding.GetBytes($text)) }
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'run-reference-case.ps1') -Destination (Join-Path $caseRoot 'run-reference-case.ps1')
$manifest = [ordered]@{
    PreparedAtUtc = [DateTime]::UtcNow.ToString('o'); Variant = $Variant; CaseName = $CaseName
    OriginalExecutableSHA256 = (Get-FileHash -LiteralPath (Join-Path $root 'res/moulinetteb.exe')).Hash
    OriginalInputSHA256 = (Get-FileHash -LiteralPath (Join-Path $root 'res/15032501.mpa')).Hash
    FixtureSHA256 = (Get-FileHash -LiteralPath $inputPath).Hash
    Notes = 'Never executed during preparation. Sentinel9 changes ranges/counts but retains other metadata totals intentionally. INI heading templates absent.'
}
$manifest | ConvertTo-Json | Set-Content -Encoding UTF8 (Join-Path $caseRoot 'logs/preparation.json')
$hostFolder = [Security.SecurityElement]::Escape($caseRoot)
@"
<Configuration>
  <Networking>Disable</Networking>
  <vGPU>Disable</vGPU>
  <ClipboardRedirection>Disable</ClipboardRedirection>
  <PrinterRedirection>Disable</PrinterRedirection>
  <AudioInput>Disable</AudioInput>
  <VideoInput>Disable</VideoInput>
  <MappedFolders>
    <MappedFolder>
      <HostFolder>$hostFolder</HostFolder>
      <SandboxFolder>C:\MoulinetteCase</SandboxFolder>
      <ReadOnly>false</ReadOnly>
    </MappedFolder>
  </MappedFolders>
</Configuration>
"@ | Set-Content -Encoding UTF8 (Join-Path $caseRoot 'reference.wsb')
Write-Output ('Prepared: ' + $caseRoot)
Write-Output 'No executable launched. Open reference.wsb manually; in the sandbox run the copied runner with -Launch.'
