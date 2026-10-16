# Verify the per-user Windows installer lifecycle on a real host.
# Actions: uninstall, install (fresh or same-version upgrade), native, status.
# Prints one JSON object on stdout; never writes to the machine PATH or firewall.
param(
  [ValidateSet('install', 'status', 'native', 'uninstall')][string]$Action = 'install',
  [string]$Root = "$env:USERPROFILE\rove-setup",
  [string]$SetupName = 'rove-windows-x86_64-setup.exe',
  [int]$TimeoutSeconds = 180
)
if ($Action -eq 'install' -or $Action -eq 'native') {
  $ErrorActionPreference = 'Stop'
} else {
  $ErrorActionPreference = 'Continue'
}
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$install = Join-Path $env:LOCALAPPDATA 'Programs\Rove'
$data = Join-Path $env:LOCALAPPDATA 'rove'
$uninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Rove'
$startMenu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Rove'
$result = [ordered]@{ action = $Action; host = $env:COMPUTERNAME; user = "$env:USERDOMAIN\$env:USERNAME" }

if ($Action -eq 'install') {
  $setup = Join-Path $Root $SetupName
  $result.setup_sha256 = (Get-FileHash $setup -Algorithm SHA256).Hash.ToLower()
  $result.setup_bytes = (Get-Item $setup).Length
  $result.installed_before = Test-Path $install
  $sw = [System.Diagnostics.Stopwatch]::StartNew()
  $proc = Start-Process -FilePath $setup -ArgumentList '/S' -PassThru
  $exited = $proc.WaitForExit($TimeoutSeconds * 1000)
  $sw.Stop()
  $result.setup_exited_within_timeout = $exited
  $result.setup_seconds = [math]::Round($sw.Elapsed.TotalSeconds, 1)
  if (-not $exited) { $result.setup_exit_code = 'timed out'; Stop-Process -Id $proc.Id -Force }
  else { $result.setup_exit_code = $proc.ExitCode }
  $result.install_exists = Test-Path $install
  $files = @()
  if ($result.install_exists) {
    foreach ($f in Get-ChildItem -Path $install -Recurse -File | Sort-Object FullName) {
      $files += [ordered]@{ path = $f.FullName.Substring($install.Length + 1); bytes = $f.Length; sha256 = (Get-FileHash $f.FullName -Algorithm SHA256).Hash.ToLower() }
    }
  }
  $result.files = $files
  if (Test-Path (Join-Path $install 'install-info.ini')) {
    $result.install_info = [System.IO.File]::ReadAllText((Join-Path $install 'install-info.ini'))
  }
  if (Test-Path $uninstallKey) {
    $props = Get-ItemProperty $uninstallKey
    $result.uninstall_registry = [ordered]@{ DisplayName = $props.DisplayName; DisplayVersion = $props.DisplayVersion; InstallLocation = $props.InstallLocation; UninstallString = $props.UninstallString; QuietUninstallString = $props.QuietUninstallString }
  }
  $result.start_menu = @()
  if (Test-Path $startMenu) { $result.start_menu = @(Get-ChildItem $startMenu | ForEach-Object { $_.Name }) }
  $result.data_dir_exists = Test-Path $data
  $result.user_path_unchanged = ([Environment]::GetEnvironmentVariable('Path', 'User') -notlike '*Rove*')
  $result.machine_path_unchanged = ([Environment]::GetEnvironmentVariable('Path', 'Machine') -notlike '*Rove*')
}

if ($Action -eq 'native') {
  $result.data_dir_before = @(Get-ChildItem $data -Force | ForEach-Object { $_.Name })
  $json = & (Join-Path $install 'Test-Rove.ps1') -DataDir $data
  $parsed = $json | ConvertFrom-Json
  $result.test_result = $parsed.result
  $result.device_id = $parsed.device_id
  $result.os = $parsed.os
  $result.checks = $parsed.checks
  $result.overlay_tested = $parsed.overlay_tested
  $result.data_dir_after = @(Get-ChildItem $data -Force | ForEach-Object { $_.Name })
}

if ($Action -eq 'uninstall') {
  if (-not (Test-Path $data)) { New-Item -ItemType Directory -Path $data | Out-Null }
  Set-Content -Path (Join-Path $data 'uninstall-marker.txt') -Value 'rove data survives uninstall'
  $result.data_marker_before = Test-Path (Join-Path $data 'uninstall-marker.txt')
  $uninst = Join-Path $install 'uninstall.exe'
  $result.uninstaller_exists = Test-Path $uninst
  if ($result.uninstaller_exists) {
    Start-Process -FilePath $uninst -ArgumentList '/S' | Out-Null
    Start-Sleep -Seconds 25
  }
  $leftovers = Get-Process | Where-Object { $_.ProcessName -like '*nsu*' -or $_.ProcessName -like 'Au_*' -or $_.ProcessName -like '*uninstall*' }
  $result.leftover_processes = @($leftovers | ForEach-Object { $_.ProcessName })
  foreach ($p in $leftovers) { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue }
  Start-Sleep -Seconds 2
  $result.install_dir_removed = -not (Test-Path $install)
  if (-not $result.install_dir_removed) { $result.install_dir_remaining = @(Get-ChildItem $install -Recurse -File | ForEach-Object { $_.FullName }) }
  $result.uninstall_registry_removed = -not (Test-Path $uninstallKey)
  $result.start_menu_removed = -not (Test-Path $startMenu)
  $result.data_marker_after = Test-Path (Join-Path $data 'uninstall-marker.txt')
  $result.data_dir_after = Test-Path $data
}

if ($Action -eq 'status') {
  $result.install_dir_removed = -not (Test-Path $install)
  $result.uninstall_registry_removed = -not (Test-Path $uninstallKey)
  $result.start_menu_removed = -not (Test-Path $startMenu)
  $result.data_dir_exists = Test-Path $data
  $result.data_marker_exists = Test-Path (Join-Path $data 'uninstall-marker.txt')
  $result.leftover_install_files = @(if (Test-Path $install) { Get-ChildItem $install -Recurse -File | ForEach-Object { $_.Name } })
  $result.any_rove_process = @(Get-Process | Where-Object { $_.ProcessName -like '*rove*' -or $_.ProcessName -like '*nsu*' } | ForEach-Object { $_.ProcessName })
}

$result | ConvertTo-Json -Depth 6
