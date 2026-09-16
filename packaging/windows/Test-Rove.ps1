param([string]$DataDir = (Join-Path $PSScriptRoot 'test-data'))
$ErrorActionPreference = 'Stop'
$agent = $null
$agentArguments = @('--data-dir', ('"' + $DataDir + '"'))
try {
    $agent = Start-Process -FilePath (Join-Path $PSScriptRoot 'rove-agent.exe') -ArgumentList $agentArguments -PassThru -WindowStyle Hidden -RedirectStandardOutput (Join-Path $PSScriptRoot 'agent.stdout.log') -RedirectStandardError (Join-Path $PSScriptRoot 'agent.stderr.log')
    Start-Sleep -Seconds 2
    if ($agent.HasExited) { throw 'Test agent exited before readiness' }
    $before = & (Join-Path $PSScriptRoot 'rove.exe') --data-dir $DataDir --json status | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'Named pipe status failed' }
    & (Join-Path $PSScriptRoot 'rove.exe') --data-dir $DataDir --json config set --max-active-runs 5 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Named pipe settings update failed' }
    Stop-Process -Id $agent.Id
    $agent.WaitForExit()
    $agent = Start-Process -FilePath (Join-Path $PSScriptRoot 'rove-agent.exe') -ArgumentList $agentArguments -PassThru -WindowStyle Hidden -RedirectStandardOutput (Join-Path $PSScriptRoot 'agent.stdout.log') -RedirectStandardError (Join-Path $PSScriptRoot 'agent.stderr.log')
    Start-Sleep -Seconds 2
    $after = & (Join-Path $PSScriptRoot 'rove.exe') --data-dir $DataDir --json status | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0 -or $before.device_id -ne $after.device_id) { throw 'Device identity changed after restart' }
    $settings = & (Join-Path $PSScriptRoot 'rove.exe') --data-dir $DataDir --json config show | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0 -or $settings.max_active_runs -ne 5) { throw 'Settings were not persisted' }
    @{ result = 'PASS'; device_id = $after.device_id; os = $after.os; checks = @('native named pipe', 'settings write', 'restart identity', 'settings persistence'); overlay_tested = $false } | ConvertTo-Json
} finally {
    if ($null -ne $agent -and -not $agent.HasExited) { Stop-Process -Id $agent.Id; $agent.WaitForExit() }
}
