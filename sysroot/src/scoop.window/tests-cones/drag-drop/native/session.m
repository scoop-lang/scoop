#import "probe.h"

static int32_t operate(SWDropProbe *probe, int32_t step) {
  NSView *view = probe.view;
  if (step == 10) {
    CHECK(![view prepareForDragOperation:probe.info]);
    CHECK(![view performDragOperation:probe.info]);
    CHECK(probe.data.reads == 0);
    [view draggingEnded:probe.info];
  }
  if (step == 12) {
    CHECK([view draggingEntered:probe.info] == NSDragOperationNone);
    CHECK(![view prepareForDragOperation:probe.info]);
    CHECK(![view performDragOperation:probe.info]);
    [view draggingExited:nil];
    CHECK(probe.data.reads == 0);
    return 0;
  }
  int32_t kind = (step <= 8) ? step : 0;
  NSDragOperation mask = step == 1 ? NSDragOperationCopy | NSDragOperationMove
                         : (step == 2 || step == 7)
                             ? NSDragOperationCopy | NSDragOperationLink
                             : NSDragOperationCopy;
  CHECK(sw_drop_test_prepare(probe, kind, mask));
  SWDropInfo *info = probe.info;
  if (step == 21) {
    CHECK(probe.oldView.registeredDraggedTypes.count == 0);
    CHECK([probe.oldView draggingEntered:info] == NSDragOperationNone);
    CHECK(![probe.oldView prepareForDragOperation:info]);
    CHECK(![probe.oldView performDragOperation:info]);
    [probe.oldView draggingExited:nil];
  }
  CHECK([view wantsPeriodicDraggingUpdates] == (step == 18));
  CHECK(probe.data.reads == 0);
  NSDragOperation result = [view draggingEntered:info];
  BOOL rejected = step == 4 || step == 5 || step == 6 || step == 13 ||
                  step == 14 || step == 23;
  if (rejected) {
    CHECK(result == NSDragOperationNone);
    CHECK(![view prepareForDragOperation:info]);
    CHECK(![view performDragOperation:info]);
    [view draggingEnded:info];
    CHECK(probe.data.reads == 0);
    return 0;
  }
  NSDragOperation expected = step == 1                  ? NSDragOperationMove
                             : (step == 2 || step == 7) ? NSDragOperationLink
                                                        : NSDragOperationCopy;
  CHECK(result == expected);
  CHECK(probe.data.reads == 0);
  if (step == 9 || step == 11)
    return 0;
  if (step == 19) {
    [view draggingExited:nil];
    [view concludeDragOperation:nil];
    [view draggingEnded:info];
    CHECK(![view performDragOperation:info]);
    CHECK(probe.data.reads == 0);
    return 0;
  }
  CHECK([view draggingUpdated:info] == expected);
  CHECK([view prepareForDragOperation:info]);
  CHECK(probe.data.reads == 0);
  if (step == 17)
    probe.data.mutation = 1;
  if (step == 22)
    probe.data.mutation = 2;
  BOOL accepted =
      step != 1 && step != 7 && step != 8 && step != 17 && step != 22;
  CHECK([view performDragOperation:info] == accepted);
  CHECK(![view performDragOperation:info]);
  [view concludeDragOperation:nil];
  [view draggingEnded:info];
  [view draggingExited:nil];
  CHECK(probe.data.reads == ((step == 3 || step == 8) ? 0 : 1));
  return 0;
}

void scoop_drop_test_post(void *pointer, int32_t step,
                          SWDropTestCallback callback, void *context) {
  SWDropProbe *probe = (__bridge SWDropProbe *)pointer;
  probe.callback = callback;
  probe.context = context;
  dispatch_async(dispatch_get_main_queue(), ^{
    if (probe.closed)
      return;
    int32_t result = operate(probe, step);
    probe.lastStep = step;
    probe.lastResult = result;
    if (!probe.closed && probe.callback)
      probe.callback(probe.context, step, result);
  });
}
