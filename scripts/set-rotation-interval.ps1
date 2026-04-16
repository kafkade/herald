<#
.SYNOPSIS
    Update the rotation interval (seconds between board transitions).
.EXAMPLE
    .\set-rotation-interval.ps1 -Seconds 15
    .\set-rotation-interval.ps1 -Seconds 60
.NOTES
    Uses $env:HERALD_ADMIN_TOKEN if -Token is not provided.
#>
param(
    [string]$Token,
    [Parameter(Mandatory)][int]$Seconds,
    [string]$Server = "http://localhost:3000"
)

if (-not $Token) { $Token = $env:HERALD_ADMIN_TOKEN }
if (-not $Token) { Write-Error "Provide -Token or set `$env:HERALD_ADMIN_TOKEN"; return }

if ($Seconds -lt 1) {
    Write-Error "Interval must be at least 1 second."
    return
}

$body = @{ rotation_interval_seconds = $Seconds.ToString() }

$result = Invoke-RestMethod -Uri "$Server/api/config" -Method Put `
    -Headers @{ Authorization = "Bearer $Token" } `
    -ContentType "application/json" `
    -Body ($body | ConvertTo-Json)

Write-Host "Rotation interval set to ${Seconds}s"
$result
