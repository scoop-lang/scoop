#import "window.h"

int32_t scoop_window_frame_capabilities(void) {
  return SW_FRAME_CLIENT | SW_FRAME_EXTENDED | SW_FRAME_SYSTEM_RESIZE |
         SW_FRAME_MOVE | SW_FRAME_IGNORE | SW_FRAME_SHADOW;
}

void sw_frame_invalidate(SWWindow *owner) {
  owner.hitRegions = nil;
  owner.frameWidth = owner.view.bounds.size.width;
  owner.frameHeight = owner.view.bounds.size.height;
  owner.frameScale = owner.window.backingScaleFactor;
  [owner emit:17];
}

void sw_frame_geometry(SWWindow *owner) {
  NSSize size = owner.view.bounds.size;
  if (size.width != owner.frameWidth || size.height != owner.frameHeight ||
      owner.window.backingScaleFactor != owner.frameScale)
    sw_frame_invalidate(owner);
}

int64_t scoop_window_frame(void *pointer, int32_t mode) {
  SWWindow *owner = (__bridge SWWindow *)pointer;
  NSWindowStyleMask style = owner.window.styleMask;
  style &=
      ~(NSWindowStyleMaskTitled | NSWindowStyleMaskClosable |
        NSWindowStyleMaskMiniaturizable | NSWindowStyleMaskFullSizeContentView);
  if (mode != 2)
    style |= NSWindowStyleMaskTitled | NSWindowStyleMaskClosable |
             NSWindowStyleMaskMiniaturizable;
  if (mode == 1)
    style |= NSWindowStyleMaskFullSizeContentView;
  if (style == owner.window.styleMask)
    return 0;
  NSSize content = owner.view.bounds.size;
  owner.window.styleMask = style;
  [owner.window setContentSize:content];
  [owner.window makeFirstResponder:owner.view];
  sw_frame_invalidate(owner);
  [owner emit:2];
  return 0;
}

SWFrameSnapshot scoop_window_frame_snapshot(void *pointer) {
  SWWindow *owner = (__bridge SWWindow *)pointer;
  NSWindowStyleMask style = owner.window.styleMask;
  int mode = !(style & NSWindowStyleMaskTitled)               ? 2
             : (style & NSWindowStyleMaskFullSizeContentView) ? 1
                                                              : 0;
  NSRect layout = [owner.view convertRect:owner.window.contentLayoutRect
                                 fromView:nil];
  return (SWFrameSnapshot){
      mode,
      1,
      {layout.origin.x, layout.origin.y, layout.size.width, layout.size.height},
      NAN};
}

SWFrameRect scoop_window_frame_button(void *pointer, int32_t role) {
  SWWindow *owner = (__bridge SWWindow *)pointer;
  NSWindowButton kind = role == 0   ? NSWindowCloseButton
                        : role == 1 ? NSWindowMiniaturizeButton
                                    : NSWindowZoomButton;
  NSButton *button = [owner.window standardWindowButton:kind];
  if (!button || button.hidden || !button.superview ||
      !(owner.window.styleMask & NSWindowStyleMaskTitled))
    return sw_frame_unknown();
  NSRect rect = [owner.view convertRect:button.bounds fromView:button];
  return (SWFrameRect){rect.origin.x, rect.origin.y, rect.size.width,
                       rect.size.height};
}

int64_t scoop_window_shadow(void *pointer, int32_t enabled) {
  ((__bridge SWWindow *)pointer).window.hasShadow = enabled != 0;
  return 0;
}

int64_t scoop_window_region(void *pointer, int32_t kind, int32_t present,
                            const SWFrameRect *rects, int64_t count) {
  return present ? -1 : 0;
}

int64_t scoop_window_ignore_mouse(void *pointer, int32_t enabled) {
  SWWindow *owner = (__bridge SWWindow *)pointer;
  if (owner.inputBlocked)
    owner.savedIgnoreMouse = enabled != 0;
  else
    owner.window.ignoresMouseEvents = enabled != 0;
  if (enabled)
    [owner.view cancelPointer:1];
  return 0;
}

int64_t scoop_window_hit_regions(void *pointer, const SWHitRect *rects,
                                 int64_t count) {
  SWWindow *owner = (__bridge SWWindow *)pointer;
  if (count < 0 || (uint64_t)count > SIZE_MAX / sizeof(*rects))
    return EOVERFLOW;
  for (int64_t index = 0; index < count; ++index) {
    if (!sw_frame_rect_valid(rects[index].rect))
      return EINVAL;
    if (rects[index].role != 0 && rects[index].role != 1)
      return -1;
  }
  owner.hitRegions =
      count ? [NSData dataWithBytes:rects
                             length:(NSUInteger)count * sizeof(*rects)]
            : nil;
  return 0;
}

BOOL sw_frame_pointer(SWView *view, NSEvent *event) {
  if (event.type != NSEventTypeLeftMouseDown)
    return NO;
  SWWindow *owner = view.owner;
  NSPoint point = [view convertPoint:event.locationInWindow fromView:nil];
  if (sw_frame_hit(owner.hitRegions.bytes,
                   owner.hitRegions.length / sizeof(SWHitRect), point.x,
                   point.y) != 1)
    return NO;
  [view cancelPointer:2];
  [owner.window performWindowDragWithEvent:event];
  return YES;
}

void scoop_window_frame_retire(void *pointer) {
  ((__bridge SWWindow *)pointer).hitRegions = nil;
}
