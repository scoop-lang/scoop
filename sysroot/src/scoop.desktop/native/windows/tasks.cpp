#include "desktop.h"
#include <memory>
#include <propkey.h>
#include <propvarutil.h>
#include <vector>

struct SDTask {
  std::wstring title, description, icon;
  int index;
  std::vector<std::wstring> arguments;
};
struct SDTasks {
  std::wstring id;
  std::vector<SDTask> items;
};

static std::wstring arguments(const std::vector<std::wstring> &values) {
  std::wstring output;
  for (const auto &value : values) {
    if (!output.empty())
      output += L' ';
    output += L'"';
    size_t slashes = 0;
    for (wchar_t character : value) {
      if (character == L'\\') {
        ++slashes;
        continue;
      }
      output.append(character == L'"' ? slashes * 2 + 1 : slashes, L'\\');
      slashes = 0;
      output += character;
    }
    output.append(slashes * 2, L'\\');
    output += L'"';
  }
  return output;
}

static HRESULT executable(std::wstring &path) {
  DWORD capacity = 256;
  for (;;) {
    path.resize(capacity);
    DWORD length = GetModuleFileNameW(nullptr, path.data(), capacity);
    if (!length)
      return HRESULT_FROM_WIN32(GetLastError());
    if (length < capacity) {
      path.resize(length);
      return S_OK;
    }
    if (capacity >= 32768)
      return HRESULT_FROM_WIN32(ERROR_FILENAME_EXCED_RANGE);
    capacity *= 2;
  }
}

static HRESULT shell_link(const SDTask &task, const std::wstring &path,
                          IShellLinkW **result) {
  SDCom<IShellLinkW> link;
  HRESULT status =
      CoCreateInstance(CLSID_ShellLink, nullptr, CLSCTX_INPROC_SERVER,
                       IID_IShellLinkW, reinterpret_cast<void **>(link.put()));
  if (FAILED(status))
    return status;
  std::wstring args = arguments(task.arguments);
  if (args.size() >= 32767 || task.description.size() >= INFOTIPSIZE)
    return E_INVALIDARG;
  status = link->SetPath(path.c_str());
  if (SUCCEEDED(status))
    status = link->SetArguments(args.c_str());
  if (SUCCEEDED(status))
    status = link->SetDescription(task.description.c_str());
  if (SUCCEEDED(status) && !task.icon.empty())
    status = link->SetIconLocation(task.icon.c_str(), task.index);
  SDCom<IPropertyStore> properties;
  if (SUCCEEDED(status))
    status = link->QueryInterface(IID_IPropertyStore,
                                  reinterpret_cast<void **>(properties.put()));
  if (SUCCEEDED(status)) {
    PROPVARIANT title;
    PropVariantInit(&title);
    status = InitPropVariantFromString(task.title.c_str(), &title);
    if (SUCCEEDED(status))
      status = properties->SetValue(PKEY_Title, title);
    PropVariantClear(&title);
    if (SUCCEEDED(status))
      status = properties->Commit();
  }
  if (SUCCEEDED(status)) {
    *result = link.value;
    link.value = nullptr;
  }
  return status;
}

SDResult scoop_desktop_tasks_create(void *pointer) {
  try {
    auto app = static_cast<SDApplication *>(pointer);
    if (app->id.empty())
      return {nullptr, SD_INVALID};
    return {new SDTasks{app->id, {}}, 0};
  } catch (const std::bad_alloc &) {
    return {nullptr, (uint32_t)E_OUTOFMEMORY};
  }
}
int64_t scoop_desktop_tasks_add(void *pointer, const uint8_t *title,
                                int64_t titleLength, const uint8_t *description,
                                int64_t descriptionLength, const uint16_t *icon,
                                int64_t iconLength, int32_t index) {
  try {
    if (iconLength < 0 || iconLength > 32767)
      return SD_INVALID;
    SDTask task;
    if (!sd_wide(title, titleLength, task.title) ||
        !sd_wide(description, descriptionLength, task.description) ||
        task.title.empty())
      return SD_INVALID;
    task.icon.assign(reinterpret_cast<const wchar_t *>(icon),
                     (size_t)iconLength);
    task.index = index;
    static_cast<SDTasks *>(pointer)->items.push_back(std::move(task));
    return 0;
  } catch (const std::bad_alloc &) {
    return (uint32_t)E_OUTOFMEMORY;
  }
}
int64_t scoop_desktop_tasks_argument(void *pointer, const uint8_t *text,
                                     int64_t length) {
  try {
    SDTasks *tasks = static_cast<SDTasks *>(pointer);
    if (tasks->items.empty())
      return SD_INVALID;
    std::wstring value;
    if (!sd_wide(text, length, value))
      return SD_INVALID;
    tasks->items.back().arguments.push_back(std::move(value));
    return 0;
  } catch (const std::bad_alloc &) {
    return (uint32_t)E_OUTOFMEMORY;
  }
}
int64_t scoop_desktop_tasks_commit(void *pointer) {
  try {
    SDTasks *tasks = static_cast<SDTasks *>(pointer);
    SDCom<ICustomDestinationList> list;
    HRESULT status = CoCreateInstance(
        CLSID_DestinationList, nullptr, CLSCTX_INPROC_SERVER,
        IID_ICustomDestinationList, reinterpret_cast<void **>(list.put()));
    if (FAILED(status))
      return (uint32_t)status;
    status = list->SetAppID(tasks->id.c_str());
    if (FAILED(status))
      return (uint32_t)status;
    UINT slots = 0;
    SDCom<IObjectArray> removed;
    status = list->BeginList(&slots, IID_IObjectArray,
                             reinterpret_cast<void **>(removed.put()));
    if (FAILED(status))
      return (uint32_t)status;
    // Abort on all failed preparation paths after BeginList, including
    // allocation failure.
    struct Abort {
      ICustomDestinationList *list;
      bool committed = false;
      ~Abort() {
        if (!committed)
          list->AbortList();
      }
    } abort{list.value};
    if (!tasks->items.empty()) {
      std::wstring path;
      status = executable(path);
      SDCom<IObjectCollection> collection;
      if (SUCCEEDED(status))
        status = CoCreateInstance(CLSID_EnumerableObjectCollection, nullptr,
                                  CLSCTX_INPROC_SERVER, IID_IObjectCollection,
                                  reinterpret_cast<void **>(collection.put()));
      for (const auto &task : tasks->items) {
        if (FAILED(status))
          break;
        SDCom<IShellLinkW> link;
        status = shell_link(task, path, link.put());
        if (SUCCEEDED(status))
          status = collection->AddObject(link.value);
      }
      SDCom<IObjectArray> array;
      if (SUCCEEDED(status))
        status = collection->QueryInterface(
            IID_IObjectArray, reinterpret_cast<void **>(array.put()));
      if (SUCCEEDED(status))
        status = list->AddUserTasks(array.value);
    }
    if (SUCCEEDED(status))
      status = list->CommitList();
    abort.committed = SUCCEEDED(status);
    return (uint32_t)status;
  } catch (const std::bad_alloc &) {
    return (uint32_t)E_OUTOFMEMORY;
  }
}
void scoop_desktop_tasks_destroy(void *pointer) {
  delete static_cast<SDTasks *>(pointer);
}
