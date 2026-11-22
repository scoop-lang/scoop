#import <AppKit/AppKit.h>
#include <stdint.h>

@interface SWGeometryProbe : NSObject
@property(nonatomic, weak) NSView *view;
@property(nonatomic, strong) NSArray *nodes;
@end
@implementation SWGeometryProbe
@end

static id find(id node, NSString *identifier) {
  if ([[node accessibilityIdentifier] isEqualToString:identifier])
    return node;
  for (id child in [node accessibilityChildren]) {
    id found = find(child, identifier);
    if (found)
      return found;
  }
  return nil;
}

void *scoop_ax_geometry_open(void *pointer) {
  NSView *view = (__bridge NSView *)pointer;
  id root = view.accessibilityChildren.firstObject;
  id group = find(root, @"hit-group");
  id back = find(root, @"hit-back");
  id front = find(root, @"hit-front");
  id label = find(root, @"hit-label");
  if (!root || !group || !back || !front || !label)
    return NULL;
  SWGeometryProbe *probe = [SWGeometryProbe new];
  probe.view = view;
  probe.nodes = @[ root, group, back, front, label ];
  return (__bridge_retained void *)probe;
}

int32_t scoop_ax_geometry_hit(void *pointer, int32_t source, double x, double y,
                              int32_t expected) {
  SWGeometryProbe *probe = (__bridge SWGeometryProbe *)pointer;
  NSView *view = probe.view;
  NSPoint windowPoint = [view convertPoint:NSMakePoint(x, y) toView:nil];
  NSPoint point = [view.window convertPointToScreen:windowPoint];
  id node = source < 0 ? view : probe.nodes[(NSUInteger)source];
  id target = expected < 0 ? nil : probe.nodes[(NSUInteger)expected];
  return [node accessibilityHitTest:point] == target ? 0 : __LINE__;
}

int32_t scoop_ax_geometry_moved_back(void *pointer) {
  SWGeometryProbe *probe = (__bridge SWGeometryProbe *)pointer;
  NSView *view = probe.view;
  id back = probe.nodes[2];
  NSRect frame = [view convertRect:NSMakeRect(115, 10, 160, 100) toView:nil];
  NSPoint activation = [view convertPoint:NSMakePoint(130, 20) toView:nil];
  if (!NSEqualRects([back accessibilityFrame],
                    [view.window convertRectToScreen:frame]))
    return __LINE__;
  if (!NSEqualPoints([back accessibilityActivationPoint],
                     [view.window convertPointToScreen:activation]))
    return __LINE__;
  return [[back accessibilityTitle] isEqualToString:@"hit-back"] &&
                 [back isAccessibilityRequired]
             ? 0
             : __LINE__;
}

void scoop_ax_geometry_close(void *pointer) { CFRelease(pointer); }
