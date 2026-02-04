; RECALL.OS NSIS Installer Hooks
; Improves bitmap scaling quality on high-DPI displays

!include "LogicLib.nsh"

; Constants for SetStretchBltMode
!define HALFTONE 4

; Hook that runs before installation starts
; We use this to set up better bitmap scaling
!macro NSIS_HOOK_PREINSTALL
    ; Get the device context for the installer window
    System::Call 'user32::GetDC(p $HWNDPARENT) p .r0'
    ${If} $0 != 0
        ; Set stretch mode to HALFTONE for high-quality bitmap scaling
        ; This makes scaled images look smoother instead of pixelated
        System::Call 'gdi32::SetStretchBltMode(p r0, i ${HALFTONE}) i .r1'

        ; Release the device context
        System::Call 'user32::ReleaseDC(p $HWNDPARENT, p r0)'
    ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
    ; Nothing needed here
!macroend

!macro NSIS_HOOK_PREUNINSTALL
    ; Nothing needed here
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
    ; Nothing needed here
!macroend
