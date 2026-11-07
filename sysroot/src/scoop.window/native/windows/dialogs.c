#define COBJMACROS
#include "../dialogs.h"
#include "window.h"
#include <shlobj.h>
#include <shobjidl.h>

typedef struct {
  SWApplication *application;
  SWWindow *owner;
  IFileDialog *dialog;
  WCHAR **paths;
  WCHAR *filter, *extension;
  int kind, outcome;
  int64_t count, error;
  bool com_initialized, started, cancelled;
} SWFileDialog;

void scoop_window_file_release(SWFileDialog *session) {
  if (session->dialog)
    IFileDialog_Release(session->dialog);
  for (int64_t index = 0; index < session->count; ++index)
    CoTaskMemFree(session->paths[index]);
  free(session->paths);
  free(session->filter);
  free(session->extension);
  if (session->com_initialized)
    CoUninitialize();
  free(session);
}
SWResult scoop_window_file_create(SWApplication *application, SWWindow *owner,
                                  int32_t kind) {
  SWFileDialog *session = calloc(1, sizeof(*session));
  if (!session)
    return (SWResult){NULL, E_OUTOFMEMORY};
  session->application = application;
  session->owner = owner;
  session->kind = kind;
  HRESULT hr = CoInitializeEx(NULL, COINIT_APARTMENTTHREADED);
  if (SUCCEEDED(hr)) {
    session->com_initialized = true;
    hr = CoCreateInstance(
        kind == 2 ? &CLSID_FileSaveDialog : &CLSID_FileOpenDialog, NULL,
        CLSCTX_INPROC_SERVER, &IID_IFileDialog, (void **)&session->dialog);
  }
  FILEOPENDIALOGOPTIONS options = 0;
  if (SUCCEEDED(hr))
    hr = IFileDialog_GetOptions(session->dialog, &options);
  if (SUCCEEDED(hr)) {
    options |= FOS_FORCEFILESYSTEM | FOS_NOCHANGEDIR | FOS_PATHMUSTEXIST;
    if (kind == 0 || kind == 1)
      options |= FOS_FILEMUSTEXIST;
    if (kind == 1)
      options |= FOS_ALLOWMULTISELECT;
    if (kind == 2)
      options |= FOS_OVERWRITEPROMPT;
    if (kind == 3)
      options |= FOS_PICKFOLDERS;
    hr = IFileDialog_SetOptions(session->dialog, options);
  }
  if (FAILED(hr)) {
    scoop_window_file_release(session);
    return (SWResult){NULL, hr};
  }
  return (SWResult){session, 0};
}

static WCHAR *copy_units(const uint8_t *data, int64_t length) {
  WCHAR *copy = calloc((size_t)length / sizeof(WCHAR) + 1, sizeof(WCHAR));
  if (copy)
    memcpy(copy, data, (size_t)length);
  return copy;
}
int64_t scoop_window_file_option(SWFileDialog *session, int32_t field,
                                 const uint8_t *data, int64_t length) {
  WCHAR *text = field == 0 || field == 3 ? sw_wide(data, length)
                                         : copy_units(data, length);
  if (!text)
    return E_OUTOFMEMORY;
  HRESULT hr = S_OK;
  if (!field)
    hr = IFileDialog_SetTitle(session->dialog, text);
  else if (field == 1) {
    IShellItem *folder = NULL;
    hr = SHCreateItemFromParsingName(text, NULL, &IID_IShellItem,
                                     (void **)&folder);
    if (SUCCEEDED(hr)) {
      hr = IFileDialog_SetDefaultFolder(session->dialog, folder);
      IShellItem_Release(folder);
    } else if (hr == HRESULT_FROM_WIN32(ERROR_FILE_NOT_FOUND) ||
               hr == HRESULT_FROM_WIN32(ERROR_PATH_NOT_FOUND))
      hr = S_OK;
  } else if (field == 2)
    hr = IFileDialog_SetFileName(session->dialog, text);
  else if (field == 3) {
    size_t previous = session->filter ? wcslen(session->filter) : 0;
    size_t size = previous + wcslen(text) + 4;
    WCHAR *filter = calloc(size, sizeof(WCHAR));
    if (!filter)
      hr = E_OUTOFMEMORY;
    else {
      size_t cursor = 0;
      if (session->filter) {
        memcpy(filter, session->filter, previous * sizeof(WCHAR));
        cursor = previous;
        filter[cursor++] = L';';
      }
      filter[cursor++] = L'*';
      filter[cursor++] = L'.';
      memcpy(filter + cursor, text, (wcslen(text) + 1) * sizeof(WCHAR));
      free(session->filter);
      session->filter = filter;
      if (!session->extension) {
        session->extension = text;
        text = NULL;
      }
    }
  } else
    hr = E_INVALIDARG;
  free(text);
  return hr;
}

