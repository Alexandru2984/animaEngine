; animaEngine's Windows installer, built by scripts/build-windows.sh:
;   makensis -DVERSION=1.5.0-rc1 -DVERSION_NUM=1.5.0 -DSTAGE=<dir> \
;            -DICON=<ico> -DOUTFILE=<exe> installer.nsi
; VERSION_NUM is VERSION without a pre-release part: Windows' version
; resource takes numbers only.
;
; Per user: into %LOCALAPPDATA%\Programs\animaEngine, with no
; administrator rights asked, a Start-menu entry, and an uninstaller that
; Settings -> Apps lists. Uninstalling keeps the settings and characters
; (%APPDATA%\animaEngine) and takes the app's "Start at login" value with
; it. In the language Windows is set to, of the app's ten.

Unicode true
!include "MUI2.nsh"
!include "LogicLib.nsh"

!define APP "animaEngine"
!define UNINST "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP}"
!define RUN "Software\Microsoft\Windows\CurrentVersion\Run"

Name "${APP}"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\Programs\${APP}"
InstallDirRegKey HKCU "Software\${APP}" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
ShowInstDetails nevershow
ShowUninstDetails nevershow

VIProductVersion "${VERSION_NUM}.0"
VIAddVersionKey /LANG=1033 "ProductName" "${APP}"
VIAddVersionKey /LANG=1033 "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=1033 "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=1033 "FileDescription" "${APP} ${VERSION} setup"
VIAddVersionKey /LANG=1033 "LegalCopyright" "Copyright (c) animaEngine contributors. MIT licence."

!define MUI_ICON "${ICON}"
!define MUI_UNICON "${ICON}"
!define MUI_FINISHPAGE_RUN "$INSTDIR\anima-engine.exe"
!insertmacro MUI_PAGE_LICENSE "${STAGE}\LICENSE.txt"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

; The first is the fallback for a Windows in any other language.
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Romanian"
!insertmacro MUI_LANGUAGE "German"
!insertmacro MUI_LANGUAGE "French"
!insertmacro MUI_LANGUAGE "Spanish"
!insertmacro MUI_LANGUAGE "Italian"
!insertmacro MUI_LANGUAGE "PortugueseBR"
!insertmacro MUI_LANGUAGE "Polish"
!insertmacro MUI_LANGUAGE "Dutch"
!insertmacro MUI_LANGUAGE "Japanese"

; A running copy holds its .exe open. Asked to close — not forced — it
; saves as it does when its window is closed; waited for, up to ten
; seconds, before its files are written over.
!macro CloseRunning
  nsExec::Exec 'taskkill /im anima-engine.exe'
  Pop $0
  StrCpy $1 0
  ${Do}
    nsExec::Exec 'cmd /c tasklist /fi "imagename eq anima-engine.exe" | find /i "anima-engine.exe"'
    Pop $0
    ${If} $0 != 0
      ${Break}
    ${EndIf}
    Sleep 500
    IntOp $1 $1 + 1
  ${LoopUntil} $1 >= 20
!macroend

Section "${APP}"
  !insertmacro CloseRunning
  SetOutPath "$INSTDIR"
  File "${STAGE}\anima-engine.exe"
  File "${STAGE}\LICENSE.txt"
  File "${STAGE}\THIRD-PARTY-LICENSES.txt"
  File "${STAGE}\README.txt"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateShortCut "$SMPROGRAMS\${APP}.lnk" "$INSTDIR\anima-engine.exe"
  WriteRegStr HKCU "Software\${APP}" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "${UNINST}" "DisplayName" "${APP}"
  WriteRegStr HKCU "${UNINST}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINST}" "Publisher" "${APP}"
  WriteRegStr HKCU "${UNINST}" "DisplayIcon" "$INSTDIR\anima-engine.exe"
  WriteRegStr HKCU "${UNINST}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINST}" "URLInfoAbout" "https://animaengine.eu"
  WriteRegStr HKCU "${UNINST}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "${UNINST}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKCU "${UNINST}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST}" "NoRepair" 1
  SectionGetSize 0 $0
  WriteRegDWORD HKCU "${UNINST}" "EstimatedSize" $0
SectionEnd

Section "Uninstall"
  !insertmacro CloseRunning
  ; A copy just closed can hold its .exe a moment longer than it is listed.
  StrCpy $1 0
  ${Do}
    Delete "$INSTDIR\anima-engine.exe"
    ${IfNot} ${FileExists} "$INSTDIR\anima-engine.exe"
      ${Break}
    ${EndIf}
    Sleep 500
    IntOp $1 $1 + 1
  ${LoopUntil} $1 >= 20
  Delete "$INSTDIR\LICENSE.txt"
  Delete "$INSTDIR\THIRD-PARTY-LICENSES.txt"
  Delete "$INSTDIR\README.txt"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\${APP}.lnk"
  DeleteRegKey HKCU "${UNINST}"
  DeleteRegKey HKCU "Software\${APP}"
  ; "Start at login" would start a program that is gone.
  DeleteRegValue HKCU "${RUN}" "${APP}"
SectionEnd
