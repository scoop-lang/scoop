#ifndef SCOOP_WINDOW_WINDOWS_ACCESSIBILITY_H
#define SCOOP_WINDOW_WINDOWS_ACCESSIBILITY_H

#include "../../accessibility/core.h"
#include "../window.h"
#include <UIAutomation.h>
#include <atomic>
#include <limits>
#include <utility>

namespace swax::win {

struct ComFailure {
  HRESULT value;
};

template <typename Body> HRESULT guarded(Body body) noexcept {
  try {
    return body();
  } catch (const std::bad_alloc &) {
    return E_OUTOFMEMORY;
  } catch (const ComFailure &error) {
    return error.value;
  } catch (...) {
    return E_FAIL;
  }
}
CONTROLTYPEID controlType(const State &value);
std::string helpText(const State &value);
std::string ariaProperties(const State &value);
inline constexpr std::pair<PROPERTYID, PATTERNID> patternAvailability[] = {
    {UIA_IsInvokePatternAvailablePropertyId, UIA_InvokePatternId},
    {UIA_IsTogglePatternAvailablePropertyId, UIA_TogglePatternId},
    {UIA_IsValuePatternAvailablePropertyId, UIA_ValuePatternId},
    {UIA_IsRangeValuePatternAvailablePropertyId, UIA_RangeValuePatternId},
    {UIA_IsSelectionPatternAvailablePropertyId, UIA_SelectionPatternId},
    {UIA_IsSelectionItemPatternAvailablePropertyId, UIA_SelectionItemPatternId},
    {UIA_IsExpandCollapsePatternAvailablePropertyId,
     UIA_ExpandCollapsePatternId},
    {UIA_IsScrollPatternAvailablePropertyId, UIA_ScrollPatternId},
    {UIA_IsScrollItemPatternAvailablePropertyId, UIA_ScrollItemPatternId},
    {UIA_IsGridPatternAvailablePropertyId, UIA_GridPatternId},
    {UIA_IsGridItemPatternAvailablePropertyId, UIA_GridItemPatternId},
    {UIA_IsTablePatternAvailablePropertyId, UIA_TablePatternId},
    {UIA_IsTableItemPatternAvailablePropertyId, UIA_TableItemPatternId},
    {UIA_IsItemContainerPatternAvailablePropertyId, UIA_ItemContainerPatternId},
    {UIA_IsVirtualizedItemPatternAvailablePropertyId,
     UIA_VirtualizedItemPatternId},
    {UIA_IsTextPatternAvailablePropertyId, UIA_TextPatternId},
    {UIA_IsTextPattern2AvailablePropertyId, UIA_TextPattern2Id},
    {UIA_IsWindowPatternAvailablePropertyId, UIA_WindowPatternId}};
HRESULT actionResult(int32_t value);
std::wstring wide(const std::string &text);
std::optional<std::string> utf8(const wchar_t *text, int length = -1);
SAFEARRAY *elementArray(const std::vector<ElementPtr> &values);
SAFEARRAY *numberArray(const std::vector<double> &values);
IRawElementProviderSimple *simple(const ElementPtr &element);
ElementPtr fromProvider(const std::shared_ptr<Tree> &tree,
                        IRawElementProviderSimple *provider);
IUnknown *textProvider(const ElementPtr &element);
ITextRangeProvider *textRange(const ElementPtr &element, int64_t start,
                              int64_t end, int32_t startAffinity = 1,
                              int32_t endAffinity = 0);

class WindowHost : public Host {
public:
  explicit WindowHost(SWWindow *value)
      : window(value), hwnd(value->hwnd), thread(GetCurrentThreadId()) {}
  SWWindow *window;
  HWND hwnd;
  DWORD thread;
  std::weak_ptr<Tree> tree;
  IRawElementProviderSimple *root = nullptr;
  bool onOwner() const override;
  int32_t dispatch(const std::function<int32_t()> &body) override;
  bool visible() const override;
  bool enabled() const override;
  bool focused() const override;
  Rect bounds() const override;
  std::optional<Rect> toScreen(Rect value) const override;
  std::optional<Point> fromScreen(Point value) const override;
  void changed(const Change &change) override;
  void nativeChanged(UINT message);
  void announce(int64_t node, const std::string &text,
                int32_t priority) override;
  void close() override;
};

class Node final : public IRawElementProviderSimple,
                   public IRawElementProviderFragment,
                   public IRawElementProviderFragmentRoot,
                   public IInvokeProvider,
                   public IToggleProvider,
                   public IValueProvider,
                   public IRangeValueProvider,
                   public ISelectionProvider,
                   public ISelectionItemProvider,
                   public IExpandCollapseProvider,
                   public IScrollProvider,
                   public IScrollItemProvider,
                   public IGridProvider,
                   public IGridItemProvider,
                   public ITableProvider,
                   public ITableItemProvider,
                   public IVirtualizedItemProvider,
                   public IItemContainerProvider,
                   public IWindowProvider {
public:
  explicit Node(ElementPtr value);
  ~Node();
  ElementPtr element;
  std::shared_ptr<Tree> tree;
  std::atomic<ULONG> references{1};
  IUnknown *marshaler = nullptr;
  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void **result) override;
  ULONG STDMETHODCALLTYPE AddRef() override;
  ULONG STDMETHODCALLTYPE Release() override;
  HRESULT STDMETHODCALLTYPE
  get_ProviderOptions(ProviderOptions *result) override;
  HRESULT STDMETHODCALLTYPE GetPatternProvider(PATTERNID pattern,
                                               IUnknown **result) override;
  HRESULT STDMETHODCALLTYPE GetPropertyValue(PROPERTYID property,
                                             VARIANT *result) override;
  HRESULT STDMETHODCALLTYPE
  get_HostRawElementProvider(IRawElementProviderSimple **result) override;
  HRESULT STDMETHODCALLTYPE
  Navigate(NavigateDirection direction,
           IRawElementProviderFragment **result) override;
  HRESULT STDMETHODCALLTYPE GetRuntimeId(SAFEARRAY **result) override;
  HRESULT STDMETHODCALLTYPE get_BoundingRectangle(UiaRect *result) override;
  HRESULT STDMETHODCALLTYPE
  GetEmbeddedFragmentRoots(SAFEARRAY **result) override;
  HRESULT STDMETHODCALLTYPE SetFocus() override;
  HRESULT STDMETHODCALLTYPE
  get_FragmentRoot(IRawElementProviderFragmentRoot **result) override;
  HRESULT STDMETHODCALLTYPE ElementProviderFromPoint(
      double x, double y, IRawElementProviderFragment **result) override;
  HRESULT STDMETHODCALLTYPE
  GetFocus(IRawElementProviderFragment **result) override;
  HRESULT STDMETHODCALLTYPE Invoke() override;
  HRESULT STDMETHODCALLTYPE Toggle() override;
  HRESULT STDMETHODCALLTYPE get_ToggleState(ToggleState *result) override;
  HRESULT STDMETHODCALLTYPE SetValue(LPCWSTR value) override;
  HRESULT STDMETHODCALLTYPE get_Value(BSTR *result) override;
  HRESULT STDMETHODCALLTYPE get_IsReadOnly(BOOL *result) override;
  HRESULT STDMETHODCALLTYPE SetValue(double value) override;
  HRESULT STDMETHODCALLTYPE get_Value(double *result) override;
  HRESULT STDMETHODCALLTYPE get_Maximum(double *result) override;
  HRESULT STDMETHODCALLTYPE get_Minimum(double *result) override;
  HRESULT STDMETHODCALLTYPE get_LargeChange(double *result) override;
  HRESULT STDMETHODCALLTYPE get_SmallChange(double *result) override;
  HRESULT STDMETHODCALLTYPE GetSelection(SAFEARRAY **result) override;
  HRESULT STDMETHODCALLTYPE get_CanSelectMultiple(BOOL *result) override;
  HRESULT STDMETHODCALLTYPE get_IsSelectionRequired(BOOL *result) override;
  HRESULT STDMETHODCALLTYPE Select() override;
  HRESULT STDMETHODCALLTYPE AddToSelection() override;
  HRESULT STDMETHODCALLTYPE RemoveFromSelection() override;
  HRESULT STDMETHODCALLTYPE get_IsSelected(BOOL *result) override;
  HRESULT STDMETHODCALLTYPE
  get_SelectionContainer(IRawElementProviderSimple **result) override;
  HRESULT STDMETHODCALLTYPE Expand() override;
  HRESULT STDMETHODCALLTYPE Collapse() override;
  HRESULT STDMETHODCALLTYPE
  get_ExpandCollapseState(ExpandCollapseState *result) override;
  HRESULT STDMETHODCALLTYPE Scroll(ScrollAmount horizontal,
                                   ScrollAmount vertical) override;
  HRESULT STDMETHODCALLTYPE SetScrollPercent(double horizontal,
                                             double vertical) override;
  HRESULT STDMETHODCALLTYPE
  get_HorizontalScrollPercent(double *result) override;
  HRESULT STDMETHODCALLTYPE get_VerticalScrollPercent(double *result) override;
  HRESULT STDMETHODCALLTYPE get_HorizontalViewSize(double *result) override;
  HRESULT STDMETHODCALLTYPE get_VerticalViewSize(double *result) override;
  HRESULT STDMETHODCALLTYPE get_HorizontallyScrollable(BOOL *result) override;
  HRESULT STDMETHODCALLTYPE get_VerticallyScrollable(BOOL *result) override;
  HRESULT STDMETHODCALLTYPE ScrollIntoView() override;
  HRESULT STDMETHODCALLTYPE
  GetItem(int row, int column, IRawElementProviderSimple **result) override;
  HRESULT STDMETHODCALLTYPE get_RowCount(int *result) override;
  HRESULT STDMETHODCALLTYPE get_ColumnCount(int *result) override;
  HRESULT STDMETHODCALLTYPE get_Row(int *result) override;
  HRESULT STDMETHODCALLTYPE get_Column(int *result) override;
  HRESULT STDMETHODCALLTYPE get_RowSpan(int *result) override;
  HRESULT STDMETHODCALLTYPE get_ColumnSpan(int *result) override;
  HRESULT STDMETHODCALLTYPE
  get_ContainingGrid(IRawElementProviderSimple **result) override;
  HRESULT STDMETHODCALLTYPE GetRowHeaders(SAFEARRAY **result) override;
  HRESULT STDMETHODCALLTYPE GetColumnHeaders(SAFEARRAY **result) override;
  HRESULT STDMETHODCALLTYPE
  get_RowOrColumnMajor(RowOrColumnMajor *result) override;
  HRESULT STDMETHODCALLTYPE GetRowHeaderItems(SAFEARRAY **result) override;
  HRESULT STDMETHODCALLTYPE GetColumnHeaderItems(SAFEARRAY **result) override;
  HRESULT STDMETHODCALLTYPE Realize() override;
  HRESULT STDMETHODCALLTYPE FindItemByProperty(
      IRawElementProviderSimple *after, PROPERTYID property, VARIANT value,
      IRawElementProviderSimple **result) override;
  HRESULT STDMETHODCALLTYPE SetVisualState(WindowVisualState state) override;
  HRESULT STDMETHODCALLTYPE Close() override;
  HRESULT STDMETHODCALLTYPE WaitForInputIdle(int milliseconds,
                                             BOOL *result) override;
  HRESULT STDMETHODCALLTYPE get_CanMaximize(BOOL *result) override;
  HRESULT STDMETHODCALLTYPE get_CanMinimize(BOOL *result) override;
  HRESULT STDMETHODCALLTYPE get_IsModal(BOOL *result) override;
  HRESULT STDMETHODCALLTYPE
  get_WindowVisualState(WindowVisualState *result) override;
  HRESULT STDMETHODCALLTYPE
  get_WindowInteractionState(WindowInteractionState *result) override;
  HRESULT STDMETHODCALLTYPE get_IsTopmost(BOOL *result) override;
  bool supports(PATTERNID pattern, const State &value);
  ElementPtr scrollBar(int orientation);
  HRESULT select(bool value, bool exclusive);
};
} // namespace swax::win
#endif
