<#
.SYNOPSIS
    Remove a countdown from the Herald board rotation.
.EXAMPLE
    .\remove-countdown.ps1 -Id "some-uuid"
    .\remove-countdown.ps1 -All
    .\remove-countdown.ps1          # lists countdowns to choose from
.NOTES
    Uses $env:HERALD_ADMIN_TOKEN if -Token is not provided.
#>
param(
    [string]$Token,
    [string]$Id,
    [switch]$All,
    [string]$Server = "http://localhost:3000"
)

if (-not $Token) { $Token = $env:HERALD_ADMIN_TOKEN }
if (-not $Token) { Write-Error "Provide -Token or set `$env:HERALD_ADMIN_TOKEN"; return }

$headers = @{ Authorization = "Bearer $Token" }

if (-not $Id -and -not $All) {
    $list = Invoke-RestMethod -Uri "$Server/api/countdowns" -Method Get -Headers $headers
    if ($list.total -eq 0) {
        Write-Host "No countdowns in rotation."
        return
    }
    Write-Host "Countdowns in rotation:"
    foreach ($c in $list.items) {
        Write-Host "  $($c.id)  $($c.label)  -> $($c.target)"
    }
    Write-Host "`nUse -Id <uuid> to remove a specific countdown, or -All to remove all."
    return
}

if ($All) {
    $list = Invoke-RestMethod -Uri "$Server/api/countdowns" -Method Get -Headers $headers
    foreach ($c in $list.items) {
        Invoke-RestMethod -Uri "$Server/api/countdowns/$($c.id)" -Method Delete -Headers $headers | Out-Null
        Write-Host "Deleted $($c.id)  $($c.label)"
    }
    Write-Host "All $($list.total) countdown(s) removed."
} else {
    Invoke-RestMethod -Uri "$Server/api/countdowns/$Id" -Method Delete -Headers $headers
    Write-Host "Countdown $Id removed."
}
