[CmdletBinding()]
param(
    [ValidateSet('compatibility', 'reference', 'constrained', 'all')]
    [string]$Stage = 'all',
    [decimal]$OperationalBudgetUsd = 5.00,
    [decimal]$AbsoluteBudgetUsd = 6.00,
    [string]$OnlyConfiguration,
    [switch]$Resume
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($OperationalBudgetUsd -ge $AbsoluteBudgetUsd -or $AbsoluteBudgetUsd -gt 6.00) {
    throw 'The operational budget must be below the absolute ceiling, which may not exceed $6.00.'
}

$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$campaign = Join-Path $repo 'research\artifacts\runpod-campaign-v2'
$toolDir = Join-Path $repo '.tools\runpodctl-2.12.0'
$runpodctl = Join-Path $toolDir 'runpodctl-windows-amd64.exe'
$bundle = Join-Path $campaign 'qenlo-campaign-input.tar.gz'
$configPath = Join-Path $repo 'research\runpod\configurations.json'
$startedUtc = (Get-Date).ToUniversalTime()
$script:estimatedExposure = [decimal]0
New-Item -ItemType Directory -Force $campaign, $toolDir | Out-Null

function Install-Runpodctl {
    $expected = 'f434915e19632097c0ec89d48fac3e25af187e14ee3d172dc37e4d5b2154a7f3'
    if (-not (Test-Path -LiteralPath $runpodctl)) {
        Invoke-WebRequest -UseBasicParsing `
            'https://github.com/runpod/runpodctl/releases/download/v2.12.0/runpodctl-windows-amd64.exe' `
            -OutFile $runpodctl
    }
    $actual = (Get-FileHash -LiteralPath $runpodctl -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expected) { throw 'runpodctl checksum mismatch.' }
    $version = & $runpodctl --version
    if ($LASTEXITCODE -ne 0 -or $version -notmatch '2\.12\.0') {
        throw 'runpodctl 2.12.0 validation failed.'
    }
}

function Invoke-RunpodctlJson([string[]]$Arguments) {
    $raw = & $runpodctl @Arguments -o json
    if ($LASTEXITCODE -ne 0) { throw "runpodctl failed: $($Arguments -join ' ')" }
    return ($raw | ConvertFrom-Json)
}

function Remove-CampaignPod([string]$PodId) {
    if ($PodId -match '^[A-Za-z0-9_-]+$') {
        & $runpodctl pod delete $PodId -o json *> $null
    }
}

function Start-DeletionWatchdog([string]$PodId, [datetime]$DeadlineUtc) {
    $watchdog = Join-Path $repo 'scripts\runpod\watchdog.ps1'
    Start-Process pwsh -WindowStyle Hidden -ArgumentList @(
        '-NoProfile', '-File', $watchdog, '-PodId', $PodId,
        '-DeadlineUtc', $DeadlineUtc.ToString('o'), '-Runpodctl', $runpodctl
    ) | Out-Null
}

function Get-SshFields([string]$PodId) {
    $json = (& $runpodctl ssh info $PodId -o json) -join "`n"
    if ($LASTEXITCODE -ne 0) { throw "Could not read SSH details for $PodId." }
    $info = $json | ConvertFrom-Json
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
    & scp -q -o StrictHostKeyChecking=no -o UserKnownHostsFile=NUL `
        -P $Ssh.Port -i $Ssh.Key $LocalPath "root@$($Ssh.Host):$RemotePath"
    if ($LASTEXITCODE -ne 0) { throw "Upload failed: $LocalPath" }
}

function Copy-FromPod([hashtable]$Ssh, [string]$RemotePath, [string]$LocalPath) {
    & scp -q -o StrictHostKeyChecking=no -o UserKnownHostsFile=NUL `
        -P $Ssh.Port -i $Ssh.Key "root@$($Ssh.Host):$RemotePath" $LocalPath
    if ($LASTEXITCODE -ne 0) { throw "Download failed: $RemotePath" }
}

function Invoke-RemoteMonitored(
    [hashtable]$Ssh,
    [string]$PodId,
    [string]$Command,
    [datetime]$DeadlineUtc,
    [string]$LogDirectory
) {
    $stdout = Join-Path $LogDirectory 'remote.stdout.log'
    $stderr = Join-Path $LogDirectory 'remote.stderr.log'
    $arguments = @(
        '-o', 'StrictHostKeyChecking=no', '-o', 'UserKnownHostsFile=NUL',
        '-p', $Ssh.Port, '-i', $Ssh.Key, "root@$($Ssh.Host)", $Command
    )
    $process = Start-Process ssh -WindowStyle Hidden -PassThru -ArgumentList $arguments `
        -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    while (-not $process.HasExited) {
        if ((Get-Date).ToUniversalTime() -ge $DeadlineUtc -or $script:estimatedExposure -ge $OperationalBudgetUsd) {
            Remove-CampaignPod $PodId
            $process.Kill($true)
            throw "Pod $PodId reached its deadline or campaign budget guard."
        }
        (& $runpodctl pod get $PodId -o json) | Add-Content (Join-Path $LogDirectory 'pod-poll.jsonl')
        (& $runpodctl billing pods --start-time $startedUtc.ToString('o') --grouping podId -o json) |
            Add-Content (Join-Path $campaign 'billing-polls.jsonl')
        Start-Sleep -Seconds 30
        $process.Refresh()
    }
    if ($process.ExitCode -ne 0) { throw "Remote stage failed with exit code $($process.ExitCode)." }
}

function New-Bundle {
    if ($Resume -and $Stage -eq 'compatibility' -and (Test-Path -LiteralPath $bundle)) { return }
    $list = Join-Path $campaign 'input-files.txt'
    $files = @(& git -C $repo ls-files --cached --modified --others --exclude-standard) |
        Where-Object {
            $_ -match '^(Cargo\.(toml|lock)|crates/|scripts/|research/scripts/|research/runpod/|paper/|docs/|README\.md|apps/.*/Cargo\.toml$)'
        }
    if ($Stage -in @('reference', 'constrained', 'all')) {
        $files += 'data/ag-news/ag-news-100k-384.qnb'
    }
    $files | Sort-Object -Unique | Set-Content -Encoding utf8NoBOM $list
    & tar -czf $bundle -C $repo -T $list
    if ($LASTEXITCODE -ne 0) { throw 'Could not package the working tree.' }
    Get-FileHash -LiteralPath $bundle -Algorithm SHA256 |
        Format-List | Out-File (Join-Path $campaign 'input.sha256.txt')
}

function Invoke-Compatibility([pscustomobject]$Configuration) {
    $resultDir = Join-Path $campaign "compatibility\$($Configuration.name)"
    New-Item -ItemType Directory -Force $resultDir | Out-Null
    if ($Resume -and (Test-Path (Join-Path $resultDir 'complete.json'))) { return }
    $deadlineMinutes = 8
    $maximumCost = [decimal]$Configuration.price * $deadlineMinutes / 60
    if ($script:estimatedExposure + $maximumCost -gt 1.00 -or $script:estimatedExposure + $maximumCost -ge $OperationalBudgetUsd) {
        @{ status = 'not_attempted'; reason = 'stage_budget_guard'; maximumCost = $maximumCost } |
            ConvertTo-Json | Set-Content (Join-Path $resultDir 'outcome.json')
        return
    }
    $script:estimatedExposure += $maximumCost
    $podId = $null
    try {
        $isAmd = $null -ne $Configuration.PSObject.Properties['amd'] -and $Configuration.amd
        $image = if ($isAmd) { 'rocm/dev-ubuntu-24.04:6.4-complete' } else { 'runpod/pytorch:1.0.3-cu1281-torch291-ubuntu2404' }
        $args = @(
            'pod', 'create', '--name', "qenlo-$($Configuration.name)",
            '--image', $image, '--gpu-id', $Configuration.gpuId,
            '--data-center-ids', $Configuration.dataCenter, '--cloud-type', $Configuration.cloud,
            '--container-disk-in-gb', '30', '--volume-in-gb', '0', '--ports', '22/tcp',
            '--env', '{"NVIDIA_DRIVER_CAPABILITIES":"compute,utility,graphics","NVIDIA_VISIBLE_DEVICES":"all"}'
        )
        if ($Configuration.cloud -eq 'COMMUNITY') { $args += '--public-ip' }
        $pod = Invoke-RunpodctlJson $args
        $podId = [string]$pod.id
        if (-not $podId) { throw 'Pod creation returned no id.' }
        $deadline = (Get-Date).ToUniversalTime().AddMinutes($deadlineMinutes)
        Start-DeletionWatchdog $podId $deadline
        $readyDeadline = (Get-Date).ToUniversalTime().AddMinutes(3)
        if ($readyDeadline -gt $deadline) { $readyDeadline = $deadline }
        $ssh = Wait-SshFields $podId $readyDeadline
        Copy-ToPod $ssh $bundle '/workspace/qenlo-campaign-input.tar.gz'
        $remote = "mkdir -p /workspace/qenlo-campaign/repo && tar -xzf /workspace/qenlo-campaign-input.tar.gz -C /workspace/qenlo-campaign/repo && bash /workspace/qenlo-campaign/repo/scripts/runpod/bootstrap.sh compatibility; code=`$?; tar -C /workspace/qenlo-campaign -czf /workspace/compatibility-artifacts.tar.gz artifacts; exit `$code"
        Invoke-RemoteMonitored $ssh $podId $remote $deadline $resultDir
        Copy-FromPod $ssh '/workspace/compatibility-artifacts.tar.gz' (Join-Path $resultDir 'artifacts.tar.gz')
        @{ status = 'complete'; podId = $podId; configuration = $Configuration } |
            ConvertTo-Json -Depth 5 | Set-Content (Join-Path $resultDir 'complete.json')
    } catch {
        @{ status = 'failed_or_unavailable'; error = $_.Exception.Message; configuration = $Configuration } |
            ConvertTo-Json -Depth 5 | Set-Content (Join-Path $resultDir 'outcome.json')
    } finally {
        if ($podId) { Remove-CampaignPod $podId }
    }
}

function Invoke-Reference {
    $resultDir = Join-Path $campaign 'reference-host'
    New-Item -ItemType Directory -Force $resultDir | Out-Null
    if ($Resume -and (Test-Path (Join-Path $resultDir 'complete.json'))) { return }
    $completed = Get-ChildItem (Join-Path $campaign 'compatibility') -Filter complete.json -Recurse -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if (-not $completed) { throw 'Reference stage requires one Vulkan-compatible completed configuration.' }
    $record = Get-Content -Raw $completed.FullName | ConvertFrom-Json
    $configuration = $record.configuration
    $stageBudget = [decimal]3.00
    $hours = [math]::Min(8.0, [double]($stageBudget / [decimal]$configuration.price))
    $maximumCost = [decimal]$configuration.price * [decimal]$hours
    if ($script:estimatedExposure + $maximumCost -ge $OperationalBudgetUsd) {
        throw 'Reference host reservation would cross the operational budget.'
    }
    $script:estimatedExposure += $maximumCost
    $podId = $null
    try {
        $args = @(
            'pod', 'create', '--name', 'qenlo-reference',
            '--image', 'runpod/pytorch:1.0.3-cu1281-torch291-ubuntu2404',
            '--gpu-id', $configuration.gpuId, '--data-center-ids', $configuration.dataCenter,
            '--cloud-type', $configuration.cloud, '--container-disk-in-gb', '40',
            '--volume-in-gb', '0', '--ports', '22/tcp',
            '--env', '{"NVIDIA_DRIVER_CAPABILITIES":"compute,utility,graphics","NVIDIA_VISIBLE_DEVICES":"all"}'
        )
        if ($configuration.cloud -eq 'COMMUNITY') { $args += '--public-ip' }
        $pod = Invoke-RunpodctlJson $args
        $podId = [string]$pod.id
        $deadline = (Get-Date).ToUniversalTime().AddHours($hours)
        Start-DeletionWatchdog $podId $deadline
        $readyDeadline = (Get-Date).ToUniversalTime().AddMinutes(5)
        $ssh = Wait-SshFields $podId $readyDeadline
        Copy-ToPod $ssh $bundle '/workspace/qenlo-campaign-input.tar.gz'
        $remote = "mkdir -p /workspace/qenlo-campaign/repo && tar -xzf /workspace/qenlo-campaign-input.tar.gz -C /workspace/qenlo-campaign/repo && bash /workspace/qenlo-campaign/repo/scripts/runpod/reference_campaign.sh; code=`$?; tar -C /workspace/qenlo-campaign -czf /workspace/reference-artifacts.tar.gz artifacts; exit `$code"
        Invoke-RemoteMonitored $ssh $podId $remote $deadline $resultDir
        Copy-FromPod $ssh '/workspace/reference-artifacts.tar.gz' (Join-Path $resultDir 'artifacts.tar.gz')
        @{ status = 'complete'; podId = $podId; configuration = $configuration } |
            ConvertTo-Json -Depth 5 | Set-Content (Join-Path $resultDir 'complete.json')
    } catch {
        if ($podId) {
            try {
                $ssh = Get-SshFields $podId
                Copy-FromPod $ssh '/workspace/reference-artifacts.tar.gz' (Join-Path $resultDir 'partial-artifacts.tar.gz')
            } catch {}
        }
        @{ status = 'failed'; error = $_.Exception.Message; configuration = $configuration } |
            ConvertTo-Json -Depth 5 | Set-Content (Join-Path $resultDir 'outcome.json')
        throw
    } finally {
        if ($podId) { Remove-CampaignPod $podId }
    }
}

Install-Runpodctl
if (-not (Test-Path -LiteralPath (Join-Path $HOME '.runpod\config.toml'))) {
    throw 'Runpod credential file is missing.'
}
& $runpodctl doctor | Out-File (Join-Path $campaign 'runpodctl-doctor.json')
$existing = Invoke-RunpodctlJson @('pod', 'list')
if (@($existing).Count -ne 0) { throw 'Refusing to start while another Runpod pod is active.' }
Invoke-RunpodctlJson @('gpu', 'list', '--include-unavailable') |
    ConvertTo-Json -Depth 8 | Set-Content (Join-Path $campaign 'gpu-catalog.json')
New-Bundle

if ($Stage -in @('compatibility', 'all')) {
    foreach ($configuration in (Get-Content -Raw $configPath | ConvertFrom-Json)) {
        if ($OnlyConfiguration -and $configuration.name -ne $OnlyConfiguration) { continue }
        Invoke-Compatibility $configuration
    }
}
if ($Stage -in @('reference', 'all')) { Invoke-Reference }
if ($Stage -eq 'constrained') {
    throw 'The constrained stage has no implementation yet; no pod was provisioned.'
}

$remaining = Invoke-RunpodctlJson @('pod', 'list')
if (@($remaining).Count -ne 0) { throw 'Final cleanup failed: one or more pods remain.' }
@{
    startedUtc = $startedUtc.ToString('o')
    finishedUtc = (Get-Date).ToUniversalTime().ToString('o')
    estimatedMaximumExposureUsd = $script:estimatedExposure
    operationalBudgetUsd = $OperationalBudgetUsd
    absoluteBudgetUsd = $AbsoluteBudgetUsd
    paidResourcesRemaining = 0
} | ConvertTo-Json | Set-Content (Join-Path $campaign 'campaign-summary.json')
