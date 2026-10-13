param([string]$OutputDirectory = 'docs/evidence/phase2')
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$bytes = [IO.File]::ReadAllBytes((Join-Path $root 'res/moulinetteb.exe'))
$script:reader = [IO.BinaryReader]::new([IO.MemoryStream]::new($bytes), [Text.Encoding]::Unicode)
function Read-Field {
    $first = $script:reader.ReadUInt16()
    if ($first -eq 0) { return '' }
    if ($first -eq 65535) { return ('ordinal:{0}' -f $script:reader.ReadUInt16()) }
    $text = [Text.StringBuilder]::new()
    [void]$text.Append([char]$first)
    while (($next = $script:reader.ReadUInt16()) -ne 0) { [void]$text.Append([char]$next) }
    return $text.ToString()
}
function Align-Reader { $script:reader.BaseStream.Position = ($script:reader.BaseStream.Position + 3) -band -4 }
$results = foreach ($block in @(@(101,712048,1422),@(102,713472,1214),@(103,714688,732),@(106,715420,182),@(108,715604,206),@(109,715812,1368),@(110,717180,962))) {
    $reader.BaseStream.Position = $block[1]
    $style = $reader.ReadUInt32()
    $extendedStyle = $reader.ReadUInt32()
    $count = $reader.ReadUInt16()
    $position = @($reader.ReadInt16(),$reader.ReadInt16(),$reader.ReadInt16(),$reader.ReadInt16())
    $menu = Read-Field
    $class = Read-Field
    $title = Read-Field
    if ($style -band 64) { $fontSize = $reader.ReadUInt16(); $font = Read-Field }
    $controls = for ($i=0; $i -lt $count; $i++) {
        Align-Reader
        $controlStyle = $reader.ReadUInt32()
        $controlExtendedStyle = $reader.ReadUInt32()
        $rect = @($reader.ReadInt16(),$reader.ReadInt16(),$reader.ReadInt16(),$reader.ReadInt16())
        $id = $reader.ReadUInt16()
        $controlClass = Read-Field
        $controlTitle = Read-Field
        $extra = $reader.ReadUInt16()
        if ($extra) { [void]$reader.ReadBytes($extra) }
        [pscustomobject]@{ ID=$id; Class=$controlClass; Title=$controlTitle; Style=('0x{0:x8}' -f $controlStyle); Rect=$rect }
    }
    if ($reader.BaseStream.Position -gt $block[1]+$block[2]) { throw 'Resource parsing exceeded the resource range' }
    [pscustomobject]@{ Dialog=$block[0]; Title=$title; Menu=$menu; Count=$count; Controls=@($controls) }
}
$destination = Join-Path $root $OutputDirectory
[void](New-Item -ItemType Directory -Force -Path $destination)
$results | ConvertTo-Json -Depth 8 | Set-Content -Encoding UTF8 (Join-Path $destination 'dialog-controls.json')
$results | ForEach-Object { 'Dialog {0}: {1}' -f $_.Dialog,$_.Title; $_.Controls | Select-Object ID,Class,Title | Format-Table -AutoSize }
