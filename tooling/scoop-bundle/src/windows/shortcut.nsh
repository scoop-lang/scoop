Function SetShortcutIdentity
  Exch $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  Push $6
  Push $7
  StrCpy $1 0
  StrCpy $2 0
  StrCpy $3 0
  StrCpy $4 0
  StrCpy $5 0
  System::Call 'ole32::CoInitializeEx(p 0, i 2) i.r6'
  ${If} $6 < 0
    StrCpy $5 $6
    Goto identity_done
  ${EndIf}
  System::Call 'ole32::CoCreateInstance(g "{00021401-0000-0000-C000-000000000046}", p 0, i 1, g "{0000010b-0000-0000-C000-000000000046}", *p.r1) i.r5'
  ${If} $5 < 0
    Goto identity_done
  ${EndIf}
  System::Call '$1->5(w r0, i 2) i.r5'
  ${If} $5 < 0
    Goto identity_done
  ${EndIf}
  System::Call '$1->0(g "{886d8eeb-8cf2-4446-8d02-cdba1dbdcf99}", *p.r2) i.r5'
  ${If} $5 < 0
    Goto identity_done
  ${EndIf}
  System::Alloc 20
  Pop $3
  System::Alloc 24
  Pop $4
  ${If} $4 != 0
    System::Call '*$4(i 0, i 0, l 0, l 0)'
  ${EndIf}
  ${If} $3 == 0
  ${OrIf} $4 == 0
    StrCpy $5 -2147024882
    Goto identity_done
  ${EndIf}
  System::Call 'propsys::PSGetPropertyKeyFromName(w "System.AppUserModel.ID", p r3) i.r5'
  ${If} $5 < 0
    Goto identity_done
  ${EndIf}
  ; InitPropVariantFromString is an SDK inline helper, not a DLL export.
  ; SHStrDupW allocates the VT_LPWSTR value with the allocator used by PropVariantClear.
  System::Call 'shlwapi::SHStrDupW(w @SCOOP_APPID@, *p.r7) i.r5'
  ${If} $5 < 0
    Goto identity_done
  ${EndIf}
  System::Call '*$4(i 31, i 0, p r7)'
  System::Call '$2->6(p r3, p r4) i.r5'
  ${If} $5 < 0
    Goto identity_done
  ${EndIf}
@SCOOP_ACTIVATOR@
  System::Call '$2->7() i.r5'
  ${If} $5 < 0
    Goto identity_done
  ${EndIf}
  System::Call '$1->6(w r0, i 1) i.r5'
identity_done:
  ${If} $4 != 0
    System::Call 'ole32::PropVariantClear(p r4)'
    System::Free $4
  ${EndIf}
  ${If} $3 != 0
    System::Free $3
  ${EndIf}
  ${If} $2 != 0
    System::Call '$2->2()'
  ${EndIf}
  ${If} $1 != 0
    System::Call '$1->2()'
  ${EndIf}
  ${If} $6 >= 0
    System::Call 'ole32::CoUninitialize()'
  ${EndIf}
  StrCpy $0 $5
  Pop $7
  Pop $6
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Exch $0
FunctionEnd
