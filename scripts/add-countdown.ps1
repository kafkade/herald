<#
.SYNOPSIS
    Add a countdown timer to the Herald board rotation.
.EXAMPLE
    .\add-countdown.ps1 -Label "LAUNCH" -Target "2026-12-31T00:00:00Z"
    .\add-countdown.ps1 -Label "MEETING" -InMinutes 45
    .\add-countdown.ps1 -Label "DEADLINE" -InHours 8
.NOTES
    Uses $env:HERALD_ADMIN_TOKEN if -Token is not provided.
#>
param(
    [string]$Token,
    [Parameter(Mandatory)][string]$Label,
    [string]$Target,       # ISO 8601 datetime
    [int]$InMinutes,       # countdown N minutes from now
    [int]$InHours,         # countdown N hours from now
    [string]$Server = "http://localhost:3000",
    [ValidateSet("show_zero","remove","pause")][string]$ZeroBehavior = "show_zero"
)

if (-not $Token) { $Token = $env:HERALD_ADMIN_TOKEN }
if (-not $Token) { Write-Error "Provide -Token or set `$env:HERALD_ADMIN_TOKEN"; return }

if (-not $Target -and -not $InMinutes -and -not $InHours) {
    Write-Error "Provide -Target (ISO datetime), -InMinutes, or -InHours"
    return
}

if ($InMinutes) {
    $Target = (Get-Date).AddMinutes($InMinutes).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
} elseif ($InHours) {
    $Target = (Get-Date).AddHours($InHours).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
}

$body = @{
    label = $Label
    target = $Target
    zero_behavior = @{ action = $ZeroBehavior }
}

$result = Invoke-RestMethod -Uri "$Server/api/countdowns" -Method Post `
    -Headers @{ Authorization = "Bearer $Token" } `
    -ContentType "application/json" `
    -Body ($body | ConvertTo-Json -Depth 3)

Write-Host "Countdown added (id: $($result.id), target: $Target)"
$result
