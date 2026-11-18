#ifndef SCOOP_WINDOW_WINDOWS_ACCESSIBILITY_TEXT_H
#define SCOOP_WINDOW_WINDOWS_ACCESSIBILITY_TEXT_H
#include "provider.h"

namespace swax::win {
struct TextView {
  std::shared_ptr<const State> state;
  int64_t start = 0, end = 0;
  int32_t first = 1, last = 0;
};
struct RangeIdentity {
  uint64_t tree, element, revision, epoch;
  SWTextDocument *document;
  int64_t start, end;
};
// Internal COM identity is used only by the standard range comparison
// operations.
inline constexpr GUID rangeIdentityId = {
    0x9f18b2ee,
    0xe532,
    0x418d,
    {0x9a, 0xc9, 0xb9, 0x79, 0x51, 0x77, 0xec, 0x06}};
struct RangeAccess : IUnknown {
  virtual HRESULT STDMETHODCALLTYPE ReadIdentity(RangeIdentity *result) = 0;
};

class TextProvider final : public ITextProvider2 {
public:
  explicit TextProvider(ElementPtr value);
  ~TextProvider();
  ElementPtr element;
  std::shared_ptr<Tree> tree;
  std::atomic<ULONG> references{1};
  IUnknown *marshaler = nullptr;
  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void **result) override;
  ULONG STDMETHODCALLTYPE AddRef() override;
  ULONG STDMETHODCALLTYPE Release() override;
  HRESULT STDMETHODCALLTYPE GetSelection(SAFEARRAY **result) override;
  HRESULT STDMETHODCALLTYPE GetVisibleRanges(SAFEARRAY **result) override;
  HRESULT STDMETHODCALLTYPE RangeFromChild(
      IRawElementProviderSimple *child, ITextRangeProvider **result) override;
  HRESULT STDMETHODCALLTYPE
  RangeFromPoint(UiaPoint point, ITextRangeProvider **result) override;
  HRESULT STDMETHODCALLTYPE
  get_DocumentRange(ITextRangeProvider **result) override;
  HRESULT STDMETHODCALLTYPE
  get_SupportedTextSelection(SupportedTextSelection *result) override;
  HRESULT STDMETHODCALLTYPE
  RangeFromAnnotation(IRawElementProviderSimple *annotation,
                      ITextRangeProvider **result) override;
  HRESULT STDMETHODCALLTYPE GetCaretRange(BOOL *active,
                                          ITextRangeProvider **result) override;
};

class TextRange final : public ITextRangeProvider2, public RangeAccess {
public:
  TextRange(ElementPtr value, const TextView &view);
  ~TextRange();
  ElementPtr element;
  std::shared_ptr<Tree> tree;
  std::atomic<ULONG> references{1};
  IUnknown *marshaler = nullptr;
  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void **result) override;
  ULONG STDMETHODCALLTYPE AddRef() override;
  ULONG STDMETHODCALLTYPE Release() override;
  HRESULT STDMETHODCALLTYPE ReadIdentity(RangeIdentity *result) override;
  HRESULT STDMETHODCALLTYPE Clone(ITextRangeProvider **result) override;
  HRESULT STDMETHODCALLTYPE Compare(ITextRangeProvider *other,
                                    BOOL *result) override;
  HRESULT STDMETHODCALLTYPE CompareEndpoints(
      TextPatternRangeEndpoint endpoint, ITextRangeProvider *other,
      TextPatternRangeEndpoint otherEndpoint, int *result) override;
  HRESULT STDMETHODCALLTYPE ExpandToEnclosingUnit(TextUnit unit) override;
  HRESULT STDMETHODCALLTYPE FindAttribute(TEXTATTRIBUTEID attribute,
                                          VARIANT value, BOOL backward,
                                          ITextRangeProvider **result) override;
  HRESULT STDMETHODCALLTYPE FindText(BSTR text, BOOL backward, BOOL ignoreCase,
                                     ITextRangeProvider **result) override;
  HRESULT STDMETHODCALLTYPE GetAttributeValue(TEXTATTRIBUTEID attribute,
                                              VARIANT *result) override;
  HRESULT STDMETHODCALLTYPE GetBoundingRectangles(SAFEARRAY **result) override;
  HRESULT STDMETHODCALLTYPE
  GetEnclosingElement(IRawElementProviderSimple **result) override;
  HRESULT STDMETHODCALLTYPE GetText(int maximum, BSTR *result) override;
  HRESULT STDMETHODCALLTYPE Move(TextUnit unit, int count,
                                 int *result) override;
  HRESULT STDMETHODCALLTYPE
  MoveEndpointByUnit(TextPatternRangeEndpoint endpoint, TextUnit unit,
                     int count, int *result) override;
  HRESULT STDMETHODCALLTYPE MoveEndpointByRange(
      TextPatternRangeEndpoint endpoint, ITextRangeProvider *other,
      TextPatternRangeEndpoint otherEndpoint) override;
  HRESULT STDMETHODCALLTYPE Select() override;
  HRESULT STDMETHODCALLTYPE AddToSelection() override;
  HRESULT STDMETHODCALLTYPE RemoveFromSelection() override;
  HRESULT STDMETHODCALLTYPE ScrollIntoView(BOOL alignToTop) override;
  HRESULT STDMETHODCALLTYPE GetChildren(SAFEARRAY **result) override;
  HRESULT STDMETHODCALLTYPE ShowContextMenu() override;
  HRESULT read(TextView &view);
  HRESULT save(const TextView &view);

private:
  std::mutex mutex;
  std::shared_ptr<Document> document;
  SWTextRange *anchor = nullptr;
  uint64_t epoch, textRevision;
  int64_t start, end;
  int32_t first, last;
};

HRESULT readText(const ElementPtr &element,
                 std::shared_ptr<const State> &result);
HRESULT textGeometryResult(int32_t status);
HRESULT enclosingUnit(const TextView &view, TextUnit unit, int64_t offset,
                      int32_t affinity, SWAccessibilityTextRange &result);
HRESULT rangeIdentity(ITextRangeProvider *range, RangeIdentity &result);
bool sameDocument(const RangeIdentity &a, const RangeIdentity &b);
SAFEARRAY *rangeArray(ITextRangeProvider *range);
} // namespace swax::win
#endif
