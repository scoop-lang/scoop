#include "../../../native/windows/accessibility/provider.h"
#include <cmath>

namespace {
using namespace swax;
struct Failure {
  int line;
};
#define CHECK(value)                                                           \
  do {                                                                         \
    if (!(value))                                                              \
      throw Failure{__LINE__};                                                 \
  } while (false)
struct Apartment {
  Apartment() {
    CHECK(SUCCEEDED(CoInitializeEx(nullptr, COINIT_MULTITHREADED)));
  }
  ~Apartment() { CoUninitialize(); }
};
struct Release {
  template <class T> void operator()(T *value) const { value->Release(); }
};
template <class T> using Com = std::unique_ptr<T, Release>;
class TestHost final : public Host {
public:
  bool open = true;
  bool onOwner() const override { return true; }
  int32_t dispatch(const std::function<int32_t()> &body) override {
    return body();
  }
  bool visible() const override { return open; }
  bool enabled() const override { return open; }
  bool focused() const override { return false; }
  Rect bounds() const override { return {0, 0, 640, 480}; }
  std::optional<Rect> toScreen(Rect value) const override { return value; }
  std::optional<Point> fromScreen(Point value) const override { return value; }
  void changed(const Change &) override {}
  void announce(int64_t, const std::string &, int32_t) override {}
  void close() override { open = false; }
};
struct Actions {
  int count = 0, operation = -1;
  int64_t node = 0, start = -1, end = -1;
  uint64_t revision = 0;
  double position = 0;
  static int32_t record(void *context, int64_t node, int32_t operation,
                        const uint8_t *, int64_t, int64_t start, int64_t end,
                        uint64_t revision, double position) {
    auto &value = *static_cast<Actions *>(context);
    ++value.count;
    value.node = node;
    value.operation = operation;
    value.start = start;
    value.end = end;
    value.revision = revision;
    value.position = position;
    return 1;
  }
};
std::shared_ptr<State> node(int64_t id, int role, int64_t parent = 0) {
  auto value = std::make_shared<State>();
  value->data.number = id;
  value->data.parent = parent;
  value->data.role = role;
  value->data.flags = SWAXEnabled;
  value->data.checked = value->data.selected = value->data.expanded = -1;
  value->data.anchor = value->data.cursor = -1;
  value->data.table_rows = value->data.table_columns = -1;
  value->data.cell_row = value->data.cell_column = -1;
  return value;
}
void publish(const std::shared_ptr<Tree> &tree,
             std::initializer_list<std::shared_ptr<State>> values) {
  tree->publishing = true;
  for (const auto &value : values)
    tree->pending[value->data.number] = value;
  tree->commit(0);
}
template <class T>
Com<T> pattern(IRawElementProviderSimple *provider, PATTERNID id) {
  IUnknown *unknown = nullptr;
  CHECK(provider->GetPatternProvider(id, &unknown) == S_OK && unknown);
  Com<IUnknown> owner(unknown);
  T *result = nullptr;
  CHECK(unknown->QueryInterface(__uuidof(T),
                                reinterpret_cast<void **>(&result)) == S_OK);
  return Com<T>(result);
}
void textEquals(ITextRangeProvider *range, int maximum,
                const std::wstring &expected) {
  BSTR value = nullptr;
  CHECK(range->GetText(maximum, &value) == S_OK);
  const std::wstring actual(value, SysStringLen(value));
  SysFreeString(value);
  CHECK(actual == expected);
}
int64_t prepend(void *, int64_t offset, int32_t) { return offset + 2; }
} // namespace

extern "C" int32_t scoop_ax_test_windows_properties() {
  try {
    Apartment apartment;
    Actions actions;
    auto tree = std::make_shared<Tree>(std::make_shared<TestHost>(),
                                       Actions::record, &actions);
    auto heading = node(1, 14);
    heading->data.heading_level = 3;
    auto button = node(2, 2);
    button->data.actions = 1;
    publish(tree, {heading, button});
    Com<IRawElementProviderSimple> title(swax::win::simple(tree->actual(1)));
    Com<IRawElementProviderSimple> control(swax::win::simple(tree->actual(2)));
    VARIANT value;
    CHECK(title->GetPropertyValue(UIA_HeadingLevelPropertyId, &value) == S_OK);
    CHECK(value.vt == VT_I4 && value.lVal == 80053);
    VariantClear(&value);
    CHECK(title->GetPropertyValue(UIA_LevelPropertyId, &value) == S_OK);
    CHECK(value.vt == VT_I4 && value.lVal == 3);
    VariantClear(&value);
    auto invoke = pattern<IInvokeProvider>(control.get(), UIA_InvokePatternId);
    CHECK(invoke->Invoke() == S_OK && actions.count == 1 && actions.node == 2 &&
          actions.operation == 0);
    auto disabled = std::make_shared<State>(*button);
    disabled->data.flags &= ~SWAXEnabled;
    publish(tree, {disabled});
    CHECK(FAILED(invoke->Invoke()) && actions.count == 1);
    tree->close();
    CHECK(control->GetPropertyValue(UIA_ControlTypePropertyId, &value) ==
          static_cast<HRESULT>(UIA_E_ELEMENTNOTAVAILABLE));
    CHECK(invoke->Invoke() == static_cast<HRESULT>(UIA_E_ELEMENTNOTAVAILABLE) &&
          actions.count == 1);
    return 0;
  } catch (Failure failure) {
    return failure.line;
  } catch (...) {
    return -1;
  }
}

