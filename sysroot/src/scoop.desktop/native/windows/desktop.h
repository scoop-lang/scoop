#ifndef SCOOP_DESKTOP_WINDOWS_H
#define SCOOP_DESKTOP_WINDOWS_H
#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include "../desktop.h"
#include <windows.h>

#include <commctrl.h>
#include <new>
#include <shobjidl.h>
#include <string>

struct SDApplication {
  std::wstring id;
  ITaskbarList3 *taskbar = nullptr;
  bool com = false;
  ~SDApplication() {
    if (taskbar)
      taskbar->Release();
    if (com)
      CoUninitialize();
  }
};
template <class T> struct SDCom {
  T *value = nullptr;
  SDCom() = default;
  explicit SDCom(T *pointer) : value(pointer) {
    if (value)
      value->AddRef();
  }
  SDCom(const SDCom &) = delete;
  SDCom &operator=(const SDCom &) = delete;
  ~SDCom() {
    if (value)
      value->Release();
  }
  T **put() { return &value; }
  T *operator->() const { return value; }
};
bool sd_wide(const uint8_t *text, int64_t length, std::wstring &result);
HICON sd_icon(SDImage *image, int width, int height);

#endif
