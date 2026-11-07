#import "window.h"

@interface SWFrameOperation : NSObject
@property(nonatomic, strong) SWWindow *owner;
@property(nonatomic, strong) NSEvent *event;
@end
@implementation SWFrameOperation
@end

SWResult scoop_window_input_copy(void *pointer) {
  SWEvent *event = (__bridge SWEvent *)pointer;
  return (SWResult){event.nativeInput ? (__bridge_retained void *)event : NULL,
                    0};
}

void scoop_window_input_release(void *pointer) {
  SWEvent *event = (__bridge_transfer SWEvent *)pointer;
  (void)event;
}

SWResult scoop_window_system_operation_prepare(void *pointer, void *input,
                                               int32_t kind, int32_t edge) {
  if (kind != 0)
    return (SWResult){NULL, -1};
  SWWindow *owner = (__bridge SWWindow *)pointer;
  SWEvent *event = (__bridge SWEvent *)input;
  if (event.data.window != owner.identity ||
      event.nativeInput.type != NSEventTypeLeftMouseDown ||
      event.nativeInput.window != owner.window ||
      owner.window.ignoresMouseEvents)
    return (SWResult){NULL, -4};
  SWFrameOperation *operation = [SWFrameOperation new];
  operation.owner = owner;
  operation.event = event.nativeInput;
  [owner.view.pressedButtons removeAllObjects];
  return (SWResult){(__bridge_retained void *)operation, 0};
}

int64_t scoop_window_system_operation_run(void *pointer) {
  SWFrameOperation *operation = (__bridge SWFrameOperation *)pointer;
  [operation.owner.window performWindowDragWithEvent:operation.event];
  return 0;
}

void scoop_window_system_operation_release(void *pointer) {
  SWFrameOperation *operation = (__bridge_transfer SWFrameOperation *)pointer;
  (void)operation;
}
