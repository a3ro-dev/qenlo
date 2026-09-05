[CmdletBinding()]
param(
    [ValidateSet('DryRun', 'Pilot', 'Matrix', 'Deep', 'Deep768', 'All')]
    [string]$Stage = 'DryRun',
    [decimal]$AbsoluteBudgetUsd = 5.00,
    [decimal]$ReserveUsd = 1.00,
    [decimal]$OperationalCeilingUsd = 4.00,
    [string]$OnlyConfiguration,
    [switch]$Resume
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($AbsoluteBudgetUsd -gt 5 -or $ReserveUsd -lt 1 -or
    $OperationalCeilingUsd -gt ($AbsoluteBudgetUsd - $ReserveUsd)) {
    throw 'Budget policy requires an absolute ceiling <= $5, reserve >= $1, and operational ceiling <= ceiling minus reserve.'
}

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$campaign = Join-Path $repo 'research\artifacts\runpod-small-2026-09-05'
$runpodctl = Join-Path $repo '.tools\runpodctl-2.12.0\runpodctl-windows-amd64.exe'
$configPath = Join-Path $repo 'research\runpod\small-collection-configurations.json'
$currentBundle = Join-Path $campaign 'current-source.tar.gz'
$baselineBundle = Join-Path $campaign 'baseline-source.tar.gz'
$deepWorkload = Join-Path $repo 'scripts\runpod\deep_small_collection.sh'
$deep768Workload = Join-Path $repo 'scripts\runpod\deep_768_supplement.sh'
$realDataset = Join-Path $repo 'data\ag-news\ag-news-100k-384.qnb'
$ledger = Join-Path $campaign 'ledger.jsonl'
$midnightIst = [DateTimeOffset]::Parse('2026-09-05T00:00:00+05:30').ToUniversalTime()
$baselineRevision = 'b22d3b5033cfb578508a4cb76a8022dd3e3e258b'
$script:reservedExposure = [decimal]0
New-Item -ItemType Directory -Force $campaign | Out-Null

function Write-Ledger([hashtable]$Record) {
    $Record.capturedUtc = (Get-Date).ToUniversalTime().ToString('o')
    $Record | ConvertTo-Json -Depth 8 -Compress | Add-Content -LiteralPath $ledger
}

function Invoke-RunpodctlJson([string[]]$Arguments) {
    $raw = (& $runpodctl @Arguments -o json) -join "`n"
    if ($LASTEXITCODE -ne 0) { throw "runpodctl failed: $($Arguments -join ' '): $raw" }
    if (-not $raw) { return $null }
    return $raw | ConvertFrom-Json
}

function Get-RecordAmount($Record) {
    if ($null -eq $Record) { return [decimal]0 }
    if ($Record -is [array]) {
        $sum = [decimal]0
        foreach ($item in $Record) { $sum += Get-RecordAmount $item }
        return $sum
    }
    foreach ($name in @('totalAmount', 'amount', 'cost')) {
        $property = $Record.PSObject.Properties[$name]
        if ($property -and $null -ne $property.Value) { return [decimal]$property.Value }
    }
    $componentSum = [decimal]0
    foreach ($property in $Record.PSObject.Properties) {
        if ($property.Name -match '^(pod|serverless|endpoint|storage|cluster|network).+Amount$' -and
            $null -ne $property.Value) {
            $componentSum += [decimal]$property.Value
        }
    }
    return $componentSum
}

function Capture-Billing([string]$Label) {
    $start = $midnightIst.ToString('o')
    $total = [decimal]0
    foreach ($scope in @('pods', 'serverless', 'network-volume')) {
        $arguments = if ($scope -eq 'network-volume') {
            @('billing', 'network-volume', '--start-time', $start)
        } else {
            @('billing', $scope, '--start-time', $start)
        }
        try {
            $records = Invoke-RunpodctlJson $arguments
            $records | ConvertTo-Json -Depth 12 -Compress |
                Add-Content -LiteralPath (Join-Path $campaign "billing-$scope-$Label.jsonl")
            $total += Get-RecordAmount $records
        } catch {
            Write-Ledger @{ event = 'billing_capture_failed'; scope = $scope; label = $Label; error = $_.Exception.Message }
            throw
        }
    }
    Write-Ledger @{ event = 'billing_capture'; label = $Label; spendSinceMidnightIstUsd = $total }
    return $total
}

function New-Bundles {
    $manifestPath = Join-Path $campaign 'source-manifest.json'
    if ($Resume -and (Test-Path -LiteralPath $currentBundle) -and
        (Test-Path -LiteralPath $baselineBundle) -and (Test-Path -LiteralPath $manifestPath)) {
        $manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
        $currentSha = (Get-FileHash -LiteralPath $currentBundle -Algorithm SHA256).Hash.ToLowerInvariant()
        $baselineSha = (Get-FileHash -LiteralPath $baselineBundle -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($currentSha -ne $manifest.currentSourceBundleSha256 -or
            $baselineSha -ne $manifest.baselineSourceBundleSha256) {
            throw 'Saved source bundles no longer match their campaign manifest.'
        }
        return @{ Current = $currentSha; Baseline = $baselineSha }
    }
    $inputList = Join-Path $campaign 'current-input-files.txt'
    $files = @(& git -C $repo ls-files --cached --modified --others --exclude-standard) |
        Where-Object {
            $_ -notmatch '^(\.tools/|target/|tmp/|research/artifacts/)' -and
            $_ -ne 'smth-cool.md'
        } | Sort-Object -Unique
    if (@($files).Count -eq 0) { throw 'No source files selected for the current bundle.' }
    $files | Set-Content -LiteralPath $inputList -Encoding utf8NoBOM
    & tar -czf $currentBundle -C $repo -T $inputList
    if ($LASTEXITCODE -ne 0) { throw 'Could not create current source bundle.' }
    & git -C $repo archive --format=tar.gz --output=$baselineBundle $baselineRevision
    if ($LASTEXITCODE -ne 0) { throw 'Could not create frozen baseline bundle.' }
    $currentSha = (Get-FileHash -LiteralPath $currentBundle -Algorithm SHA256).Hash.ToLowerInvariant()
    $baselineSha = (Get-FileHash -LiteralPath $baselineBundle -Algorithm SHA256).Hash.ToLowerInvariant()
    @{
        currentSourceBundleSha256 = $currentSha
        baselineSourceBundleSha256 = $baselineSha
        baselineGitRevision = $baselineRevision
        currentGitRevision = (& git -C $repo rev-parse HEAD).Trim()
        currentWorktreeDirty = [bool](& git -C $repo status --porcelain)
    } | ConvertTo-Json | Set-Content -LiteralPath $manifestPath
    return @{ Current = $currentSha; Baseline = $baselineSha }
}

function Get-SshFields([string]$PodId) {
    $info = Invoke-RunpodctlJson @('ssh', 'info', $PodId)
    if (-not $info.ip -or -not $info.port -or -not $info.ssh_key.path) {
        throw "SSH is not ready for $PodId."
    }
    return @{ Host = [string]$info.ip; Port = [string]$info.port; Key = [string]$info.ssh_key.path }
}

function Wait-SshFields([string]$PodId, [datetime]$DeadlineUtc) {
    do {
        try { return Get-SshFields $PodId } catch {
            if ((Get-Date).ToUniversalTime() -ge $DeadlineUtc) { throw }
            Start-Sleep -Seconds 5
        }
    } while ($true)
}

function Copy-ToPod([hashtable]$Ssh, [string]$LocalPath, [string]$RemotePath) {
    & scp -q -o StrictHostKeyChecking=no -o UserKnownHostsFile=NUL -P $Ssh.Port -i $Ssh.Key `
        $LocalPath "root@$($Ssh.Host):$RemotePath"
    if ($LASTEXITCODE -ne 0) { throw "Upload failed: $LocalPath" }
}

function Copy-FromPod([hashtable]$Ssh, [string]$RemotePath, [string]$LocalPath) {
    & scp -q -o StrictHostKeyChecking=no -o UserKnownHostsFile=NUL -P $Ssh.Port -i $Ssh.Key `
        "root@$($Ssh.Host):$RemotePath" $LocalPath
    if ($LASTEXITCODE -ne 0) { throw "Download failed: $RemotePath" }
}

function Remove-CampaignPod([string]$PodId) {
    if ($PodId -notmatch '^[A-Za-z0-9_-]+$') { throw 'Refusing to delete an invalid pod ID.' }
    for ($attempt = 1; $attempt -le 3; $attempt++) {
        & $runpodctl pod delete $PodId -o json *> $null
        & $runpodctl pod get $PodId -o json *> $null
        if ($LASTEXITCODE -ne 0) {
            Write-Ledger @{ event = 'pod_deleted_verified'; podId = $PodId; attempt = $attempt }
            return
        }
        Start-Sleep -Seconds 5
    }
    throw "Pod deletion could not be verified: $PodId"
}

function Start-DeletionWatchdog([string]$PodId, [datetime]$DeadlineUtc) {
    Start-Process pwsh -WindowStyle Hidden -ArgumentList @(
        '-NoProfile', '-File', (Join-Path $repo 'scripts\runpod\watchdog.ps1'),
        '-PodId', $PodId, '-DeadlineUtc', $DeadlineUtc.ToString('o'),
        '-Runpodctl', $runpodctl, '-Ledger', $ledger
    ) | Out-Null
}

function Get-LiveRate([pscustomobject]$Configuration, $Catalog) {
    $gpu = @($Catalog | Where-Object gpuId -eq $Configuration.gpuId)
    if ($gpu.Count -ne 1) { throw "GPU is absent from the live catalog: $($Configuration.gpuId)" }
    $property = if ($Configuration.cloud -eq 'SECURE') { 'securePricePerHr' } else { 'communityPricePerHr' }
    $rate = $gpu[0].$property
    if ($null -eq $rate) { throw "No live $($Configuration.cloud) price for $($Configuration.gpuId)." }
    return [decimal]$rate
}

function Invoke-RemoteMonitored(
    [hashtable]$Ssh, [string]$PodId, [string]$Command,
    [datetime]$DeadlineUtc, [string]$ResultDirectory
) {
    $arguments = @(
        '-o', 'StrictHostKeyChecking=no', '-o', 'UserKnownHostsFile=NUL',
        '-p', $Ssh.Port, '-i', $Ssh.Key, "root@$($Ssh.Host)", $Command
    )
    $process = Start-Process ssh -WindowStyle Hidden -PassThru -ArgumentList $arguments `
        -RedirectStandardOutput (Join-Path $ResultDirectory 'remote.stdout.log') `
        -RedirectStandardError (Join-Path $ResultDirectory 'remote.stderr.log')
    while (-not $process.HasExited) {
        if ((Get-Date).ToUniversalTime() -ge $DeadlineUtc) {
            Remove-CampaignPod $PodId
            $process.Kill($true)
            throw "Pod $PodId reached its hard deadline."
        }
        (& $runpodctl pod get $PodId -o json) |
            Add-Content -LiteralPath (Join-Path $ResultDirectory 'pod-polls.jsonl')
        $spend = Capture-Billing "poll-$PodId"
        if ($spend + $script:reservedExposure -gt $OperationalCeilingUsd) {
            Remove-CampaignPod $PodId
            $process.Kill($true)
            throw 'Actual spend plus remaining reserved exposure crossed the operational ceiling.'
        }
        Start-Sleep -Seconds 30
        $process.Refresh()
    }
    return $process.ExitCode
}

function Invoke-Configuration(
    [pscustomobject]$Configuration, [string]$Mode, [int]$DeadlineMinutes,
    [hashtable]$Digests, $Catalog, [decimal]$SpendAtStart
) {
    $resultDir = Join-Path $campaign "$Mode\$($Configuration.name)"
    New-Item -ItemType Directory -Force $resultDir | Out-Null
    if ($Resume -and (Test-Path -LiteralPath (Join-Path $resultDir 'complete.json'))) { return $true }
    $rate = Get-LiveRate $Configuration $Catalog
    $maximumCost = $rate * [decimal]$DeadlineMinutes / 60
    if ($SpendAtStart + $script:reservedExposure + $maximumCost -gt $OperationalCeilingUsd) {
        @{ status = 'not_attempted'; reason = 'worst_case_budget_guard'; maximumCostUsd = $maximumCost } |
            ConvertTo-Json | Set-Content -LiteralPath (Join-Path $resultDir 'outcome.json')
        return $false
    }
    $script:reservedExposure += $maximumCost
    $podId = $null
    $created = $null
    try {
        $arguments = @(
            'pod', 'create', '--name', "qenlo-small-$($Configuration.name)",
            '--image', 'runpod/pytorch:1.0.3-cu1281-torch291-ubuntu2404',
            '--gpu-id', $Configuration.gpuId, '--data-center-ids', $Configuration.dataCenter,
            '--cloud-type', $Configuration.cloud, '--container-disk-in-gb', '30',
            '--volume-in-gb', '0', '--ports', '22/tcp',
            '--env', '{"NVIDIA_DRIVER_CAPABILITIES":"compute,utility,graphics","NVIDIA_VISIBLE_DEVICES":"all"}'
        )
        if ($Configuration.cloud -eq 'COMMUNITY') { $arguments += '--public-ip' }
        $created = Invoke-RunpodctlJson $arguments
        $podId = [string]$created.id
        if (-not $podId) { throw 'Pod creation returned no ID.' }
        Write-Ledger @{
            event = 'pod_created'; podId = $podId; configuration = $Configuration.name;
            mode = $Mode; liveUsdPerHour = $rate; maximumCostUsd = $maximumCost
        }
        $deadline = (Get-Date).ToUniversalTime().AddMinutes($DeadlineMinutes)
        Start-DeletionWatchdog $podId $deadline
        $ssh = Wait-SshFields $podId ((Get-Date).ToUniversalTime().AddMinutes(5))
        Copy-ToPod $ssh $currentBundle '/workspace/current-source.tar.gz'
        if ($Mode -ne 'deep768') {
            Copy-ToPod $ssh $baselineBundle '/workspace/baseline-source.tar.gz'
        }
        if ($Mode -eq 'deep') {
            Copy-ToPod $ssh $deepWorkload '/workspace/deep_small_collection.sh'
            Copy-ToPod $ssh $realDataset '/workspace/ag-news-100k-384.qnb'
            $remote = "mkdir -p /workspace/qenlo-deep/current /workspace/qenlo-deep/baseline; " +
                "echo '$($Digests.Current)  /workspace/current-source.tar.gz' | sha256sum -c -; " +
                "echo '$($Digests.Baseline)  /workspace/baseline-source.tar.gz' | sha256sum -c -; " +
                "tar -xzf /workspace/current-source.tar.gz -C /workspace/qenlo-deep/current; " +
                "tar -xzf /workspace/baseline-source.tar.gz -C /workspace/qenlo-deep/baseline; " +
                "export QENLO_SOURCE_BUNDLE_SHA256=$($Digests.Current) QENLO_BASELINE_BUNDLE_SHA256=$($Digests.Baseline) " +
                "QENLO_WORKLOAD_SCRIPT_SHA256=$($Digests.Workload) QENLO_REAL_DATASET_SHA256=$($Digests.Real); " +
                "bash /workspace/deep_small_collection.sh; " +
                "code=`$?; tar -czf /workspace/qenlo-small-artifacts.tar.gz -C /workspace/qenlo-deep artifacts; exit `$code"
        } elseif ($Mode -eq 'deep768') {
            Copy-ToPod $ssh $deep768Workload '/workspace/deep_768_supplement.sh'
            $remote = "mkdir -p /workspace/qenlo-deep768/current; " +
                "echo '$($Digests.Current)  /workspace/current-source.tar.gz' | sha256sum -c -; " +
                "tar -xzf /workspace/current-source.tar.gz -C /workspace/qenlo-deep768/current; " +
                "export QENLO_SOURCE_BUNDLE_SHA256=$($Digests.Current) QENLO_WORKLOAD_SCRIPT_SHA256=$($Digests.Deep768); " +
                "bash /workspace/deep_768_supplement.sh; " +
                "code=`$?; tar -czf /workspace/qenlo-small-artifacts.tar.gz -C /workspace/qenlo-deep768 artifacts; exit `$code"
        } else {
            $remote = "mkdir -p /workspace/qenlo-small/current /workspace/qenlo-small/baseline; " +
                "tar -xzf /workspace/current-source.tar.gz -C /workspace/qenlo-small/current; " +
                "tar -xzf /workspace/baseline-source.tar.gz -C /workspace/qenlo-small/baseline; " +
                "export QENLO_SOURCE_BUNDLE_SHA256=$($Digests.Current) QENLO_BASELINE_BUNDLE_SHA256=$($Digests.Baseline); " +
                "bash /workspace/qenlo-small/current/scripts/runpod/small_collection_campaign.sh $Mode; " +
                "code=`$?; tar -czf /workspace/qenlo-small-artifacts.tar.gz -C /workspace/qenlo-small artifacts; exit `$code"
        }
        $exitCode = Invoke-RemoteMonitored $ssh $podId $remote $deadline $resultDir
        Copy-FromPod $ssh '/workspace/qenlo-small-artifacts.tar.gz' (Join-Path $resultDir 'artifacts.tar.gz')
        if ($exitCode -ne 0) { throw "Remote workload exited with code $exitCode." }
        @{
            status = 'complete'; podId = $podId; mode = $Mode; configuration = $Configuration;
            liveUsdPerHour = $rate; maximumCostUsd = $maximumCost; sourceDigests = $Digests
        } | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $resultDir 'complete.json')
        return $true
    } catch {
        @{ status = 'failed_or_unavailable'; error = $_.Exception.Message; podId = $podId;
           configuration = $Configuration; createResponse = $created } |
            ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $resultDir 'outcome.json')
        Write-Ledger @{ event = 'configuration_failed'; podId = $podId; configuration = $Configuration.name; error = $_.Exception.Message }
        return $false
    } finally {
        $script:reservedExposure -= $maximumCost
        if ($podId) { Remove-CampaignPod $podId }
    }
}

