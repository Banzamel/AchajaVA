; Achaja - the install folder and the move from the Python version (included by hooks.nsh).
; Note for editors: makensis joins a line that ends in a backslash with the next one.

!define ACHAJA_OLD_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\{6F0B7C2E-4B7A-4E0C-9C55-3A1C4E7D2B90}_is1"

; The template's default folder, before AchajaFixInstDir redirects the install
Var AchajaOrigDir
; The Python version started with Windows (a shortcut in the Startup folder)
Var AchajaAutostart

; The Python version's updater starts the new installer the Inno Setup way: /SILENT (a progress
; window, then Achaja starts again when /relaunch=1) or /VERYSILENT (on quit, nothing shown).
; NSIS knows neither, so the installer starts itself again in passive (/P) or silent (/S) mode.
Function AchajaInnoFlags
  ${GetParameters} $R0
  ClearErrors
  ${GetOptions} $R0 "/VERYSILENT" $R1
  ${IfNot} ${Errors}
    Exec '"$EXEPATH" /S'
    Quit
  ${EndIf}
  ClearErrors
  ${GetOptions} $R0 "/SILENT" $R1
  ${IfNot} ${Errors}
    StrCpy $R2 "0"
    ClearErrors
    ${GetOptions} $R0 "/relaunch=" $R2
    ${If} $R2 == "1"
      Exec '"$EXEPATH" /P /R'
    ${Else}
      Exec '"$EXEPATH" /P'
    ${EndIf}
    Quit
  ${EndIf}
FunctionEnd

; Out: $R4 = UninstallString of the Python version ("" when it is not installed),
;      $R5 = its folder without the trailing backslash
Function AchajaOldDir
  ReadRegStr $R4 HKCU "${ACHAJA_OLD_KEY}" "UninstallString"
  ReadRegStr $R5 HKCU "${ACHAJA_OLD_KEY}" "InstallLocation"
  ${If} $R4 == ""
    ReadRegStr $R4 HKLM "${ACHAJA_OLD_KEY}" "UninstallString"
    ReadRegStr $R5 HKLM "${ACHAJA_OLD_KEY}" "InstallLocation"
  ${EndIf}
  ${If} $R4 != ""
  ${AndIf} $R5 == ""
    StrCpy $R5 $R4 "" 1
    StrCpy $R5 $R5 -1
    ${GetParent} $R5 $R5
  ${EndIf}
  StrCpy $R3 $R5 1 -1
  ${If} $R3 == "\"
    StrCpy $R5 $R5 -1
  ${EndIf}
FunctionEnd