extern "C" int32_t scoop_ax_test_windows_scroll() {
  try {
    Apartment apartment;
    Actions actions;
    auto tree = std::make_shared<Tree>(std::make_shared<TestHost>(),
                                       Actions::record, &actions);
    auto viewport = node(1, 5);
    auto bar = node(2, 6, 1);
    bar->data.scroll_orientation = 2;
    bar->data.scroll_content = 300;
    bar->data.scroll_viewport = 100;
    bar->data.scroll_offset = 100;
    bar->data.actions = 128;
    publish(tree, {viewport, bar});
    Com<IRawElementProviderSimple> provider(swax::win::simple(tree->actual(1)));
    auto scroll = pattern<IScrollProvider>(provider.get(), UIA_ScrollPatternId);
    double value;
    BOOL scrollable;
    CHECK(scroll->get_HorizontalScrollPercent(&value) == S_OK && value == -1);
    CHECK(scroll->get_HorizontallyScrollable(&scrollable) == S_OK &&
          !scrollable);
    CHECK(scroll->get_VerticalScrollPercent(&value) == S_OK && value == 50);
    CHECK(scroll->get_VerticallyScrollable(&scrollable) == S_OK && scrollable);
    CHECK(scroll->SetScrollPercent(-1, 25) == S_OK);
    CHECK(actions.count == 1 && actions.node == 2 && actions.operation == 8 &&
          actions.position == 0.25);
    CHECK(scroll->SetScrollPercent(0, -1) ==
          static_cast<HRESULT>(UIA_E_INVALIDOPERATION));
    CHECK(scroll->SetScrollPercent(-1, 101) == E_INVALIDARG);
    CHECK(actions.count == 1);
    tree->close();
    CHECK(scroll->get_VerticalScrollPercent(&value) ==
          static_cast<HRESULT>(UIA_E_ELEMENTNOTAVAILABLE));
    return 0;
  } catch (Failure failure) {
    return failure.line;
  } catch (...) {
    return -1;
  }
}

extern "C" int32_t scoop_ax_test_windows_text() {
  try {
    Apartment apartment;
    Actions actions;
    auto tree = std::make_shared<Tree>(std::make_shared<TestHost>(),
                                       Actions::record, &actions);
    auto created = scoop_window_text_document_create(6);
    CHECK(created.error == 0 && created.pointer);
    auto document = std::make_shared<Document>(
        static_cast<SWTextDocument *>(created.pointer));
    sw_text_document_release(document->pointer);
    auto edit = node(1, 4);
    edit->text[3] = "A\xF0\x9F\x98\x80"
                    "B";
    edit->data.actions = 16;
    edit->data.content_revision = 7;
    edit->document = document;
    edit->documentRevision = sw_text_document_state(document->pointer).revision;
    edit->documentEpoch = 1;
    publish(tree, {edit});
    Com<IRawElementProviderSimple> provider(swax::win::simple(tree->actual(1)));
    auto text = pattern<ITextProvider2>(provider.get(), UIA_TextPattern2Id);
    ITextRangeProvider *full = nullptr;
    CHECK(text->get_DocumentRange(&full) == S_OK && full);
    Com<ITextRangeProvider> fullRange(full);
    textEquals(fullRange.get(), -1, L"A\U0001F600B");
    Com<ITextRangeProvider> selection(
        swax::win::textRange(tree->actual(1), 1, 5));
    CHECK(selection);
    textEquals(selection.get(), 1, L"");
    textEquals(selection.get(), 2, L"\U0001F600");
    CHECK(selection->Select() == S_OK && actions.count == 1);
    CHECK(actions.operation == 5 && actions.start == 1 && actions.end == 5 &&
          actions.revision == 7);
    CHECK(scoop_window_text_document_map(document->pointer, 8, prepend,
                                         nullptr) == 0);
    auto edited = std::make_shared<State>(*edit);
    edited->text[3] = "hiA\xF0\x9F\x98\x80"
                      "B";
    edited->data.content_revision = 8;
    edited->documentRevision =
        sw_text_document_state(document->pointer).revision;
    publish(tree, {edited});
    textEquals(selection.get(), -1, L"\U0001F600");
    CHECK(selection->Select() == S_OK && actions.start == 3 &&
          actions.end == 7 && actions.revision == 8);
    CHECK(scoop_window_text_document_reset(document->pointer, 8) == 0);
    auto reset = std::make_shared<State>(*edited);
    reset->documentRevision =
        sw_text_document_state(document->pointer).revision;
    reset->documentEpoch = 2;
    publish(tree, {reset});
    BSTR value = nullptr;
    CHECK(selection->GetText(-1, &value) ==
              static_cast<HRESULT>(UIA_E_ELEMENTNOTAVAILABLE) &&
          !value);
    Com<ITextRangeProvider> fresh(swax::win::textRange(tree->actual(1), 3, 7));
    CHECK(fresh);
    auto protectedEdit = std::make_shared<State>(*reset);
    protectedEdit->data.flags |= SWAXProtected;
    publish(tree, {protectedEdit});
    CHECK(fresh->GetText(-1, &value) ==
              static_cast<HRESULT>(UIA_E_NOTSUPPORTED) &&
          !value);
    IUnknown *hidden = nullptr;
    CHECK(provider->GetPatternProvider(UIA_TextPatternId, &hidden) == S_OK &&
          !hidden);
    tree->close();
    CHECK(fresh->GetText(-1, &value) ==
              static_cast<HRESULT>(UIA_E_ELEMENTNOTAVAILABLE) &&
          !value);
    scoop_window_text_document_close(document->pointer);
    return 0;
  } catch (Failure failure) {
    return failure.line;
  } catch (...) {
    return -1;
  }
}
