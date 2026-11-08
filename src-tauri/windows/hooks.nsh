; Achaja - hooks for the Tauri NSIS installer.
;
; Everything Achaja writes lives in its install folder (<install>\data), never in the user
; profile, so the folder is never inside %USERPROFILE% (AppData included):
;   1. Install folder: the Python version's folder when it is installed outside the profile,
;      otherwise <system drive>\Banzamel\Achaja. Tauri updates reuse the current folder.
;   2. The Python version (Inno Setup, up to 0.13.x) is replaced: its settings and data are copied
;      into <install>\data, it is uninstalled silently, and only when it is really gone are its
;      leftovers removed (achaja-migrate.nsh). Its self-updater runs this installer with Inno's
;      /SILENT or /VERYSILENT, which become NSIS' passive or silent mode here.
;   3. Uninstall deletes data\ and backups\ only when "Delete the application data" is ticked;
;      updates never delete them.
; Note for editors: makensis joins a line that ends in a backslash with the next one.

!include LogicLib.nsh
!include FileFunc.nsh
!include "${__FILEDIR__}\achaja-migrate.nsh"

; Before the pages: the Python updater's flags, and the safe folder for the directory page
!define MUI_CUSTOMFUNCTION_GUIINIT AchajaGuiInit
Function AchajaGuiInit
  Call AchajaInnoFlags
  Call AchajaFixInstDir
FunctionEnd

!macro NSIS_HOOK_PREINSTALL
  ; A silent install skips the pages, so the folder is checked here too. The template has already
  ; created its default folder (SetOutPath); it is removed again when the install goes elsewhere.
  StrCpy $AchajaOrigDir $INSTDIR
  Call AchajaFixInstDir
  ${If} $AchajaOrigDir != $INSTDIR
    RMDir $AchajaOrigDir
  ${EndIf}
  SetOutPath $INSTDIR
  Call AchajaMigratePython
  SetOutPath $INSTDIR
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; The language chosen for the installer becomes Achaja's first language
  ${IfNot} ${FileExists} "$INSTDIR\data\config.json"
    CreateDirectory "$INSTDIR\data"
    FileOpen $R6 "$INSTDIR\data\install-language" w
    ${If} $LANGUAGE == 1045
      FileWrite $R6 "pl"
    ${Else}
      FileWrite $R6 "en"
    ${EndIf}
    FileClose $R6
  ${EndIf}
  ; Settings have their own Start menu entry, like before (it opens the window on Settings)
  ${If} $LANGUAGE == 1045
    CreateShortCut "$SMPROGRAMS\Achaja - ustawienia.lnk" "$INSTDIR\${MAINBINARYNAME}.exe" "--settings" "$INSTDIR\${MAINBINARYNAME}.exe" 0
  ${Else}
    CreateShortCut "$SMPROGRAMS\Achaja - settings.lnk" "$INSTDIR\${MAINBINARYNAME}.exe" "--settings" "$INSTDIR\${MAINBINARYNAME}.exe" 0
  ${EndIf}
  ; The Python version started with Windows: so does this one (an entry in HKCU\...\Run)
  ${If} $AchajaAutostart == 1
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Achaja" '"$INSTDIR\${MAINBINARYNAME}.exe" --minimized'
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ${If} $UpdateMode <> 1
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Achaja"
    Delete "$SMPROGRAMS\Achaja - ustawienia.lnk"
    Delete "$SMPROGRAMS\Achaja - settings.lnk"
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    RMDir /r "$INSTDIR\resources"
    ${If} $DeleteAppDataCheckboxState = 1
      RMDir /r "$INSTDIR\data"
      RMDir /r "$INSTDIR\backups"
    ${EndIf}
  ${EndIf}
  RMDir "$INSTDIR"
!macroend
