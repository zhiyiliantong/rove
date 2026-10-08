# Network behaviour of an installed Windows Rove build on a real host.
# Records TCP listeners before/after starting the shipped agent, runs the product
# reachability probe (rove network test-peer) for each endpoint, and optionally
# holds one inbound listener open so another platform can probe it.
param(
  [string[]]$Endpoints = @('tcp://10.1.2.242:22', 'tcp://10.1.2.242:59999'),
  [int]$InboundProbeSeconds = 0,
  [int]$InboundProbePort = 15888,
  [string]$InstallDir = "$env:LOCALAPPDATA\Programs\Rove"
)
$ErrorActionPreference = 'Continue'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$agent = Join-Path $InstallDir 'rove-agent.exe'
$cli = Join-Path $InstallDir 'rove.exe'
$out = [ordered]@{ host = $env:COMPUTERNAME; time = (Get-Date).ToString('o') }

function Get-ListenerLines {
  Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue |
    ForEach-Object {
      $name = (Get-Process -Id $_.OwningProcess -ErrorAction SilentlyContinue).ProcessName
      '{0}:{1}/pid={2}/{3}' -f $_.LocalAddress, $_.LocalPort, $_.OwningProcess, $name
    } | Sort-Object -Unique
}

function Invoke-RoveCli([string[]]$CliArgs) {
  $o = Join-Path $env:TEMP ('rove-cli-' + [guid]::NewGuid().ToString('n') + '.out')
  $e = Join-Path $env:TEMP ('rove-cli-' + [guid]::NewGuid().ToString('n') + '.err')
  $proc = Start-Process -FilePath $cli -ArgumentList $CliArgs -NoNewWindow -PassThru -RedirectStandardOutput $o -RedirectStandardError $e
  $null = $proc.WaitForExit(30000)
  if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force }
  # Read as plain text: Get-Content would wrap each line in a FileSystem provider object.
  $stdout = @(if (Test-Path $o) { [System.IO.File]::ReadAllText($o) -split "`r?`n" | Where-Object { $_ -ne '' } })
  $stderr = @(if (Test-Path $e) { [System.IO.File]::ReadAllText($e) -split "`r?`n" | Where-Object { $_ -ne '' } })
  Remove-Item $o, $e -ErrorAction SilentlyContinue
  [ordered]@{ args = $CliArgs; exited = $proc.HasExited; exit_code = $(if ($proc.HasExited) { $proc.ExitCode } else { 'timeout' }); stdout = $stdout; stderr = $stderr }
}

$out.local_ipv4 = @(Get-NetIPAddress -AddressFamily IPv4 -ErrorAction SilentlyContinue | ForEach-Object { '{0}/{1}' -f $_.IPAddress, $_.PrefixLength })
$out.firewall_profiles = @(Get-NetFirewallProfile -ErrorAction SilentlyContinue | ForEach-Object { '{0}:{1}' -f $_.Name, $_.Enabled })
$out.rove_firewall_rules = @(Get-NetFirewallRule -ErrorAction SilentlyContinue | Where-Object DisplayName -Like '*Rove*' | ForEach-Object { $_.DisplayName })

$before = @(Get-ListenerLines)
$out.baseline_listener_count = $before.Count
$out.rove_processes_before = @(Get-Process -Name 'rove*' -ErrorAction SilentlyContinue | ForEach-Object { '{0}/{1}' -f $_.ProcessName, $_.Id })

$p = Start-Process -FilePath $agent -WorkingDirectory $InstallDir -PassThru -WindowStyle Hidden -RedirectStandardOutput (Join-Path $env:TEMP 'rove-net-agent.out.log') -RedirectStandardError (Join-Path $env:TEMP 'rove-net-agent.err.log')
Start-Sleep -Seconds 7
$out.agent_pid = $p.Id
$out.agent_exited = $p.HasExited
if ($p.HasExited) { $out.agent_exit_code = $p.ExitCode }
else {
  $rovePids = @(Get-Process -Name 'rove*' -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id)
  $after = @(Get-ListenerLines)
  $out.rove_processes_after = @(Get-Process -Name 'rove*' -ErrorAction SilentlyContinue | ForEach-Object { '{0}/{1}' -f $_.ProcessName, $_.Id })
  $out.listeners_added_by_rove = @(Compare-Object $before $after | Where-Object SideIndicator -eq '=>' | ForEach-Object InputObject)
  $out.rove_owned_listeners = @($after | Where-Object { $line = $_; @($rovePids | Where-Object { $line -match ('/pid=' + $_ + '/') }).Count -gt 0 })
  $out.cli_status = Invoke-RoveCli @('--json', 'status')
  $out.cli_hello = Invoke-RoveCli @('--json', 'call', 'get_hello')
  $out.cli_network_list = Invoke-RoveCli @('--json', 'network', 'list')
  $out.cli_test_peer = @()
  foreach ($endpoint in $Endpoints) {
    $out.cli_test_peer += [ordered]@{ endpoint = $endpoint; result = (Invoke-RoveCli @('--json', 'network', 'test-peer', $endpoint)) }
  }
  if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force }
}
Start-Sleep -Seconds 1
$out.agent_stopped = -not [bool](Get-Process -Id $p.Id -ErrorAction SilentlyContinue)

if ($InboundProbeSeconds -gt 0) {
  $listener = New-Object System.Net.Sockets.TcpListener([System.Net.IPAddress]::Any, $InboundProbePort)
  $listener.Start()
  $out.inbound_probe_port = $InboundProbePort
  $out.inbound_listener_open = $true
  Write-Output ($out | ConvertTo-Json -Depth 6)
  Write-Output 'LISTENER-OPEN'
  Start-Sleep -Seconds $InboundProbeSeconds
  $listener.Stop()
  $out.inbound_listener_open = $false
}
$out | ConvertTo-Json -Depth 6
