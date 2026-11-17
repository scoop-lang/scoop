#include "provider.h"
#include <algorithm>
#include <cmath>
#include <stdexcept>

namespace swax::win {
HRESULT actionResult(int32_t value) {
  switch (value) {
  case 1:
    return S_OK;
  case 2:
    return E_INVALIDARG;
  case 3:
    return UIA_E_NOTSUPPORTED;
  case 4:
    return UIA_E_ELEMENTNOTAVAILABLE;
  case 5:
    return UIA_E_INVALIDOPERATION;
  case 6:
    return RPC_E_SERVERCALL_RETRYLATER;
  default:
    return E_FAIL;
  }
}
std::wstring wide(const std::string &text) {
  if (text.empty())
    return {};
  if (text.size() > INT_MAX)
    throw std::length_error("text exceeds Windows string range");
  int count = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text.data(),
                                  static_cast<int>(text.size()), nullptr, 0);
  if (!count)
    throw std::invalid_argument("invalid UTF-8");
  std::wstring result(static_cast<size_t>(count), 0);
  MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, text.data(),
                      static_cast<int>(text.size()), result.data(), count);
  return result;
}
std::optional<std::string> utf8(const wchar_t *text, int length) {
  if (!text)
    return {};
  if (length == -1) {
    auto count = wcslen(text);
    if (count > INT_MAX)
      return {};
    length = static_cast<int>(count);
  }
  if (!length)
    return std::string();
  int count = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, text, length,
                                  nullptr, 0, nullptr, nullptr);
  if (!count)
    return {};
  std::string result(static_cast<size_t>(count), 0);
  WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, text, length,
                      result.data(), count, nullptr, nullptr);
  return result;
}
IRawElementProviderSimple *simple(const ElementPtr &element) {
  return element ? static_cast<IRawElementProviderSimple *>(new Node(element))
                 : nullptr;
}
SAFEARRAY *elementArray(const std::vector<ElementPtr> &values) {
  if (values.size() > LONG_MAX)
    throw std::length_error("too many elements");
  auto array =
      SafeArrayCreateVector(VT_UNKNOWN, 0, static_cast<ULONG>(values.size()));
  if (!array)
    throw std::bad_alloc();
  try {
    LONG index = 0;
    for (const auto &value : values) {
      auto object = simple(value);
      HRESULT result = SafeArrayPutElement(array, &index, object);
      if (object)
        object->Release();
      if (FAILED(result))
        throw std::bad_alloc();
      ++index;
    }
    return array;
  } catch (...) {
    SafeArrayDestroy(array);
    throw;
  }
}
SAFEARRAY *numberArray(const std::vector<double> &values) {
  if (values.size() > LONG_MAX)
    throw std::length_error("too many coordinates");
  auto array =
      SafeArrayCreateVector(VT_R8, 0, static_cast<ULONG>(values.size()));
  if (!array)
    throw std::bad_alloc();
  for (LONG index = 0; index < static_cast<LONG>(values.size()); ++index) {
    double value = values[index];
    if (FAILED(SafeArrayPutElement(array, &index, &value))) {
      SafeArrayDestroy(array);
      throw std::bad_alloc();
    }
  }
  return array;
}
Node::Node(ElementPtr value)
    : element(std::move(value)), tree(element->tree.lock()) {
  const auto status = CoCreateFreeThreadedMarshaler(
      static_cast<IRawElementProviderSimple *>(this), &marshaler);
  if (FAILED(status))
    throw ComFailure{status};
}
Node::~Node() {
  if (marshaler)
    marshaler->Release();
}
ULONG Node::AddRef() { return ++references; }
ULONG Node::Release() {
  ULONG left = --references;
  if (!left)
    delete this;
  return left;
}
HRESULT Node::QueryInterface(REFIID iid, void **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  if (iid == IID_IUnknown || iid == __uuidof(IRawElementProviderSimple))
    *result = static_cast<IRawElementProviderSimple *>(this);
  else if (iid == __uuidof(IRawElementProviderFragment))
    *result = static_cast<IRawElementProviderFragment *>(this);
  else if (iid == __uuidof(IRawElementProviderFragmentRoot) &&
           !element->reference.kind && !element->reference.node)
    *result = static_cast<IRawElementProviderFragmentRoot *>(this);
  else if (iid == __uuidof(IInvokeProvider))
    *result = static_cast<IInvokeProvider *>(this);
  else if (iid == __uuidof(IToggleProvider))
    *result = static_cast<IToggleProvider *>(this);
  else if (iid == __uuidof(IValueProvider))
    *result = static_cast<IValueProvider *>(this);
  else if (iid == __uuidof(IRangeValueProvider))
    *result = static_cast<IRangeValueProvider *>(this);
  else if (iid == __uuidof(ISelectionProvider))
    *result = static_cast<ISelectionProvider *>(this);
  else if (iid == __uuidof(ISelectionItemProvider))
    *result = static_cast<ISelectionItemProvider *>(this);
  else if (iid == __uuidof(IExpandCollapseProvider))
    *result = static_cast<IExpandCollapseProvider *>(this);
  else if (iid == __uuidof(IScrollProvider))
    *result = static_cast<IScrollProvider *>(this);
  else if (iid == __uuidof(IScrollItemProvider))
    *result = static_cast<IScrollItemProvider *>(this);
  else if (iid == __uuidof(IGridProvider))
    *result = static_cast<IGridProvider *>(this);
  else if (iid == __uuidof(IGridItemProvider))
    *result = static_cast<IGridItemProvider *>(this);
  else if (iid == __uuidof(ITableProvider))
    *result = static_cast<ITableProvider *>(this);
  else if (iid == __uuidof(ITableItemProvider))
    *result = static_cast<ITableItemProvider *>(this);
  else if (iid == __uuidof(IVirtualizedItemProvider))
    *result = static_cast<IVirtualizedItemProvider *>(this);
  else if (iid == __uuidof(IItemContainerProvider))
    *result = static_cast<IItemContainerProvider *>(this);
  else if (iid == __uuidof(IWindowProvider))
    *result = static_cast<IWindowProvider *>(this);
  else if (iid == IID_IMarshal && marshaler)
    return marshaler->QueryInterface(iid, result);
  else
    return E_NOINTERFACE;
  AddRef();
  return S_OK;
}
HRESULT Node::get_ProviderOptions(ProviderOptions *result) {
  if (!result)
    return E_POINTER;
  *result = static_cast<ProviderOptions>(ProviderOptions_ServerSideProvider |
                                         ProviderOptions_UseComThreading);
  return S_OK;
}
HRESULT Node::get_HostRawElementProvider(IRawElementProviderSimple **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  if (!tree || tree->closed)
    return UIA_E_ELEMENTNOTAVAILABLE;
  if (element->reference.node || element->reference.kind)
    return S_OK;
  return UiaHostProviderFromHwnd(
      std::static_pointer_cast<WindowHost>(tree->host)->hwnd, result);
}
HRESULT Node::GetRuntimeId(SAFEARRAY **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  if (!tree || tree->closed || element->retired)
    return UIA_E_ELEMENTNOTAVAILABLE;
  LONG values[] = {UiaAppendRuntimeId, static_cast<LONG>(tree->identity),
                   static_cast<LONG>(tree->identity >> 32),
                   static_cast<LONG>(element->serial),
                   static_cast<LONG>(element->serial >> 32)};
  auto array = SafeArrayCreateVector(VT_I4, 0, 5);
  if (!array)
    return E_OUTOFMEMORY;
  for (LONG index = 0; index < 5; ++index)
    SafeArrayPutElement(array, &index, &values[index]);
  *result = array;
  return S_OK;
}
ElementPtr fromProvider(const std::shared_ptr<Tree> &tree,
                        IRawElementProviderSimple *provider) {
  if (!provider || !tree)
    return {};
  IRawElementProviderFragment *fragment = nullptr;
  if (FAILED(provider->QueryInterface(__uuidof(IRawElementProviderFragment),
                                      reinterpret_cast<void **>(&fragment))))
    return {};
  SAFEARRAY *array = nullptr;
  HRESULT status = fragment->GetRuntimeId(&array);
  fragment->Release();
  if (FAILED(status) || !array)
    return {};
  LONG lower = 0, upper = -1;
  SafeArrayGetLBound(array, 1, &lower);
  SafeArrayGetUBound(array, 1, &upper);
  LONG parts[5] = {};
  bool valid = SafeArrayGetDim(array) == 1 && upper - lower + 1 == 5;
  VARTYPE type = VT_EMPTY;
  valid &= SUCCEEDED(SafeArrayGetVartype(array, &type)) && type == VT_I4;
  if (valid)
    for (LONG index = 0; index < 5; ++index) {
      LONG at = lower + index;
      valid &= SUCCEEDED(SafeArrayGetElement(array, &at, &parts[index]));
    }
  SafeArrayDestroy(array);
  const auto identity =
      static_cast<uint64_t>(static_cast<uint32_t>(parts[1])) |
      (static_cast<uint64_t>(static_cast<uint32_t>(parts[2])) << 32);
  const auto serial =
      static_cast<uint64_t>(static_cast<uint32_t>(parts[3])) |
      (static_cast<uint64_t>(static_cast<uint32_t>(parts[4])) << 32);
  return valid && parts[0] == UiaAppendRuntimeId && identity == tree->identity
             ? tree->elementBySerial(serial)
             : nullptr;
}
HRESULT Node::Navigate(NavigateDirection direction,
                       IRawElementProviderFragment **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return guarded([&]() -> HRESULT {
    if (!element->state())
      return UIA_E_ELEMENTNOTAVAILABLE;
    ElementPtr target;
    if (direction == NavigateDirection_Parent)
      target = element->parent();
    else if (direction == NavigateDirection_FirstChild)
      target = element->childAt(0);
    else if (direction == NavigateDirection_LastChild)
      target = element->childAt(element->childCount() - 1);
    else if (direction == NavigateDirection_NextSibling ||
             direction == NavigateDirection_PreviousSibling) {
      auto parent = element->parent();
      auto index = element->indexInParent();
      if (parent && index >= 0)
        target = parent->childAt(
            index + (direction == NavigateDirection_NextSibling ? 1 : -1));
    } else
      return E_INVALIDARG;
    if (target)
      *result = static_cast<IRawElementProviderFragment *>(new Node(target));
    return S_OK;
  });
}
HRESULT Node::get_BoundingRectangle(UiaRect *result) {
  if (!result)
    return E_POINTER;
  *result = {};
  return guarded([&]() -> HRESULT {
    auto value = element->state();
    if (!value)
      return UIA_E_ELEMENTNOTAVAILABLE;
    auto bounds = !element->reference.node && !element->reference.kind
                      ? tree->host->bounds()
                      : Rect{value->data.x, value->data.y, value->data.width,
                             value->data.height};
    if ((element->reference.node || element->reference.kind) &&
        !value->data.geometry)
      return S_OK;
    auto screen = tree->host->toScreen(bounds);
    if (!screen)
      return UIA_E_ELEMENTNOTAVAILABLE;
    *result = {screen->x, screen->y, screen->width, screen->height};
    return S_OK;
  });
}
HRESULT Node::GetEmbeddedFragmentRoots(SAFEARRAY **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return S_OK;
}
HRESULT Node::SetFocus() {
  return guarded([&] { return actionResult(element->perform(3)); });
}
HRESULT Node::get_FragmentRoot(IRawElementProviderFragmentRoot **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return guarded([&]() -> HRESULT {
    auto root = tree ? tree->actual(0) : nullptr;
    if (!root)
      return UIA_E_ELEMENTNOTAVAILABLE;
    *result = static_cast<IRawElementProviderFragmentRoot *>(new Node(root));
    return S_OK;
  });
}
HRESULT Node::ElementProviderFromPoint(double x, double y,
                                       IRawElementProviderFragment **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return guarded([&]() -> HRESULT {
    if (!tree || tree->closed)
      return UIA_E_ELEMENTNOTAVAILABLE;
    if (!std::isfinite(x) || !std::isfinite(y))
      return E_INVALIDARG;
    auto point = tree->host->fromScreen({x, y});
    if (!point)
      return UIA_E_ELEMENTNOTAVAILABLE;
    auto bounds = tree->host->bounds();
    if (point->x < 0 || point->y < 0 || point->x >= bounds.width ||
        point->y >= bounds.height)
      return S_OK;
    auto hit = tree->hit(*point);
    if (hit)
      *result = static_cast<IRawElementProviderFragment *>(new Node(hit));
    return S_OK;
  });
}
HRESULT Node::GetFocus(IRawElementProviderFragment **result) {
  if (!result)
    return E_POINTER;
  *result = nullptr;
  return guarded([&]() -> HRESULT {
    auto current = tree ? tree->snapshot() : nullptr;
    if (!current)
      return UIA_E_ELEMENTNOTAVAILABLE;
    if (!tree->host->focused() || !current->focused)
      return S_OK;
    auto target = tree->actual(current->focused);
    if (target && target->exposed())
      *result = static_cast<IRawElementProviderFragment *>(new Node(target));
    return S_OK;
  });
}
} // namespace swax::win
