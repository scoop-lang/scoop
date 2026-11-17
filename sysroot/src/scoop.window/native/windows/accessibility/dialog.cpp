#include "provider.h"

namespace swax::win {
HRESULT Node::SetVisualState(WindowVisualState value) {
  return guarded([&]() -> HRESULT {
    auto state = element->state();
    if (!state)
      return UIA_E_ELEMENTNOTAVAILABLE;
    if (state->data.role != 21)
      return UIA_E_NOTSUPPORTED;
    return value == WindowVisualState_Normal ? S_OK : UIA_E_INVALIDOPERATION;
  });
}
HRESULT Node::Close() {
  return guarded([&] { return actionResult(element->perform(19)); });
}
HRESULT Node::WaitForInputIdle(int milliseconds, BOOL *result) {
  if (!result)
    return E_POINTER;
  *result = FALSE;
  if (milliseconds < 0)
    return E_INVALIDARG;
  return guarded([&]() -> HRESULT {
    auto state = element->state();
    if (!state || !tree)
      return UIA_E_ELEMENTNOTAVAILABLE;
    if (tree->host->onOwner()) {
      *result = !(state->data.flags & SWAXBusy);
      return S_OK;
    }
    DWORD waited = ::WaitForInputIdle(GetCurrentProcess(),
                                      static_cast<DWORD>(milliseconds));
    if (waited == WAIT_FAILED)
      return HRESULT_FROM_WIN32(GetLastError());
    *result = waited == 0;
    return S_OK;
  });
}
HRESULT Node::get_CanMaximize(BOOL *result) {
  if (!result)
    return E_POINTER;
  *result = FALSE;
  return tree && !tree->closed ? S_OK : UIA_E_ELEMENTNOTAVAILABLE;
}
HRESULT Node::get_CanMinimize(BOOL *result) { return get_CanMaximize(result); }
HRESULT Node::get_IsTopmost(BOOL *result) { return get_CanMaximize(result); }
HRESULT Node::get_IsModal(BOOL *result) {
  if (!result)
    return E_POINTER;
  *result = FALSE;
  return guarded([&]() -> HRESULT {
    auto state = element->state();
    if (!state)
      return UIA_E_ELEMENTNOTAVAILABLE;
    *result = (state->data.flags & SWAXModal) != 0;
    return S_OK;
  });
}
HRESULT Node::get_WindowVisualState(WindowVisualState *result) {
  if (!result)
    return E_POINTER;
  *result = WindowVisualState_Normal;
  return tree && !tree->closed ? S_OK : UIA_E_ELEMENTNOTAVAILABLE;
}
HRESULT Node::get_WindowInteractionState(WindowInteractionState *result) {
  if (!result)
    return E_POINTER;
  *result = WindowInteractionState_Closing;
  return guarded([&]() -> HRESULT {
    auto state = element->state();
    if (!state)
      return UIA_E_ELEMENTNOTAVAILABLE;
    *result = !element->exposed(true)
                  ? WindowInteractionState_BlockedByModalWindow
              : state->data.flags & SWAXBusy
                  ? WindowInteractionState_Running
                  : WindowInteractionState_ReadyForUserInteraction;
    return S_OK;
  });
}
} // namespace swax::win
