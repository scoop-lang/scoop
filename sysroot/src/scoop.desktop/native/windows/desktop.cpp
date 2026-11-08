#include "desktop.h"
#include <limits.h>
#include <memory>

extern "C" int32_t scoop_window_windows_taskbar_ready(HWND window);

struct SDWindow {
  SDApplication *application;
  HWND window;
  UINT created;
  SDImage *image = nullptr, *overlayImage = nullptr;
  HICON smallIcon = nullptr, largeIcon = nullptr, overlay = nullptr;
  std::wstring description, caption;
  bool ready = false, iconSet = false, overlaySet = false, captionSet = false,
       progressSet = false;
  bool progressApplied = false, captionApplied = false;
  int32_t progress = 0;
  uint64_t completed = 0, total = 0;
  ~SDWindow() {
    if (smallIcon)
      DestroyIcon(smallIcon);
    if (largeIcon)
      DestroyIcon(largeIcon);
    if (overlay)
      DestroyIcon(overlay);
    scoop_desktop_image_destroy(image);
    scoop_desktop_image_destroy(overlayImage);
  }
};

bool sd_wide(const uint8_t *text, int64_t length, std::wstring &result) {
  if (length < 0 || length > INT_MAX)
    return false;
  if (!length) {
    result.clear();
    return true;
  }
  int count = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS,
                                  reinterpret_cast<const char *>(text),
                                  (int)length, nullptr, 0);
  if (!count)
    return false;
  result.resize(count);
  return MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS,
                             reinterpret_cast<const char *>(text), (int)length,
                             result.data(), count) == count;
}

static HRESULT taskbar(SDApplication *app) {
  if (app->taskbar)
    return S_OK;
  HRESULT status = CoCreateInstance(CLSID_TaskbarList, nullptr,
                                    CLSCTX_INPROC_SERVER, IID_ITaskbarList3,
                                    reinterpret_cast<void **>(&app->taskbar));
  if (FAILED(status))
    return status;
  status = app->taskbar->HrInit();
  if (FAILED(status)) {
    app->taskbar->Release();
    app->taskbar = nullptr;
  }
  return status;
}
static HRESULT progress(SDWindow *window, ITaskbarList3 *bar) {
  static const TBPFLAG flags[] = {TBPF_NOPROGRESS, TBPF_INDETERMINATE,
                                  TBPF_NORMAL, TBPF_PAUSED, TBPF_ERROR};
  HRESULT status =
      bar->SetProgressState(window->window, flags[window->progress]);
  if (SUCCEEDED(status) && window->progress >= 2)
    status =
        bar->SetProgressValue(window->window, window->completed, window->total);
  window->progressApplied = SUCCEEDED(status);
  return status;
}
static HRESULT replay(SDWindow *window) {
  window->progressApplied = false;
  window->captionApplied = false;
  HRESULT status = taskbar(window->application);
  if (FAILED(status))
    return status;
  SDCom<ITaskbarList3> bar(window->application->taskbar);
  if (window->progressSet)
    status = progress(window, bar.value);
  if (window->overlaySet) {
    HRESULT next = bar->SetOverlayIcon(window->window, window->overlay,
                                       window->description.c_str());
    if (FAILED(next))
      status = next;
  }
  if (window->captionSet) {
    HRESULT next = bar->SetThumbnailTooltip(
        window->window,
        window->caption.empty() ? nullptr : window->caption.c_str());
    window->captionApplied = SUCCEEDED(next);
    if (FAILED(next))
      status = next;
  }
  return status;
}
static int64_t publish_icon(SDWindow *window, SDImage *image) {
  UINT dpi = GetDpiForWindow(window->window);
  HICON smallIcon =
      image ? sd_icon(image, GetSystemMetricsForDpi(SM_CXSMICON, dpi),
                      GetSystemMetricsForDpi(SM_CYSMICON, dpi))
            : nullptr;
  if (image && !smallIcon)
    return GetLastError();
  HICON largeIcon = image
                        ? sd_icon(image, GetSystemMetricsForDpi(SM_CXICON, dpi),
                                  GetSystemMetricsForDpi(SM_CYICON, dpi))
                        : nullptr;
  if (image && !largeIcon) {
    DWORD error = GetLastError();
    DestroyIcon(smallIcon);
    return error;
  }
  SendMessageW(window->window, WM_SETICON, ICON_SMALL,
               reinterpret_cast<LPARAM>(smallIcon));
  SendMessageW(window->window, WM_SETICON, ICON_BIG,
               reinterpret_cast<LPARAM>(largeIcon));
  if (window->smallIcon)
    DestroyIcon(window->smallIcon);
  if (window->largeIcon)
    DestroyIcon(window->largeIcon);
  window->smallIcon = smallIcon;
  window->largeIcon = largeIcon;
  window->iconSet = true;
  return 0;
}
static LRESULT CALLBACK window_proc(HWND hwnd, UINT message, WPARAM wp,
                                    LPARAM lp, UINT_PTR id, DWORD_PTR data) {
  SDWindow *window = reinterpret_cast<SDWindow *>(data);
  LRESULT result = DefSubclassProc(hwnd, message, wp, lp);
  if (message == window->created) {
    window->ready = true;
    // Explorer has recreated this button; the previous COM proxy may have
    // disconnected.
    if (window->application->taskbar) {
      window->application->taskbar->Release();
      window->application->taskbar = nullptr;
    }
    if (window->iconSet) {
      SendMessageW(hwnd, WM_SETICON, ICON_SMALL,
                   reinterpret_cast<LPARAM>(window->smallIcon));
      SendMessageW(hwnd, WM_SETICON, ICON_BIG,
                   reinterpret_cast<LPARAM>(window->largeIcon));
    }
    replay(window);
  } else if (message == WM_DPICHANGED) {
    if (window->iconSet)
      publish_icon(window, window->image);
    if (window->overlayImage) {
      UINT dpi = GetDpiForWindow(hwnd);
      HICON image = sd_icon(window->overlayImage,
                            GetSystemMetricsForDpi(SM_CXSMICON, dpi),
                            GetSystemMetricsForDpi(SM_CYSMICON, dpi));
      if (image) {
        if (window->overlay)
          DestroyIcon(window->overlay);
        window->overlay = image;
        if (window->ready)
          replay(window);
      }
    }
  } else if (message == WM_NCDESTROY) {
    window->window = nullptr;
    RemoveWindowSubclass(hwnd, window_proc, id);
  }
  return result;
}

