param([string]$DataDir = (Join-Path $PSScriptRoot 'gui-test-data'))
$ErrorActionPreference = 'Stop'
$agent = $null
$gui = $null
try {
    $env:ROVE_DATA_DIR = $DataDir
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--force-renderer-accessibility'
    $agent = Start-Process (Join-Path $PSScriptRoot 'rove-agent.exe') -PassThru
    Start-Sleep -Seconds 2
    $status = & (Join-Path $PSScriptRoot 'rove.exe') --json status | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'Agent SDK status failed' }
    $gui = Start-Process (Join-Path $PSScriptRoot 'rove-gui.exe') -PassThru -RedirectStandardError (Join-Path $PSScriptRoot 'gui.stderr.log')
    $deadline = (Get-Date).AddSeconds(30)
    do {
        Start-Sleep -Seconds 1
        $gui.Refresh()
        if ($gui.HasExited) { throw "GUI exited before window creation: $($gui.ExitCode)" }
    } while ($gui.MainWindowHandle -eq 0 -and (Get-Date) -lt $deadline)
    if ($gui.MainWindowHandle -eq 0) { throw 'GUI window was not created' }
    Add-Type -AssemblyName UIAutomationClient
    Add-Type -AssemblyName UIAutomationTypes
    $window = [System.Windows.Automation.AutomationElement]::FromHandle($gui.MainWindowHandle)
    $deadline = (Get-Date).AddSeconds(30)
    $matched = $false
    do {
        Start-Sleep -Seconds 1
        $elements = $window.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
        foreach ($element in $elements) {
            if ($element.Current.Name.Contains([string]$status.device_id)) { $matched = $true; break }
        }
    } while (-not $matched -and (Get-Date) -lt $deadline)
    if (-not $matched) { throw 'GUI did not expose the agent device identity in its accessibility tree' }
    @{ result = 'PASS'; scope = 'native GUI startup and rendered device identity matches CLI'; title = $gui.MainWindowTitle; device_id = $status.device_id; gui_ipc_verified = $true; overlay_tested = $false } | ConvertTo-Json | Set-Content (Join-Path $PSScriptRoot 'gui-test-result.json')
} catch {
    @{ result = 'FAIL'; error = $_.Exception.Message } | ConvertTo-Json | Set-Content (Join-Path $PSScriptRoot 'gui-test-result.json')
    throw
} finally {
    foreach ($owned in @($gui, $agent)) {
        if ($null -ne $owned -and -not $owned.HasExited) { Stop-Process -Id $owned.Id; $owned.WaitForExit() }
    }
}
