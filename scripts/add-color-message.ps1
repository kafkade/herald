<#
.SYNOPSIS
    Add a message with colored tiles to the Herald board rotation.
    Builds a raw 6x22 grid with text on specified rows and color fills.
.EXAMPLE
    .\add-color-message.ps1 -Text "GO TEAM" -Color green
    .\add-color-message.ps1 -Text "ALERT" -Color red -FillRows top
    .\add-color-message.ps1 -Text "HELLO" -Color blue -FillRows all
    .\add-color-message.ps1 -Text "SUNSET" -BgColor orange -FgColor black
.NOTES
    Uses $env:HERALD_ADMIN_TOKEN if -Token is not provided.
    Colors: red, orange, yellow, green, blue, violet, white, black
#>
param(
    [string]$Token,
    [Parameter(Mandatory)][string]$Text,
    [ValidateSet("red","orange","yellow","green","blue","violet","white","black")]
    [string]$Color = "white",
    [ValidateSet("red","orange","yellow","green","blue","violet","white","black")]
    [string]$BgColor,
    [ValidateSet("red","orange","yellow","green","blue","violet","white","black")]
    [string]$FgColor,
    [ValidateSet("none","top","bottom","all")][string]$FillRows = "none",
    [ValidateSet("left","center","right")][string]$HAlign = "center",
    [string]$Server = "http://localhost:3000"
)

if (-not $Token) { $Token = $env:HERALD_ADMIN_TOKEN }
if (-not $Token) { Write-Error "Provide -Token or set `$env:HERALD_ADMIN_TOKEN"; return }

# Use BgColor/FgColor if set, otherwise fall back to Color for both concept
if (-not $BgColor) { $BgColor = $Color }

$rows = 6; $cols = 22
$blank = @{ type = "blank" }

# Build the grid
$grid = @()
for ($r = 0; $r -lt $rows; $r++) {
    $row = @()
    for ($c = 0; $c -lt $cols; $c++) { $row += $blank }
    $grid += ,@($row)
}

# Determine which rows get color fill
$colorFillRows = @()
switch ($FillRows) {
    "top"    { $colorFillRows = @(0, 1) }
    "bottom" { $colorFillRows = @(4, 5) }
    "all"    { $colorFillRows = @(0, 1, 2, 3, 4, 5) }
}

$colorCell = @{ type = "color"; value = $BgColor }
foreach ($r in $colorFillRows) {
    for ($c = 0; $c -lt $cols; $c++) {
        $grid[$r][$c] = $colorCell
    }
}

# Place text (uppercase, centered or aligned)
$textUpper = $Text.ToUpper()

# Word-wrap into lines of max 22 chars
$words = $textUpper -split '\s+'
$lines = @()
$current = ""
foreach ($w in $words) {
    if ($current.Length -eq 0) { $current = $w }
    elseif (($current.Length + 1 + $w.Length) -le $cols) { $current += " $w" }
    else { $lines += $current; $current = $w }
}
if ($current.Length -gt 0) { $lines += $current }

# Vertically center text lines
$startRow = [math]::Floor(($rows - $lines.Count) / 2)

for ($li = 0; $li -lt $lines.Count; $li++) {
    $line = $lines[$li]
    $r = $startRow + $li
    if ($r -lt 0 -or $r -ge $rows) { continue }

    # Horizontal alignment
    switch ($HAlign) {
        "left"   { $startCol = 0 }
        "right"  { $startCol = $cols - $line.Length }
        default  { $startCol = [math]::Floor(($cols - $line.Length) / 2) }
    }

    for ($ci = 0; $ci -lt $line.Length; $ci++) {
        $ch = $line[$ci]
        $col = $startCol + $ci
        if ($col -ge 0 -and $col -lt $cols) {
            $grid[$r][$col] = @{ type = "char"; value = "$ch" }
        }
    }
}

$body = @{ grid = $grid }

$result = Invoke-RestMethod -Uri "$Server/api/messages" -Method Post `
    -Headers @{ Authorization = "Bearer $Token" } `
    -ContentType "application/json" `
    -Body ($body | ConvertTo-Json -Depth 5)

Write-Host "Color message added (id: $($result.id), color: $BgColor, fill: $FillRows)"
$result
