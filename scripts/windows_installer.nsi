; NSIS Installer Script
; Modern UI with license, installation scope, and shortcut options

!include "MUI2.nsh"
!include "LogicLib.nsh"

!define APP_NAME "vkCommander"
!define APP_DISPLAY_NAME "vkCommander"
!define APP_EXECUTABLE "vk_commander.exe"
!define APP_PUBLISHER "Benjamin Oswald"
!define APP_VERSION "0.0.0"

Name "${APP_DISPLAY_NAME}"
OutFile "../releases/windows/${APP_NAME}Installer.exe"
InstallDir "$PROGRAMFILES64\${APP_DISPLAY_NAME}"
InstallDirRegKey HKLM "Software\${APP_NAME}" "InstallPath"
RequestExecutionLevel highest
ShowInstDetails nevershow
ShowUninstDetails nevershow
SetCompressor /SOLID lzma

Var StartMenuFolder
Var InstallForAllUsers
Var CreateDesktopShortcut
Var CreateStartMenuShortcut
Var RadioAllUsers
Var RadioCurrentUser
Var CheckboxDesktop
Var CheckboxStartMenu
Var HasAdminRights

!define MUI_ABORTWARNING
!define MUI_ICON "build_resources/app_icon/vk_commander.ico"
!define MUI_UNICON "build_resources/app_icon/vk_commander.ico"

!define MUI_STARTMENUPAGE_REGISTRY_ROOT "HKLM"
!define MUI_STARTMENUPAGE_REGISTRY_KEY "Software\${APP_NAME}"
!define MUI_STARTMENUPAGE_REGISTRY_VALUENAME "Start Menu Folder"

; Pages
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "../../license.txt"
Page custom InstallScopePage InstallScopePageLeave
Page custom ShortcutsPage ShortcutsPageLeave
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

; Uninstaller pages
!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

; Languages
!insertmacro MUI_LANGUAGE "English"

Function un.onInit
    ; Determine installation scope from registry
    ReadRegStr $0 HKLM "Software\${APP_NAME}" "InstallScope"
    ${If} $0 == "AllUsers"
        StrCpy $InstallForAllUsers "1"
        SetShellVarContext all
    ${Else}
        ; Check if it was a current user installation
        ReadRegStr $0 HKCU "Software\${APP_NAME}" "InstallScope"
        ${If} $0 == "CurrentUser"
            StrCpy $InstallForAllUsers "0"
            SetShellVarContext current
        ${Else}
            ; Fallback: try to determine from uninstall registry location
            ReadRegStr $0 HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "DisplayName"
            ${If} $0 != ""
                StrCpy $InstallForAllUsers "1"
                SetShellVarContext all
            ${Else}
                StrCpy $InstallForAllUsers "0"
                SetShellVarContext current
            ${EndIf}
        ${EndIf}
    ${EndIf}

    MessageBox MB_YESNO "Do you want to remove vkCommander from your computer?" IDYES NoAbort
      Abort
    NoAbort:
FunctionEnd

; Custom page for installation scope
Function InstallScopePage
    !insertmacro MUI_HEADER_TEXT "Installation Scope" "Choose installation scope for the application."

    nsDialogs::Create 1018
    Pop $0

    ${NSD_CreateLabel} 0 0 100% 20u "Choose whether to install for all users or just for you:"
    Pop $0

    ${NSD_CreateRadioButton} 10 30u 280u 12u "Install for all users (recommended)"
    Pop $RadioAllUsers

    ${NSD_CreateRadioButton} 10 50u 280u 12u "Install for current user only"
    Pop $RadioCurrentUser

    ; Check if user has admin rights
    UserInfo::GetAccountType
    Pop $0
    ${If} $0 == "admin"
        StrCpy $HasAdminRights "1"
        ${NSD_SetState} $RadioAllUsers ${BST_CHECKED}
        ${NSD_CreateLabel} 10 80u 280u 40u "Note: Installing for all users requires administrator privileges."
        Pop $0
    ${Else}
        StrCpy $HasAdminRights "0"
        ; Disable the "all users" option and select "current user"
        EnableWindow $RadioAllUsers 0
        ${NSD_SetState} $RadioCurrentUser ${BST_CHECKED}
        ${NSD_CreateLabel} 10 80u 280u 40u "Administrator privileges are required to install for all users. Option disabled."
        Pop $0
    ${EndIf}

    nsDialogs::Show
FunctionEnd

Function InstallScopePageLeave
    ${If} $HasAdminRights == "1"
        ; Check which option is selected (only if admin rights available)
        ${NSD_GetState} $RadioAllUsers $0
        ${If} $0 == ${BST_CHECKED}
            StrCpy $InstallForAllUsers "1"
            ; Set context for all users installation
            SetShellVarContext all
            ; Change install directory to Program Files
            StrCpy $INSTDIR "$PROGRAMFILES64\${APP_DISPLAY_NAME}"
        ${Else}
            StrCpy $InstallForAllUsers "0"
            SetShellVarContext current
            ; Change install directory to user's local app data
            StrCpy $INSTDIR "$LOCALAPPDATA\${APP_DISPLAY_NAME}"
        ${EndIf}
    ${Else}
        ; No admin rights - force current user installation
        StrCpy $InstallForAllUsers "0"
        SetShellVarContext current
        ; Change install directory to user's local app data
        StrCpy $INSTDIR "$LOCALAPPDATA\${APP_DISPLAY_NAME}"
    ${EndIf}
