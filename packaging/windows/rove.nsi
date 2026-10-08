; Rove Windows 安装器（开发测试版）
;
; 每用户安装，不请求管理员权限，不修改 PATH、系统服务、防火墙或网络驱动。
; 用户数据保存在 %LOCALAPPDATA%\rove，卸载默认保留。
;
; 由 scripts/package-windows-setup.py 调用，必需定义：
;   VERSION / SRCDIR / OUTFILE / ICONFILE / DEFAULTDISCOVERYURL

Unicode true
!include "MUI2.nsh"
!include "FileFunc.nsh"

!define APPNAME "Rove"
!define APPNAME_CN "Rove 开发测试版"
!define PUBLISHER "Rove"
!define UNINSTKEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Rove"
!define DATADIR "$LOCALAPPDATA\rove"

Name "${APPNAME_CN}"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\Programs\Rove"
InstallDirRegKey HKCU "${UNINSTKEY}" "InstallLocation"
RequestExecutionLevel user
SetCompressor /SOLID lzma
CRCCheck on
XPStyle on
ShowInstDetails show
ShowUninstDetails show

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${APPNAME}"
VIAddVersionKey "FileDescription" "${APPNAME} 安装程序（开发测试版）"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "CompanyName" "${PUBLISHER}"
VIAddVersionKey "LegalCopyright" "Development build, not a signed release"

!define MUI_ICON "${ICONFILE}"
!define MUI_UNICON "${ICONFILE}"
!define MUI_ABORTWARNING
!define MUI_WELCOMEPAGE_TITLE "${APPNAME_CN} 安装程序"
!define MUI_WELCOMEPAGE_TEXT "本程序把 Rove GUI、CLI、普通用户 agent、EasyTier 网络 helper 和签名 Wintun 组件安装到当前用户目录。$\r$\n$\r$\n安装本身不需要管理员权限。启动网络时由同一 Windows 账号确认系统提权，仅网络 helper 使用权限；不提升业务 agent，不修改现有代理或物理网卡设置。$\r$\n$\r$\n用户数据保存在 %LOCALAPPDATA%\rove，卸载时默认保留。这是未经正式签名的开发测试版。"
!define MUI_FINISHPAGE_TITLE "${APPNAME_CN} 安装完成"
!define MUI_FINISHPAGE_RUN
!define MUI_FINISHPAGE_RUN_TEXT "启动 Rove"
!define MUI_FINISHPAGE_RUN_FUNCTION LaunchGui
!define MUI_FINISHPAGE_TEXT "安装位置：$INSTDIR$\r$\n$\r$\n数据目录（卸载保留）：${DATADIR}$\r$\n$\r$\n打开 Rove 会连接或启动当前用户的 agent；启动网络时按系统提示授权网络 helper。CLI 为 $INSTDIR\rove.exe。"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

!include "LogicLib.nsh"

Var /GLOBAL WebView2Present

Function EnsureRoveStopped
    ; Windows may keep a running executable locked while allowing the remaining
    ; payload to be replaced. Refuse the whole install instead of reporting a
    ; successful but mixed-version upgrade.
    nsExec::ExecToStack 'powershell.exe -NoProfile -NonInteractive -Command "if (Get-Process -Name rove,rove-agent,rove-gui,rove-network-helper -ErrorAction SilentlyContinue) { exit 42 }"'
    Pop $3
    Pop $4
    ${If} $3 == "0"
        Return
    ${ElseIf} $3 == "42"
        ${If} ${Silent}
            SetErrorLevel 32
            Quit
        ${Else}
            MessageBox MB_ICONSTOP|MB_OK "检测到正在运行的 Rove、rove-agent 或 rove-gui。请先关闭这些程序，再重新运行安装器。"
            Abort
        ${EndIf}
    ${Else}
        ${If} ${Silent}
            SetErrorLevel 33
            Quit
        ${Else}
            MessageBox MB_ICONSTOP|MB_OK "无法确认 Rove 进程状态（检查程序返回 $3）。为避免不完整升级，安装已停止。"
            Abort
        ${EndIf}
    ${EndIf}
FunctionEnd

Function .onInit
    SetShellVarContext current
    Call EnsureRoveStopped
    StrCpy $WebView2Present "0"
    SetRegView 32
    ReadRegStr $0 HKLM "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
    SetRegView 64
    ReadRegStr $1 HKLM "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
    ReadRegStr $2 HKCU "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
    ${If} $0 != ""
    ${OrIf} $1 != ""
    ${OrIf} $2 != ""
        StrCpy $WebView2Present "1"
    ${EndIf}
FunctionEnd

Function un.onInit
    SetShellVarContext current
FunctionEnd

Function LaunchGui
    ExecShell "open" "$INSTDIR\rove-gui.exe"
FunctionEnd

