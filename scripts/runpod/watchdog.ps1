param(
    [Parameter(Mandatory = $true)][ValidatePattern('^[A-Za-z0-9_-]+$')][string]$PodId,
    [Parameter(Mandatory = $true)][datetime]$DeadlineUtc,
    [Parameter(Mandatory = $true)][string]$Runpodctl
)
$remaining = $DeadlineUtc.ToUniversalTime() - (Get-Date).ToUniversalTime()
if ($remaining.TotalSeconds -gt 0) {
    Start-Sleep -Seconds ([math]::Ceiling($remaining.TotalSeconds))
}
& $Runpodctl pod delete $PodId *> $null