FunctionEnd

; Custom page for shortcuts
Function ShortcutsPage
    !insertmacro MUI_HEADER_TEXT "Shortcuts" "Choose which shortcuts to create."

    nsDialogs::Create 1018
    Pop $0

    ${NSD_CreateLabel} 0 0 100% 20u "Select the shortcuts you want to create:"
    Pop $0

    ${NSD_CreateCheckbox} 10 30u 280u 12u "Create desktop shortcut"
    Pop $CheckboxDesktop
    ${NSD_SetState} $CheckboxDesktop ${BST_CHECKED}

    ${NSD_CreateCheckbox} 10 50u 280u 12u "Create Start Menu shortcut"
    Pop $CheckboxStartMenu
    ${NSD_SetState} $CheckboxStartMenu ${BST_CHECKED}

    nsDialogs::Show
FunctionEnd

Function ShortcutsPageLeave
    ${NSD_GetState} $CheckboxDesktop $0
    ${If} $0 == ${BST_CHECKED}
        StrCpy $CreateDesktopShortcut "1"
    ${Else}
        StrCpy $CreateDesktopShortcut "0"
    ${EndIf}

    ${NSD_GetState} $CheckboxStartMenu $0
    ${If} $0 == ${BST_CHECKED}
        StrCpy $CreateStartMenuShortcut "1"
    ${Else}
        StrCpy $CreateStartMenuShortcut "0"
    ${EndIf}
FunctionEnd

; Installer section
Section "Main Application" SecMain
    SectionIn RO
    SetOutPath $INSTDIR

    ; App files
    File "../target/x86_64-pc-windows-gnu/release/${APP_EXECUTABLE}"

    ; Registration
    ${If} $InstallForAllUsers == "1"
        WriteRegStr HKLM "Software\${APP_NAME}" "InstallPath" $INSTDIR
        WriteRegStr HKLM "Software\${APP_NAME}" "InstallScope" "AllUsers"
        WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "DisplayName" "${APP_DISPLAY_NAME}"
        WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "UninstallString" "$INSTDIR\Uninstall.exe"
        WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "DisplayIcon" "$INSTDIR\${APP_EXECUTABLE}"
        WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "Publisher" "${APP_PUBLISHER}"
        WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "DisplayVersion" "${APP_VERSION}"
        WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "NoModify" 1
        WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "NoRepair" 1
    ${Else}
        WriteRegStr HKCU "Software\${APP_NAME}" "InstallPath" $INSTDIR
        WriteRegStr HKCU "Software\${APP_NAME}" "InstallScope" "CurrentUser"
        WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "DisplayName" "${APP_DISPLAY_NAME}"
        WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "UninstallString" "$INSTDIR\Uninstall.exe"
        WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "DisplayIcon" "$INSTDIR\${APP_EXECUTABLE}"
        WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "Publisher" "${APP_PUBLISHER}"
        WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "DisplayVersion" "${APP_VERSION}"
        WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "NoModify" 1
        WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}" "NoRepair" 1
    ${EndIf}

    ; Create uninstaller
    WriteUninstaller "$INSTDIR\Uninstall.exe"
SectionEnd

; Shortcuts section
Section "Shortcuts" SecShortcuts
    ; Create Start Menu shortcut if requested
    ${If} $CreateStartMenuShortcut == "1"
        CreateShortCut "$SMPROGRAMS\${APP_DISPLAY_NAME}.lnk" "$INSTDIR\${APP_EXECUTABLE}"
    ${EndIf}

    ; Create Desktop shortcut if requested
    ${If} $CreateDesktopShortcut == "1"
        CreateShortCut "$DESKTOP\${APP_DISPLAY_NAME}.lnk" "$INSTDIR\${APP_EXECUTABLE}"
    ${EndIf}
SectionEnd

; Initialize default values
Function .onInit
    StrCpy $StartMenuFolder "${APP_DISPLAY_NAME}"
    StrCpy $InstallForAllUsers "1"
    StrCpy $CreateDesktopShortcut "1"
    StrCpy $CreateStartMenuShortcut "1"
    StrCpy $HasAdminRights "0"
FunctionEnd

; Uninstaller section
Section "Uninstall"
    ; Remove registry keys based on installation scope
    ${If} $InstallForAllUsers == "1"
        DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"
        DeleteRegKey HKLM "Software\${APP_NAME}"
    ${Else}
        DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"
        DeleteRegKey HKCU "Software\${APP_NAME}"
    ${EndIf}

    ; Remove files and uninstaller
    Delete "$INSTDIR\${APP_EXECUTABLE}"
    Delete "$INSTDIR\Uninstall.exe"

    ; Remove directories
    RMDir "$INSTDIR"

    ; Remove shortcuts (context is already set in un.onInit)
    Delete "$DESKTOP\${APP_DISPLAY_NAME}.lnk"
    Delete "$SMPROGRAMS\${APP_DISPLAY_NAME}.lnk"
SectionEnd
