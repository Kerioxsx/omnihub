; OmniHub's installer hooks (bundle > windows > nsis > installerHooks).

!macro NSIS_HOOK_PREINSTALL
  ; OmniHub installs for the current user, so this installer runs without
  ; administrator rights. A copy that sits in Program Files (or any folder
  ; only administrators can change) cannot be replaced that way: copying
  ; would stop with "Error writing to file". Ask for approval instead and
  ; run this installer again, elevated, into the same folder.
  Push $0
  Push $1
  ClearErrors
  FileOpen $0 "$INSTDIR\.omnihub-write-test" w
  ${If} ${Errors}
    ${GetOptions} $CMDLINE "/OHELEVATED" $1
    ${IfNot} ${Errors}
      ; Already the elevated run, and still not allowed.
      MessageBox MB_ICONSTOP "OmniHub could not write to $INSTDIR, even with administrator rights.$\r$\n$\r$\nUninstall OmniHub in Windows Settings > Apps (your data is kept), then run this setup again." /SD IDOK
      Pop $1
      Pop $0
      Abort
    ${EndIf}
    MessageBox MB_OKCANCEL|MB_ICONINFORMATION "OmniHub is installed in $INSTDIR, which only an administrator can change.$\r$\n$\r$\nClick OK and approve the Windows prompt to update it there.$\r$\n$\r$\nTip: to update without administrator rights in the future, uninstall OmniHub in Windows Settings > Apps (your data is kept) and run this setup again. It then installs into your own user folder." /SD IDOK IDOK omnihub_elevate
    Pop $1
    Pop $0
    Abort
    omnihub_elevate:
    ; Passive (a progress bar) or silent like this run, start OmniHub when
    ; done, and the folder last and unquoted as NSIS wants it.
    ${If} ${Silent}
      StrCpy $1 "/S /R /OHELEVATED"
    ${Else}
      StrCpy $1 "/P /R /OHELEVATED"
    ${EndIf}
    ${IfThen} $UpdateMode = 1 ${|} StrCpy $1 "$1 /UPDATE" ${|}
    ${IfThen} $NoShortcutMode = 1 ${|} StrCpy $1 "$1 /NS" ${|}
    ClearErrors
    ExecShell "runas" "$EXEPATH" "$1 /D=$INSTDIR"
    ${If} ${Errors}
      MessageBox MB_ICONEXCLAMATION "Windows did not give administrator approval, so OmniHub was not updated.$\r$\n$\r$\nRun this setup again and approve the prompt, or uninstall OmniHub in Windows Settings > Apps (your data is kept) and run this setup again to install it into your own user folder." /SD IDOK
      Pop $1
      Pop $0
      Abort
    ${EndIf}
    ; The elevated run takes over from here.
    Pop $1
    Pop $0
    SetErrorLevel 0
    Quit
  ${EndIf}
  FileClose $0
  Delete "$INSTDIR\.omnihub-write-test"

  ; Close every OmniHub still running, wherever it runs from: the app
  ; (it hides to the tray when asked to close), a copy in another folder,
  ; and the helper Brave, Chrome or Edge start for the OmniHub extension.
  ; Any of them would keep files locked.
  nsExec::Exec 'cmd.exe /C tasklist /FI "IMAGENAME eq omnihub.exe" /NH | find "omnihub.exe"'
  Pop $1
  ${If} $1 == 0
    ${IfNot} ${Silent}
    ${AndIf} $PassiveMode <> 1
      MessageBox MB_OKCANCEL|MB_ICONINFORMATION "OmniHub is running. Click OK to close it and continue." IDOK omnihub_close
      Pop $1
      Pop $0
      Abort
    ${EndIf}
    omnihub_close:
    nsExec::Exec 'taskkill.exe /F /T /IM omnihub.exe'
    Pop $1
    Sleep 1000
  ${EndIf}
  Pop $1
  Pop $0
!macroend
