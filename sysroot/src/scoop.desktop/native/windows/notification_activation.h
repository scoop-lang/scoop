#ifndef SCOOP_NOTIFICATION_ACTIVATION_H
#define SCOOP_NOTIFICATION_ACTIVATION_H

#if __has_include(<notificationactivationcallback.h>)
#include <notificationactivationcallback.h>
#else
#include <unknwn.h>

// LLVM-MinGW omits this Windows SDK COM declaration.
struct NOTIFICATION_USER_INPUT_DATA {
  LPCWSTR Key;
  LPCWSTR Value;
};

MIDL_INTERFACE("53E31837-6600-4A81-9395-75CFFE746F94")
INotificationActivationCallback : public IUnknown {
  virtual HRESULT STDMETHODCALLTYPE Activate(
      LPCWSTR appUserModelId, LPCWSTR invokedArgs,
      const NOTIFICATION_USER_INPUT_DATA *data, ULONG count) = 0;
};

__CRT_UUID_DECL(INotificationActivationCallback, 0x53e31837, 0x6600, 0x4a81,
                0x93, 0x95, 0x75, 0xcf, 0xfe, 0x74, 0x6f, 0x94)
#endif

#endif
