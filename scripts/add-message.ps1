<#
.SYNOPSIS
    Add a text message to the Herald board rotation.
.EXAMPLE
    .\add-message.ps1 -Token "mytoken" -Text "HELLO WORLD"
    .\add-message.ps1 -Token "mytoken" -Text "LEFT ALIGNED" -HAlign left
    .\add-message.ps1 -Token "mytoken" -Text "EXPIRES SOON" -ExpiresIn 300
#>
param(
    [Parameter(Mandatory)][string]$Token,
    [Parameter(Mandatory)][string]$Text,
    [string]$Server = "http://localhost:3000",
    [ValidateSet("left","center","right")][string]$HAlign = "center",
    [ValidateSet("top","middle")][string]$VAlign = "middle",
    [int]$ExpiresIn  # seconds from now
)

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
