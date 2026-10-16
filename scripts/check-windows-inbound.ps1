# Hold one TCP port open on a Windows host so a second platform can probe it.
# The host self-connects first, which proves the port really is bound and
# accepting: any external failure is therefore path/firewall, not a missing
# service. Prints one JSON object while the port is open and one when it closes.
param(
  [int]$Port = 15888,
  [int]$HoldSeconds = 30,
  [int]$SelfProbes = 3
)
$ErrorActionPreference = 'Continue'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$out = [ordered]@{ host = $env:COMPUTERNAME; port = $Port; hold_seconds = $HoldSeconds }
$listener = New-Object System.Net.Sockets.TcpListener([System.Net.IPAddress]::Any, $Port)
try {
  $listener.Start()
  $out.bound = $true
} catch {
  $out.bound = $false
  $out.bind_error = $_.Exception.Message
}

if ($out.bound) {
  $self = @()
  for ($i = 0; $i -lt $SelfProbes; $i++) {
    try {
      $c = New-Object System.Net.Sockets.TcpClient
      $c.Connect('127.0.0.1', $Port)
      $self += 'reachable'
      $c.Close()
    } catch { $self += 'failed' }
  }
  $out.local_self_connect = $self
  $out.listener_address = $listener.LocalEndpoint.ToString()
  Write-Output ($out | ConvertTo-Json -Depth 4)
  Write-Output 'LISTENER-OPEN'
  Start-Sleep -Seconds $HoldSeconds
  $listener.Stop()
}
$out.listener_open = $false
Write-Output ($out | ConvertTo-Json -Depth 4)