; A folder inside the user profile (the template's default $LOCALAPPDATA\Achaja, AppData, Desktop)
; becomes the Python version's folder when that one is outside the profile and outside Program
; Files (not writable without administrator rights), otherwise <system drive>\Banzamel\Achaja.
Function AchajaFixInstDir
  StrLen $R2 "$PROFILE\"
  StrCpy $R3 "$INSTDIR\" $R2
  ${If} $R3 != "$PROFILE\"
    Return
  ${EndIf}
  Call AchajaOldDir
  StrCpy $R3 "$R5\" $R2
  StrLen $R1 $PROGRAMFILES64
  StrCpy $R1 $R5 $R1
  StrLen $R0 $PROGRAMFILES
  StrCpy $R0 $R5 $R0
  ${If} $R5 != ""
  ${AndIf} ${FileExists} "$R5\*.*"
  ${AndIf} $R3 != "$PROFILE\"
  ${AndIf} $R1 != $PROGRAMFILES64
  ${AndIf} $R0 != $PROGRAMFILES
    StrCpy $INSTDIR $R5
  ${Else}
    StrCpy $R1 $WINDIR 2
    StrCpy $INSTDIR "$R1\Banzamel\Achaja"
  ${EndIf}
FunctionEnd

; Copies a file of the old folder into data\ unless data\ already has it: Push name.
Function AchajaCopyFile
  Exch $R0
  ${If} ${FileExists} "$R5\$R0"
  ${AndIfNot} ${FileExists} "$R6\$R0"
    ${GetParent} "$R6\$R0" $R1
    CreateDirectory $R1
    CopyFiles /SILENT "$R5\$R0" $R1
  ${EndIf}
  Pop $R0
FunctionEnd

; Copies a folder of the old folder into data\ unless data\ already has it: Push name.
Function AchajaCopyDir
  Exch $R0
  ${If} ${FileExists} "$R5\$R0\*.*"
  ${AndIfNot} ${FileExists} "$R6\$R0\*.*"
    CreateDirectory "$R6\$R0"
    CopyFiles /SILENT "$R5\$R0\*.*" "$R6\$R0"
  ${EndIf}
  Pop $R0
FunctionEnd

; The Python version: copy its data into data\, uninstall it, check it is really gone, and only
; then remove what it left (its settings and data, now copies in data\).
Function AchajaMigratePython
  StrCpy $AchajaAutostart 0
  Call AchajaOldDir
  ${If} $R4 == ""
    Return
  ${EndIf}
  DetailPrint "Achaja (Python) $R5 -> $INSTDIR"
  ${If} ${FileExists} "$SMSTARTUP\Achaja.lnk"
    StrCpy $AchajaAutostart 1
  ${EndIf}
  StrCpy $R6 "$INSTDIR\data"
  StrCpy $R7 0
  ${IfNot} ${FileExists} "$R6\*.*"
    StrCpy $R7 1
  ${EndIf}
  CreateDirectory $R6
  Push "config.json"
  Call AchajaCopyFile
  Push ".mcp.json"
  Call AchajaCopyFile
  Push ".claude\settings.local.json"
  Call AchajaCopyFile
  Push ".claude\CLAUDE.local.md"
  Call AchajaCopyFile
  Push "state"
  Call AchajaCopyDir
  Push "models"
  Call AchajaCopyDir
  ; the copy must be there before anything of the old version goes
  ${If} ${FileExists} "$R5\config.json"
  ${AndIfNot} ${FileExists} "$R6\config.json"
    Goto achaja_failed
  ${EndIf}
  ExecWait '$R4 /VERYSILENT /SUPPRESSMSGBOXES /NORESTART'
  ; Inno's uninstaller finishes in a copy of itself: wait until unins000.exe is gone (max 60 s)
  StrCpy $R3 0
  achaja_wait:
    IfFileExists "$R5\unins000.exe" 0 achaja_waited
    IntCmp $R3 120 achaja_waited
    Sleep 500
    IntOp $R3 $R3 + 1
    Goto achaja_wait
  achaja_waited:
  ReadRegStr $R3 HKCU "${ACHAJA_OLD_KEY}" "UninstallString"
  ${If} $R3 == ""
    ReadRegStr $R3 HKLM "${ACHAJA_OLD_KEY}" "UninstallString"
  ${EndIf}
  ${If} ${FileExists} "$R5\unins000.exe"
  ${OrIf} ${FileExists} "$R5\src\achaja\__main__.py"
  ${OrIf} $R3 != ""
    Goto achaja_failed
  ${EndIf}
  ; gone: what it left behind (its settings and data are copies in data\ now)
  Delete "$R5\config.json"
  Delete "$R5\.mcp.json"
  RMDir /r "$R5\state"
  RMDir /r "$R5\models"
  RMDir /r "$R5\.venv"
  RMDir /r "$R5\runtime"
  RMDir /r "$R5\.claude"
  ${If} $R5 != $INSTDIR
    RMDir "$R5"
  ${EndIf}
  Return
  achaja_failed:
    ; the old version stays complete and usable; a data folder made here goes again
    ${If} $R7 == 1
      RMDir /r $R6
    ${EndIf}
    MessageBox MB_ICONSTOP|MB_OK "The previous Achaja (0.13 or older) could not be removed. Close it, uninstall it in Settings > Apps, then run this installer again. Nothing was changed." /SD IDOK
    Abort
FunctionEnd
