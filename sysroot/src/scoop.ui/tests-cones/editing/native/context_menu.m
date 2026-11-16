#import <AppKit/AppKit.h>
#include <stdbool.h>
#include <stdint.h>

static int32_t semanticMenuResult;

static id findSemanticText(id node, NSString *identifier) {
  if ([[node accessibilityIdentifier] isEqualToString:identifier])
    return node;
  for (id child in [node accessibilityChildren]) {
    id found = findSemanticText(child, identifier);
    if (found)
      return found;
  }
  return nil;
}

int32_t scoop_test_editor_semantic_menu(void *pointer, const uint8_t *bytes,
                                        int64_t length, bool request) {
  NSView *view = (__bridge NSView *)pointer;
  NSString *identifier = [[NSString alloc] initWithBytes:bytes
                                                  length:(NSUInteger)length
                                                encoding:NSUTF8StringEncoding];
  id node = findSemanticText(view, identifier);
  if (!node)
    return -1;
  if (!request)
    return [node
        isAccessibilitySelectorAllowed:@selector(accessibilityPerformShowMenu)];
  semanticMenuResult = -1;
  dispatch_async(dispatch_get_main_queue(), ^{
    semanticMenuResult = [node accessibilityPerformShowMenu] ? 1 : 0;
  });
  return 0;
}

int32_t scoop_test_editor_semantic_menu_result(void) {
  return semanticMenuResult;
}

int32_t scoop_test_editor_context_click(void *pointer, double x, double y,
                                        bool control) {
  NSView *view = (__bridge NSView *)pointer;
  for (int release = 0; release != 2; ++release) {
    NSEventType type =
        control
            ? (release ? NSEventTypeLeftMouseUp : NSEventTypeLeftMouseDown)
            : (release ? NSEventTypeRightMouseUp : NSEventTypeRightMouseDown);
    NSEvent *event = [NSEvent
        mouseEventWithType:type
                  location:[view convertPoint:NSMakePoint(x, y) toView:nil]
             modifierFlags:control ? NSEventModifierFlagControl : 0
                 timestamp:NSProcessInfo.processInfo.systemUptime
              windowNumber:view.window.windowNumber
                   context:nil
               eventNumber:1
                clickCount:1
                  pressure:release ? 0.0 : 1.0];
    // The NSEvent constructor leaves buttonNumber at zero for right events.
    if (!control) {
      CGEventRef input = CGEventCreateCopy(event.CGEvent);
      if (!input)
        return 1;
      CGEventSetIntegerValueField(input, kCGMouseEventButtonNumber,
                                  kCGMouseButtonRight);
      event = [NSEvent eventWithCGEvent:input];
      CFRelease(input);
    }
    if (control) {
      if (release)
        [view mouseUp:event];
      else
        [view mouseDown:event];
    } else {
      if (release)
        [view rightMouseUp:event];
      else
        [view rightMouseDown:event];
    }
  }
  return 0;
}