SDResult scoop_desktop_application_create(SDCallback callback, void *context,
                                          const uint8_t *id, int64_t length) {
  try {
    auto app = std::make_unique<SDApplication>();
    if (!sd_wide(id, length, app->id))
      return {nullptr, SD_INVALID};
    HRESULT status = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
    if (FAILED(status))
      return {nullptr, (uint32_t)status};
    app->com = true;
    return {app.release(), 0};
  } catch (const std::bad_alloc &) {
    return {nullptr, (uint32_t)E_OUTOFMEMORY};
  }
}
int32_t scoop_desktop_application_capabilities(void *app) { return 8; }
int32_t scoop_desktop_application_busy(void *app) { return 0; }
void scoop_desktop_application_cancel(void *app) { (void)app; }
void scoop_desktop_application_destroy(void *app) {
  delete static_cast<SDApplication *>(app);
}
int64_t scoop_desktop_application_icon(void *app, SDImage *image) {
  return SD_UNSUPPORTED;
}
int64_t scoop_desktop_application_badge(void *app, const uint8_t *text,
                                        int64_t length) {
  return SD_UNSUPPORTED;
}
int64_t scoop_desktop_application_menu(void *app, void *menu) {
  return SD_UNSUPPORTED;
}

SDResult scoop_desktop_window_create(void *app, void *native) {
  auto window = std::unique_ptr<SDWindow>(new (std::nothrow) SDWindow());
  if (!window)
    return {nullptr, (uint32_t)E_OUTOFMEMORY};
  window->application = static_cast<SDApplication *>(app);
  window->window = static_cast<HWND>(native);
  window->created = RegisterWindowMessageW(L"TaskbarButtonCreated");
  window->ready = scoop_window_windows_taskbar_ready(window->window) != 0;
  if (!SetWindowSubclass(window->window, window_proc,
                         reinterpret_cast<UINT_PTR>(window.get()),
                         reinterpret_cast<DWORD_PTR>(window.get())))
    return {nullptr, GetLastError()};
  return {window.release(), 0};
}
int32_t scoop_desktop_window_capabilities(void *window) {
  return 1 | 2 | 4 | 16;
}
void scoop_desktop_window_destroy(void *pointer) {
  auto owner = std::unique_ptr<SDWindow>(static_cast<SDWindow *>(pointer));
  if (!owner->window)
    return;
  RemoveWindowSubclass(owner->window, window_proc,
                       reinterpret_cast<UINT_PTR>(owner.get()));
  if (owner->iconSet) {
    SendMessageW(owner->window, WM_SETICON, ICON_SMALL, 0);
    SendMessageW(owner->window, WM_SETICON, ICON_BIG, 0);
  }
  if (owner->ready && SUCCEEDED(taskbar(owner->application))) {
    SDCom<ITaskbarList3> bar(owner->application->taskbar);
    if (owner->progressSet)
      bar->SetProgressState(owner->window, TBPF_NOPROGRESS);
    if (owner->overlaySet)
      bar->SetOverlayIcon(owner->window, nullptr, L"");
    if (owner->captionSet)
      bar->SetThumbnailTooltip(owner->window, nullptr);
  }
}
int64_t scoop_desktop_window_icon(void *pointer, SDImage *image) {
  SDWindow *window = static_cast<SDWindow *>(pointer);
  int64_t status = publish_icon(window, image);
  if (status)
    return status;
  SDImage *previous = window->image;
  window->image = sd_retain_image(image);
  scoop_desktop_image_destroy(previous);
  return 0;
}
int64_t scoop_desktop_window_overlay(void *pointer, SDImage *image,
                                     const uint8_t *text, int64_t length) {
  try {
    SDWindow *window = static_cast<SDWindow *>(pointer);
    std::wstring description;
    if (!sd_wide(text, length, description))
      return SD_INVALID;
    UINT dpi = GetDpiForWindow(window->window);
    HICON icon = image
                     ? sd_icon(image, GetSystemMetricsForDpi(SM_CXSMICON, dpi),
                               GetSystemMetricsForDpi(SM_CYSMICON, dpi))
                     : nullptr;
    if (image && !icon)
      return GetLastError();
    if (window->overlay)
      DestroyIcon(window->overlay);
    window->overlay = icon;
    SDImage *previous = window->overlayImage;
    window->overlayImage = sd_retain_image(image);
    scoop_desktop_image_destroy(previous);
    window->description = std::move(description);
    window->overlaySet = true;
    if (!window->ready)
      return SD_REQUESTED;
    HRESULT status = taskbar(window->application);
    if (SUCCEEDED(status)) {
      SDCom<ITaskbarList3> bar(window->application->taskbar);
      status = bar->SetOverlayIcon(window->window, icon,
                                   window->description.c_str());
    }
    return (uint32_t)status;
  } catch (const std::bad_alloc &) {
    return (uint32_t)E_OUTOFMEMORY;
  }
}
int64_t scoop_desktop_window_progress(void *pointer, int32_t state,
                                      uint64_t completed, uint64_t total) {
  if (state < 0 || state > 4 || (state >= 2 && (!total || completed > total)))
    return SD_INVALID;
  SDWindow *window = static_cast<SDWindow *>(pointer);
  if (window->progressSet && window->progress == state &&
      window->completed == completed && window->total == total &&
      (!window->ready || window->progressApplied))
    return window->ready ? 0 : SD_REQUESTED;
  window->progress = state;
  window->completed = completed;
  window->total = total;
  window->progressSet = true;
  window->progressApplied = false;
  if (!window->ready)
    return SD_REQUESTED;
  HRESULT status = taskbar(window->application);
  if (SUCCEEDED(status)) {
    SDCom<ITaskbarList3> bar(window->application->taskbar);
    status = progress(window, bar.value);
  }
  return (uint32_t)status;
}
int64_t scoop_desktop_window_caption(void *pointer, int32_t thumbnail,
                                     const uint8_t *text, int64_t length) {
  if (!thumbnail)
    return SD_UNSUPPORTED;
  try {
    SDWindow *window = static_cast<SDWindow *>(pointer);
    std::wstring value;
    if (!sd_wide(text, length, value))
      return SD_INVALID;
    if (window->captionSet && window->caption == value &&
        (!window->ready || window->captionApplied))
      return window->ready ? 0 : SD_REQUESTED;
    window->caption = std::move(value);
    window->captionSet = true;
    window->captionApplied = false;
    if (!window->ready)
      return SD_REQUESTED;
    HRESULT status = taskbar(window->application);
    if (SUCCEEDED(status)) {
      SDCom<ITaskbarList3> bar(window->application->taskbar);
      status = bar->SetThumbnailTooltip(
          window->window,
          window->caption.empty() ? nullptr : window->caption.c_str());
    }
    window->captionApplied = SUCCEEDED(status);
    return (uint32_t)status;
  } catch (const std::bad_alloc &) {
    return (uint32_t)E_OUTOFMEMORY;
  }
}
