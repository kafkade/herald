<#
.SYNOPSIS
    Add a text message to the Herald board rotation.
.EXAMPLE
    .\add-message.ps1 -Text "HELLO WORLD"
    .\add-message.ps1 -Text "LEFT ALIGNED" -HAlign left
    .\add-message.ps1 -Token "mytoken" -Text "EXPIRES SOON" -ExpiresIn 300
.NOTES
    Uses $env:HERALD_ADMIN_TOKEN if -Token is not provided.
#>
param(
    [string]$Token,
    [Parameter(Mandatory)][string]$Text,
    [string]$Server = "http://localhost:3000",
    [ValidateSet("left","center","right")][string]$HAlign = "center",
    [ValidateSet("top","middle")][string]$VAlign = "middle",
    [int]$ExpiresIn  # seconds from now
)

if (-not $Token) { $Token = $env:HERALD_ADMIN_TOKEN }
if (-not $Token) { Write-Error "Provide -Token or set `$env:HERALD_ADMIN_TOKEN"; return }

$body = @{ text = $Text; h_align = $HAlign; v_align = $VAlign }
if ($ExpiresIn) {
    $body.expires_at = (Get-Date).AddSeconds($ExpiresIn).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
}

$result = Invoke-RestMethod -Uri "$Server/api/messages" -Method Post `
    -Headers @{ Authorization = "Bearer $Token" } `
    -ContentType "application/json" `
    -Body ($body | ConvertTo-Json -Depth 3)

Write-Host "Message added (id: $($result.id))"
$result
