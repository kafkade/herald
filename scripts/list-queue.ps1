<#
.SYNOPSIS
    List all messages and countdowns currently in the Herald rotation queue.
.EXAMPLE
    .\list-queue.ps1
    .\list-queue.ps1 -Server "http://myhost:3000"
.NOTES
    Uses $env:HERALD_ADMIN_TOKEN if -Token is not provided.
#>
param(
    [string]$Token,
    [string]$Server = "http://localhost:3000"
)

if (-not $Token) { $Token = $env:HERALD_ADMIN_TOKEN }
if (-not $Token) { Write-Error "Provide -Token or set `$env:HERALD_ADMIN_TOKEN"; return }

$headers = @{ Authorization = "Bearer $Token" }

$queue = Invoke-RestMethod -Uri "$Server/api/queue" -Method Get -Headers $headers

if ($queue.total -eq 0) {
    Write-Host "Queue is empty — the HERALD splash screen is showing."
    return
}

Write-Host "Queue ($($queue.total) items):`n"

$i = 1
foreach ($item in $queue.items) {
    $kind = $item.kind
    $label = $item.label
    $id = $item.id
    $expires = if ($item.expires_at) { " (expires $($item.expires_at))" } else { "" }

    if ($kind -eq "message") {
        Write-Host ("  {0,2}. [MSG]       {1}{2}" -f $i, $label, $expires)
    } elseif ($kind -eq "countdown") {
        Write-Host ("  {0,2}. [COUNTDOWN] {1}{2}" -f $i, $label, $expires)
    } else {
        Write-Host ("  {0,2}. [{1}] {2}{3}" -f $i, $kind.ToUpper(), $label, $expires)
    }
    Write-Host "      id: $id"
    $i++
}
