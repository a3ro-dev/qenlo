param(
    [Parameter(Mandatory = $true)][ValidatePattern('^[A-Za-z0-9_-]+$')][string]$PodId,
    [Parameter(Mandatory = $true)][datetime]$DeadlineUtc,
    [Parameter(Mandatory = $true)][string]$Runpodctl,
    [string]$Ledger
)
$remaining = $DeadlineUtc.ToUniversalTime() - (Get-Date).ToUniversalTime()
if ($remaining.TotalSeconds -gt 0) {
    Start-Sleep -Seconds ([math]::Ceiling($remaining.TotalSeconds))
}
$deleted = $false
for ($attempt = 1; $attempt -le 3 -and -not $deleted; $attempt++) {
    & $Runpodctl pod delete $PodId -o json *> $null
    & $Runpodctl pod get $PodId -o json *> $null
    $deleted = $LASTEXITCODE -ne 0
    if (-not $deleted) { Start-Sleep -Seconds 5 }
}
if ($Ledger) {
    @{
        event = 'watchdog_delete'
        podId = $PodId
        deleted = $deleted
        capturedUtc = (Get-Date).ToUniversalTime().ToString('o')
    } | ConvertTo-Json -Compress | Add-Content -LiteralPath $Ledger
}
if (-not $deleted) { exit 1 }