LangString DESC_SecMain ${LANG_SIMPCHINESE} "Rove 桌面 GUI、CLI（rove.exe）、设备 agent（rove-agent.exe）与 WebView2 加载器。"
LangString DESC_SecMain ${LANG_ENGLISH} "Rove desktop GUI, CLI (rove.exe), device agent (rove-agent.exe) and WebView2 loader."
LangString DESC_SecDesktopIcon ${LANG_SIMPCHINESE} "在桌面创建 Rove 快捷方式。"
LangString DESC_SecDesktopIcon ${LANG_ENGLISH} "Create a Rove shortcut on the desktop."

Section "Rove（必需）" SecMain
    SectionIn RO
    SetOutPath "$INSTDIR"
    SetOverwrite on

    File "${SRCDIR}/rove.exe"
    File "${SRCDIR}/rove-agent.exe"
    File "${SRCDIR}/rove-network-helper.exe"
    File "${SRCDIR}/wintun.dll"
    File "${SRCDIR}/WINTUN-LICENSE.txt"
    File "${SRCDIR}/WinDivert64.sys"
    File "${SRCDIR}/WINDIVERT-LICENSE.txt"
    File "${SRCDIR}/WinDivert-2.2.2-Source.zip"
    File "${SRCDIR}/rove-gui.exe"
    File "${SRCDIR}/WebView2Loader.dll"
    File "${SRCDIR}/rove-defaults.json"

    File "${SRCDIR}/Test-Rove.ps1"
    File "${SRCDIR}/Test-RoveGui.ps1"
    ; 回归脚本与二进制放在同一目录：脚本内部用 $PSScriptRoot 拼接 rove.exe / rove-agent.exe。

    FileOpen $3 "$INSTDIR\README.txt" w
    FileWrite $3 "Rove Windows x86_64 开发测试安装包$\r$\n"
    FileWrite $3 "版本 ${VERSION}$\r$\n$\r$\n"
    FileWrite $3 "安装内容：$\r$\n"
    FileWrite $3 "  rove.exe            CLI 客户端$\r$\n"
    FileWrite $3 "  rove-agent.exe      每设备 agent（本机 named pipe 服务）$\r$\n"
    FileWrite $3 "  rove-network-helper.exe  受限 EasyTier 网络运行时$\r$\n"
    FileWrite $3 "  wintun.dll         官方签名虚拟网卡组件（许可见 WINTUN-LICENSE.txt）$\r$\n"
    FileWrite $3 "  rove-gui.exe        桌面 GUI（需要 WebView2 Runtime）$\r$\n"
    FileWrite $3 "  WebView2Loader.dll  GUI 加载器$\r$\n"
    FileWrite $3 "  rove-defaults.json 打包时设置的默认发现服务（用户设置可覆盖）$\r$\n"
    FileWrite $3 "  Test-Rove.ps1       原生回归（named pipe / 身份 / 设置持久化）$\r$\n"
    FileWrite $3 "  Test-RoveGui.ps1    界面回归（需要交互式桌面）$\r$\n$\r$\n"
    FileWrite $3 "使用：打开 Rove GUI 自动连接或启动普通用户 agent；也可单独运行 agent，再使用 CLI。$\r$\n"
    FileWrite $3 "数据目录：%LOCALAPPDATA%\rove（卸载保留）。$\r$\n"
    FileWrite $3 "启动网络会请求系统提权，仅网络 helper 获得权限，业务 agent 保持普通用户权限。$\r$\n"
    FileWrite $3 "仅为 Rove overlay 添加临时隔离规则，不修改现有代理、全局转发或物理网卡；无须单独安装 EasyTier 或 Mihomo。$\r$\n"
    FileWrite $3 "原生回归：powershell -NoProfile -File .\Test-Rove.ps1$\r$\n"
    FileWrite $3 "界面回归：在已登录交互式桌面运行 .\Test-RoveGui.ps1$\r$\n"
    FileClose $3

    WriteINIStr "$INSTDIR\install-info.ini" "install" "version" "${VERSION}"
    WriteINIStr "$INSTDIR\install-info.ini" "install" "layout" "per-user"
    WriteINIStr "$INSTDIR\install-info.ini" "install" "webview2_runtime" "$WebView2Present"
    WriteINIStr "$INSTDIR\install-info.ini" "install" "overlay_runtime" "easytier-native-helper"
    WriteINIStr "$INSTDIR\install-info.ini" "install" "data_dir" "${DATADIR}"
    WriteINIStr "$INSTDIR\install-info.ini" "install" "discovery_service_url" "${DEFAULTDISCOVERYURL}"

    CreateDirectory "$SMPROGRAMS\Rove"
    CreateShortCut "$SMPROGRAMS\Rove\Rove.lnk" "$INSTDIR\rove-gui.exe" "" "$INSTDIR\rove-gui.exe" 0
    CreateShortCut "$SMPROGRAMS\Rove\卸载 Rove.lnk" "$INSTDIR\uninstall.exe"

    WriteRegStr HKCU "${UNINSTKEY}" "DisplayName" "${APPNAME_CN}"
    WriteRegStr HKCU "${UNINSTKEY}" "DisplayVersion" "${VERSION}"
    WriteRegStr HKCU "${UNINSTKEY}" "Publisher" "${PUBLISHER}"
    WriteRegStr HKCU "${UNINSTKEY}" "DisplayIcon" "$INSTDIR\rove-gui.exe"
    WriteRegStr HKCU "${UNINSTKEY}" "InstallLocation" "$INSTDIR"
    WriteRegStr HKCU "${UNINSTKEY}" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
    WriteRegStr HKCU "${UNINSTKEY}" "QuietUninstallString" "$\"$INSTDIR\uninstall.exe$\" /S"
    WriteRegStr HKCU "${UNINSTKEY}" "NoModify" "1"
    WriteRegStr HKCU "${UNINSTKEY}" "NoRepair" "1"
    WriteRegDWORD HKCU "${UNINSTKEY}" "VersionMajor" 0
    WriteRegDWORD HKCU "${UNINSTKEY}" "VersionMinor" 1

    WriteUninstaller "$INSTDIR\uninstall.exe"

    ${GetSize} "$INSTDIR" "/S=0K" $4 $5 $6
    IntFmt $4 "0x%08X" $4
    WriteRegDWORD HKCU "${UNINSTKEY}" "EstimatedSize" "$4"

    ${IfNot} ${Silent}
    ${AndIf} $WebView2Present != "1"
        MessageBox MB_ICONEXCLAMATION|MB_OK "未检测到 WebView2 Runtime。rove-gui.exe 需要它才能显示窗口；CLI 与 agent 不受影响。请安装 Microsoft Edge WebView2 Runtime 后重新启动 Rove。"
    ${EndIf}