if (-not (Test-Path -LiteralPath $runpodctl)) { throw 'Pinned runpodctl 2.12.0 is missing.' }
$version = (& $runpodctl --version) -join "`n"
if ($LASTEXITCODE -ne 0 -or $version -notmatch '2\.12\.0') { throw 'Pinned runpodctl validation failed.' }
& $runpodctl doctor | Set-Content -LiteralPath (Join-Path $campaign 'runpodctl-doctor.json')
if ($LASTEXITCODE -ne 0) { throw 'runpodctl doctor failed.' }
$activePods = @(Invoke-RunpodctlJson @('pod', 'list'))
$activePods | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $campaign 'preexisting-pods.json')
if ($activePods.Count -ne 0) { throw 'Refusing to launch while any pre-existing pod is active.' }
$catalog = @(Invoke-RunpodctlJson @('gpu', 'list', '--include-unavailable'))
$catalog | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $campaign 'gpu-catalog.json')
$configurations = @(Get-Content -LiteralPath $configPath -Raw | ConvertFrom-Json)
$digests = New-Bundles
$digests.Workload = (Get-FileHash -LiteralPath $deepWorkload -Algorithm SHA256).Hash.ToLowerInvariant()
$digests.Deep768 = (Get-FileHash -LiteralPath $deep768Workload -Algorithm SHA256).Hash.ToLowerInvariant()
$digests.Real = (Get-FileHash -LiteralPath $realDataset -Algorithm SHA256).Hash.ToLowerInvariant()
$spendAtStart = Capture-Billing 'preflight'
if ($spendAtStart -gt $OperationalCeilingUsd) { throw 'Today''s existing spend already exceeds the operational ceiling.' }