static HRESULT collect_paths(SWFileDialog *session) {
  if (session->kind != 1) {
    IShellItem *item = NULL;
    HRESULT hr = IFileDialog_GetResult(session->dialog, &item);
    if (FAILED(hr))
      return hr;
    session->paths = calloc(1, sizeof(*session->paths));
    if (!session->paths) {
      IShellItem_Release(item);
      return E_OUTOFMEMORY;
    }
    session->count = 1;
    hr = IShellItem_GetDisplayName(item, SIGDN_FILESYSPATH, &session->paths[0]);
    IShellItem_Release(item);
    return hr;
  }
  IFileOpenDialog *open = NULL;
  HRESULT hr = IFileDialog_QueryInterface(session->dialog, &IID_IFileOpenDialog,
                                          (void **)&open);
  if (FAILED(hr))
    return hr;
  IShellItemArray *items = NULL;
  hr = IFileOpenDialog_GetResults(open, &items);
  IFileOpenDialog_Release(open);
  if (FAILED(hr))
    return hr;
  DWORD count = 0;
  hr = IShellItemArray_GetCount(items, &count);
  if (SUCCEEDED(hr)) {
    session->paths = calloc(count ? count : 1, sizeof(*session->paths));
    if (!session->paths)
      hr = E_OUTOFMEMORY;
    else
      session->count = count;
  }
  for (DWORD index = 0; SUCCEEDED(hr) && index < count; ++index) {
    IShellItem *item = NULL;
    hr = IShellItemArray_GetItemAt(items, index, &item);
    if (SUCCEEDED(hr)) {
      hr = IShellItem_GetDisplayName(item, SIGDN_FILESYSPATH,
                                     &session->paths[index]);
      IShellItem_Release(item);
    }
  }
  IShellItemArray_Release(items);
  return hr;
}

void scoop_window_file_start(SWFileDialog *session) {
  if (session->outcome)
    return;
  HRESULT hr = S_OK;
  if (session->filter) {
    COMDLG_FILTERSPEC filter = {session->filter, session->filter};
    hr = IFileDialog_SetFileTypes(session->dialog, 1, &filter);
    if (SUCCEEDED(hr) && session->kind == 2)
      hr = IFileDialog_SetDefaultExtension(session->dialog, session->extension);
  }
  if (SUCCEEDED(hr)) {
    session->started = true;
    hr = IFileDialog_Show(session->dialog, session->owner
                                               ? session->owner->hwnd
                                               : session->application->hwnd);
  }
  if (session->owner && session->owner->input_blocked)
    EnableWindow(session->owner->hwnd, FALSE);
  if (session->cancelled || hr == HRESULT_FROM_WIN32(ERROR_CANCELLED))
    session->outcome = 2;
  else {
    if (SUCCEEDED(hr))
      hr = collect_paths(session);
    if (FAILED(hr)) {
      session->error = hr;
      session->outcome = 3;
    } else
      session->outcome = 1;
  }
  sw_wake(session->application);
}
int32_t scoop_window_file_cancel(SWFileDialog *session) {
  if (session->outcome || session->cancelled)
    return 0;
  session->cancelled = true;
  if (session->started)
    IFileDialog_Close(session->dialog, HRESULT_FROM_WIN32(ERROR_CANCELLED));
  else
    session->outcome = 2;
  sw_wake(session->application);
  return 1;
}
SWFileResult scoop_window_file_result(SWFileDialog *session) {
  return (SWFileResult){session->outcome, session->error, session->count,
                        (const uint8_t *)"HRESULT", 7};
}
SWFilePath scoop_window_file_path(SWFileDialog *session, int64_t index) {
  const WCHAR *path = session->paths[index];
  return (SWFilePath){(const uint8_t *)path,
                      (int64_t)(wcslen(path) * sizeof(WCHAR))};
}
