<#
.SYNOPSIS
    Remove a message from the Herald board rotation.
.EXAMPLE
    .\remove-message.ps1 -Token "mytoken" -Id "some-uuid"
    .\remove-message.ps1 -Token "mytoken" -All   # remove all messages
#>
param(
    [Parameter(Mandatory)][string]$Token,
    [string]$Id,
    [switch]$All,
    [string]$Server = "http://localhost:3000"
)

if (-not $Id -and -not $All) {
    # List messages so the user can pick one
    $list = Invoke-RestMethod -Uri "$Server/api/messages" -Method Get `
        -Headers @{ Authorization = "Bearer $Token" }
    if ($list.total -eq 0) {
        Write-Host "No messages in rotation."
        return
    }
    Write-Host "Messages in rotation:"
    foreach ($m in $list.items) {
        $preview = if ($m.source_text) { $m.source_text } else { "(raw grid)" }
        Write-Host "  $($m.id)  $preview"
    }
    Write-Host "`nUse -Id <uuid> to remove a specific message, or -All to remove all."
    return
}

if ($All) {
    $list = Invoke-RestMethod -Uri "$Server/api/messages" -Method Get `
        -Headers @{ Authorization = "Bearer $Token" }
    foreach ($m in $list.items) {
        Invoke-RestMethod -Uri "$Server/api/messages/$($m.id)" -Method Delete `
            -Headers @{ Authorization = "Bearer $Token" } | Out-Null
        Write-Host "Deleted $($m.id)"
    }
    Write-Host "All $($list.total) message(s) removed."
} else {
    Invoke-RestMethod -Uri "$Server/api/messages/$Id" -Method Delete `
        -Headers @{ Authorization = "Bearer $Token" }
    Write-Host "Message $Id removed."
}