if ($Stage -eq 'DryRun') {
    @{ status = 'dry-run-complete'; spendSinceMidnightIstUsd = $spendAtStart;
       operationalAllowanceUsd = $OperationalCeilingUsd - $spendAtStart; reserveUsd = $ReserveUsd;
       configurations = $configurations.Count; sourceDigests = $digests } |
        ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $campaign 'dry-run.json')
    return
}

$pilot = @($configurations | Where-Object role -eq 'pilot')
if ($pilot.Count -ne 1) { throw 'Exactly one pilot configuration is required.' }
$pilotComplete = Test-Path -LiteralPath (Join-Path $campaign "pilot\$($pilot[0].name)\complete.json")
if ($Stage -in @('Pilot', 'All')) {
    $pilotComplete = Invoke-Configuration $pilot[0] pilot 20 $digests $catalog $spendAtStart
    if (-not $pilotComplete) { throw 'Pilot failed; matrix expansion is stopped.' }
}
if ($Stage -in @('Matrix', 'All')) {
    if (-not $pilotComplete) { throw 'A completed pilot is required before matrix expansion.' }
    foreach ($configuration in $configurations) {
        if ($configuration.role -in @('pilot', 'deep', 'deep768')) { continue }
        if ($OnlyConfiguration -and $configuration.name -ne $OnlyConfiguration) { continue }
        $mode = if ($configuration.role -eq 'reference') { 'reference' } else { 'common' }
        $minutes = if ($mode -eq 'reference') { 50 } else { 35 }
        $currentSpend = Capture-Billing "before-$($configuration.name)"
        [void](Invoke-Configuration $configuration $mode $minutes $digests $catalog $currentSpend)
    }
}
if ($Stage -in @('Deep768', 'All')) {
    if (-not $pilotComplete) { throw 'A completed pilot is required before the 768-dimensional supplement.' }
    $deep768 = @($configurations | Where-Object {
        $_.role -eq 'deep768' -and (-not $OnlyConfiguration -or $_.name -eq $OnlyConfiguration)
    })
    if ($deep768.Count -ne 1) { throw 'Exactly one deep768 configuration is required.' }
    $currentSpend = Capture-Billing "before-$($deep768[0].name)"
    [void](Invoke-Configuration $deep768[0] deep768 35 $digests $catalog $currentSpend)
}
if ($Stage -in @('Deep', 'All')) {
    if (-not $pilotComplete) { throw 'A completed pilot is required before the deep reference run.' }
    $deep = @($configurations | Where-Object role -eq 'deep')
    if ($deep.Count -ne 1) { throw 'Exactly one deep configuration is required.' }
    if (-not $OnlyConfiguration -or $deep[0].name -eq $OnlyConfiguration) {
        $currentSpend = Capture-Billing "before-$($deep[0].name)"
        [void](Invoke-Configuration $deep[0] deep 60 $digests $catalog $currentSpend)
    }
}

$remaining = @(Invoke-RunpodctlJson @('pod', 'list'))
if ($remaining.Count -ne 0) { throw 'Final cleanup failed: at least one pod remains.' }
$finalSpend = Capture-Billing 'final'
@{
    status = 'finished'; startedSpendUsd = $spendAtStart; finalKnownSpendUsd = $finalSpend;
    billingDelayPossible = $true; absoluteBudgetUsd = $AbsoluteBudgetUsd;
    operationalCeilingUsd = $OperationalCeilingUsd; reserveUsd = $ReserveUsd;
    campaignPodsRemaining = 0; finishedUtc = (Get-Date).ToUniversalTime().ToString('o')
} | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $campaign 'campaign-summary.json')