SectionEnd

Section /o "桌面快捷方式" SecDesktopIcon
    CreateShortCut "$DESKTOP\Rove.lnk" "$INSTDIR\rove-gui.exe" "" "$INSTDIR\rove-gui.exe" 0
SectionEnd

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
!insertmacro MUI_DESCRIPTION_TEXT ${SecMain} $(DESC_SecMain)
!insertmacro MUI_DESCRIPTION_TEXT ${SecDesktopIcon} $(DESC_SecDesktopIcon)
!insertmacro MUI_FUNCTION_DESCRIPTION_END

Function .onInstSuccess
    DetailPrint "已安装到 $INSTDIR"
    DetailPrint "数据目录保留：${DATADIR}"
FunctionEnd

Section "Uninstall"
    Call un.EnsureNetworkHelperStopped
    Delete "$INSTDIR\rove.exe"
    Delete "$INSTDIR\rove-agent.exe"
    Delete "$INSTDIR\rove-network-helper.exe"
    Delete "$INSTDIR\wintun.dll"
    Delete "$INSTDIR\WINTUN-LICENSE.txt"
    Delete "$INSTDIR\WinDivert64.sys"
    Delete "$INSTDIR\WINDIVERT-LICENSE.txt"
    Delete "$INSTDIR\WinDivert-2.2.2-Source.zip"
    Delete "$INSTDIR\rove-gui.exe"
    Delete "$INSTDIR\WebView2Loader.dll"
    Delete "$INSTDIR\rove-defaults.json"
    Delete "$INSTDIR\README.txt"
    Delete "$INSTDIR\install-info.ini"
    Delete "$INSTDIR\Test-Rove.ps1"
    Delete "$INSTDIR\Test-RoveGui.ps1"
    Delete "$INSTDIR\uninstall.exe"
    RMDir "$INSTDIR"

    Delete "$SMPROGRAMS\Rove\Rove.lnk"
    Delete "$SMPROGRAMS\Rove\卸载 Rove.lnk"
    RMDir "$SMPROGRAMS\Rove"
    Delete "$DESKTOP\Rove.lnk"

    DeleteRegKey HKCU "${UNINSTKEY}"

    ${IfNot} ${Silent}
        MessageBox MB_ICONINFORMATION|MB_OK "Rove 已卸载。$\r$\n$\r$\n用户数据仍保留在 ${DATADIR}（设备身份、会话与设置）。如需彻底清除，请手动删除该目录。"
    ${EndIf}
SectionEnd

Function un.EnsureNetworkHelperStopped
    nsExec::ExecToStack 'powershell.exe -NoProfile -NonInteractive -Command "if (Get-Process -Name rove,rove-agent,rove-gui,rove-network-helper -ErrorAction SilentlyContinue) { exit 42 }"'
    Pop $3
    Pop $4
    ${If} $3 != "0"
        SetErrorLevel 32
        ${IfNot} ${Silent}
            MessageBox MB_ICONSTOP|MB_OK "请先关闭 Rove 与网络 helper，再卸载。"
        ${EndIf}
        Quit
    ${EndIf}
FunctionEnd
