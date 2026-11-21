#import <AppKit/AppKit.h>
#include <stdint.h>
#include <stdio.h>
#include <unistd.h>

@interface SWUiAccessibilityCalls : NSObject
@property(nonatomic) BOOL closed;
@property(nonatomic, strong) NSTask *task;
@end
@implementation SWUiAccessibilityCalls
@end

static NSString *text(const uint8_t *bytes, int64_t length) {
  return length < 0 ? nil
                    : [[NSString alloc] initWithBytes:bytes
                                               length:(NSUInteger)length
                                             encoding:NSUTF8StringEncoding];
}

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

void *scoop_ui_ax_root(void *pointer) {
  NSView *view = (__bridge NSView *)pointer;
  return (__bridge_retained void *)view.accessibilityChildren.firstObject;
}

void *scoop_ui_ax_find(void *pointer, const uint8_t *bytes, int64_t length) {
  return (__bridge_retained void *)find((__bridge id)pointer,
                                        text(bytes, length));
}

int64_t scoop_ui_ax_count(void *pointer) {
  return (int64_t)[[(__bridge id)pointer accessibilityChildren] count];
}

void *scoop_ui_ax_child(void *pointer, int64_t index) {
  NSArray *children = [(__bridge id)pointer accessibilityChildren];
  if (index < 0 || (uint64_t)index >= children.count)
    return NULL;
  return (__bridge_retained void *)children[(NSUInteger)index];
}

int32_t scoop_ui_ax_parent(void *pointer, void *parent) {
  return [(__bridge id)pointer accessibilityParent] == (__bridge id)parent;
}

int32_t scoop_ui_ax_text(void *pointer, int32_t field, const uint8_t *bytes,
                         int64_t length) {
  id node = (__bridge id)pointer;
  id value = nil;
  switch (field) {
  case 0:
    value = [node accessibilityLabel];
    break;
  case 1:
    value = [node accessibilityValue];
    break;
  case 2:
    value = [node accessibilityRole];
    break;
  case 3:
    value = [node accessibilityIdentifier];
    break;
  case 4:
    value = [node accessibilitySelectedText];
    break;
  case 5:
    value = [node accessibilityHelp];
    break;
  case 6:
    value = [node accessibilitySubrole];
    break;
  default:
    return 0;
  }
  NSString *expected = text(bytes, length);
  return value == expected || [value isEqual:expected];
}

int64_t scoop_ui_ax_number(void *pointer) {
  id value = [(__bridge id)pointer accessibilityValue];
  return [value isKindOfClass:[NSNumber class]] ? [value longLongValue] : -1;
}

double scoop_ui_ax_scroll_position(void *pointer) {
  id value = [(__bridge id)pointer accessibilityValue];
  return [value isKindOfClass:NSNumber.class] ? [value doubleValue] : -1.0;
}

void *scoop_ui_ax_scroll_bar(void *pointer, bool vertical) {
  id node = (__bridge id)pointer;
  return (
      __bridge_retained void *)(vertical
                                    ? [node accessibilityVerticalScrollBar]
                                    : [node accessibilityHorizontalScrollBar]);
}

int32_t scoop_ui_ax_flag(void *pointer, int32_t field) {
  id node = (__bridge id)pointer;
  switch (field) {
  case 0:
    return [node isAccessibilityElement];
  case 1:
    return [node isAccessibilityEnabled];
  case 2:
    return [node isAccessibilitySelectorAllowed:@selector(accessibilityFrame)];
  case 3:
    return [node
        isAccessibilitySelectorAllowed:@selector(accessibilityPerformPress)];
  case 4:
    return [node isAccessibilityFocused];
  case 5:
    return [node
        isAccessibilitySelectorAllowed:@selector(
                                           accessibilityNumberOfCharacters)];
  case 6:
    return [node
        isAccessibilitySelectorAllowed:@selector(
                                           setAccessibilitySelectedTextRange:)];
  case 7:
    return [node
        isAccessibilitySelectorAllowed:@selector(
                                           setAccessibilitySelectedText:)];
  case 8:
    return [node
        isAccessibilitySelectorAllowed:@selector(accessibilityPerformShowMenu)];
  case 9:
    return [node isAccessibilityRequired];
  case 10:
    return [node
        isAccessibilitySelectorAllowed:@selector(isAccessibilityRequired)];
  case 11:
    return [node
        isAccessibilitySelectorAllowed:@selector(setAccessibilityRequired:)];
  case 12:
    return [node accessibilityHorizontalScrollBar] != nil;
  case 13:
    return [node accessibilityVerticalScrollBar] != nil;
  case 14:
    return [node accessibilityCustomActions].count == 1;
  case 15:
    return [node accessibilityCustomActions].count == 2;
  case 16:
    return [node
        isAccessibilitySelectorAllowed:@selector(setAccessibilityFocused:)];
  case 17:
    return [node isAccessibilityExpanded];
  case 18:
    return [node
        isAccessibilitySelectorAllowed:@selector(setAccessibilityExpanded:)];
  default:
    return 0;
  }
}

int64_t scoop_ui_ax_hierarchy_level(void *pointer) {
  id node = (__bridge id)pointer;
  return [node isAccessibilitySelectorAllowed:@selector(
                                                  accessibilityDisclosureLevel)]
             ? (int64_t)[node accessibilityDisclosureLevel]
             : -1;
}

void *scoop_ui_ax_parent_node(void *pointer) {
  id node = (__bridge id)pointer;
  return (__bridge_retained void *)[node accessibilityParent];
}

int64_t scoop_ui_ax_row_count(void *pointer) {
  id node = (__bridge id)pointer;
  return (int64_t)[node accessibilityRowCount];
}

void *scoop_ui_ax_cell(void *pointer, int64_t row, int64_t column) {
  id node = (__bridge id)pointer;
  return (__bridge_retained void *)[node accessibilityCellForColumn:column
                                                                row:row];
}

void *scoop_ui_ax_column_header(void *pointer, int64_t index) {
  NSArray *headers = [(__bridge id)pointer accessibilityColumnHeaderUIElements];
  return index >= 0 && (uint64_t)index < headers.count
             ? (__bridge_retained void *)headers[(NSUInteger)index]
             : NULL;
}

int32_t scoop_ui_ax_cell_span(void *pointer, int64_t row, int64_t column,
                              int64_t rows, int64_t columns) {
  id node = (__bridge id)pointer;
  return NSEqualRanges([node accessibilityRowIndexRange],
                       NSMakeRange(row, rows)) &&
         NSEqualRanges([node accessibilityColumnIndexRange],
                       NSMakeRange(column, columns));
}

int64_t scoop_ui_ax_row_cell_count(void *pointer) {
  id parent = [(__bridge id)pointer accessibilityParent];
  return (int64_t)[[parent accessibilityChildren] count];
}

int32_t scoop_ui_ax_frame(void *pointer, void *view, double x, double y,
                          double width, double height) {
  NSRect expected = NSAccessibilityFrameInView((__bridge NSView *)view,
                                               NSMakeRect(x, y, width, height));
  NSRect actual = [(__bridge id)pointer accessibilityFrame];
  BOOL equal = NSEqualRects(actual, expected);
  if (!equal)
    fprintf(stderr, "AX frame %s: expected %s, actual %s\n",
            [([(__bridge id)pointer accessibilityIdentifier]
                  ?: @"unnamed") UTF8String],
            NSStringFromRect(expected).UTF8String,
            NSStringFromRect(actual).UTF8String);
  return equal;
}

int32_t scoop_ui_ax_hit(void *pointer, void *viewPointer, double x, double y,
                        void *expected) {
  NSView *view = (__bridge NSView *)viewPointer;
  NSPoint windowPoint = [view convertPoint:NSMakePoint(x, y) toView:nil];
  NSPoint point = [view.window convertPointToScreen:windowPoint];
  return [(__bridge id)pointer accessibilityHitTest:point] ==
         (__bridge id)expected;
}

int32_t scoop_ui_ax_press(void *pointer) {
  return [(__bridge id)pointer accessibilityPerformPress];
}

int64_t scoop_ui_ax_text_count(void *pointer) {
  return [(__bridge id)pointer accessibilityNumberOfCharacters];
}

int32_t scoop_ui_ax_selection(void *pointer, int64_t start, int64_t length) {
  id node = (__bridge id)pointer;
  NSRange expected = NSMakeRange(start < 0 ? NSNotFound : (NSUInteger)start,
                                 (NSUInteger)length);
  NSArray *ranges = [node accessibilitySelectedTextRanges];
  return NSEqualRanges([node accessibilitySelectedTextRange], expected) &&
         (start < 0 ? ranges.count == 0
                    : ranges.count == 1 &&
                          NSEqualRanges([ranges[0] rangeValue], expected));
}

int32_t scoop_ui_ax_text_range(void *pointer, int64_t start, int64_t length,
                               const uint8_t *bytes, int64_t count,
                               bool attributed) {
  NSRange range = NSMakeRange(start < 0 ? NSNotFound : (NSUInteger)start,
                              (NSUInteger)length);
  id value = attributed
                 ? [[(__bridge id)pointer
                       accessibilityAttributedStringForRange:range] string]
                 : [(__bridge id)pointer accessibilityStringForRange:range];
  NSString *expected = text(bytes, count);
  return value == expected || [value isEqual:expected];
}

void scoop_ui_ax_release(void *pointer) { (void)CFBridgingRelease(pointer); }

void *scoop_ui_ax_calls(void) {
  return (__bridge_retained void *)[SWUiAccessibilityCalls new];
}

static BOOL performCustomAction(NSAccessibilityCustomAction *action) {
  if (!action)
    return NO;
  if (action.handler)
    return action.handler();
  id target = action.target;
  SEL selector = action.selector;
  if (!target || !selector || ![target respondsToSelector:selector])
    return NO;
  BOOL (*perform)(id, SEL) =
      (BOOL (*)(id, SEL))[target methodForSelector:selector];
  return perform(target, selector);
}

void scoop_ui_ax_post(void *pointer, void *element, int32_t tag, int32_t action,
                      const uint8_t *bytes, int64_t length,
                      int64_t selectionStart, int64_t selectionLength,
                      int32_t (*callback)(void *, int32_t, int32_t),
                      void *context) {
  SWUiAccessibilityCalls *calls = (__bridge SWUiAccessibilityCalls *)pointer;
  id node = (__bridge id)element;
  NSAccessibilityCustomAction *retainedAction =
      action == 13 ? [node accessibilityCustomActions].firstObject : nil;
  NSString *value = text(bytes, length);
  dispatch_async(dispatch_get_main_queue(), ^{
    if (calls.closed)
      return;
    int32_t result = 0;
    @try {
      switch (action) {
      case 0:
        result = [node accessibilityPerformPress];
        break;
      case 1:
        [node setAccessibilityValue:value];
        result = 1;
        break;
      case 2:
        [node setAccessibilityFocused:YES];
        result = 1;
        break;
      case 3:
        [node setAccessibilityFocused:NO];
        result = 1;
        break;
      case 4:
        [node
            setAccessibilitySelectedTextRange:NSMakeRange(
                                                  (NSUInteger)selectionStart,
                                                  (NSUInteger)selectionLength)];
        result = 1;
        break;
      case 5:
        [node setAccessibilitySelectedText:value];
        result = 1;
        break;
      case 6:
        [node setAccessibilitySelectedTextRanges:@[
          [NSValue valueWithRange:NSMakeRange((NSUInteger)selectionStart,
                                              (NSUInteger)selectionLength)]
        ]];
        result = 1;
        break;
      case 7:
        [node setAccessibilitySelectedTextRanges:@[
          [NSValue valueWithRange:NSMakeRange(0, 1)],
          [NSValue valueWithRange:NSMakeRange(1, 1)]
        ]];
        result = 1;
        break;
      case 8:
        result = [node accessibilityPerformShowMenu] ? 1 : 0;
        break;
      case 9:
        [node setAccessibilityValue:@(value.doubleValue)];
        result = 1;
        break;
      case 10:
        result = [node accessibilityPerformIncrement];
        break;
      case 11:
        result = [node accessibilityPerformDecrement];
        break;
      case 12:
        result =
            performCustomAction([node accessibilityCustomActions].firstObject);
        break;
      case 13:
        result = performCustomAction(retainedAction);
        break;
      case 14:
        [node setAccessibilityExpanded:YES];
        result = 1;
        break;
      case 15:
        [node setAccessibilityExpanded:NO];
        result = 1;
        break;
      case 16:
        [node setAccessibilitySelected:YES];
        result = 1;
        break;
      case 17:
        [node setAccessibilitySelected:NO];
        result = 1;
        break;
      case 18:
        result = [node accessibilityPerformCancel];
        break;
      case 19: {
        NSArray *actions = [node accessibilityCustomActions];
        result = selectionStart >= 0 && (uint64_t)selectionStart < actions.count
                     ? performCustomAction(actions[(NSUInteger)selectionStart])
                     : 0;
        break;
      }
      case 20:
        result = 1;
        break;
      default:
        result = -1;
      }
    } @catch (NSException *exception) {
      result = -2;
    }
    if (!calls.closed)
      callback(context, tag, result);
  });
}

void scoop_ui_ax_close_calls(void *pointer) {
  SWUiAccessibilityCalls *calls = CFBridgingRelease(pointer);
  calls.closed = YES;
  calls.task.terminationHandler = nil;
  if (calls.task.running)
    [calls.task terminate];
  calls.task = nil;
}

int32_t scoop_ui_ax_text_client(void *pointer, void *element, void *viewPointer,
                                int32_t tag, double x, double y, double width,
                                double height,
                                int32_t (*callback)(void *, int32_t, int32_t),
                                void *context) {
  const char *client = getenv("SCOOP_ACCESSIBILITY_TEXT_CLIENT");
  if (!client || !*client)
    return 0;
  SWUiAccessibilityCalls *calls = (__bridge SWUiAccessibilityCalls *)pointer;
  NSRect screen = NSAccessibilityFrameInView((__bridge NSView *)viewPointer,
                                             NSMakeRect(x, y, width, height));
  // AXUIElement uses a top-left origin on the primary display.
  double top = NSMaxY(NSScreen.screens.firstObject.frame) - NSMaxY(screen);
  NSTask *task = [NSTask new];
  calls.task = task;
  task.executableURL =
      [NSURL fileURLWithPath:[NSString stringWithUTF8String:client]];
  task.arguments = @[
    [NSString stringWithFormat:@"%d", getpid()],
    [(__bridge id)element accessibilityIdentifier],
    [NSString stringWithFormat:@"%.17g", screen.origin.x],
    [NSString stringWithFormat:@"%.17g", top],
    [NSString stringWithFormat:@"%.17g", screen.size.width],
    [NSString stringWithFormat:@"%.17g", screen.size.height]
  ];
  task.terminationHandler = ^(NSTask *finished) {
    int32_t result =
        finished.terminationStatus ? -100 - finished.terminationStatus : 1;
    dispatch_async(dispatch_get_main_queue(), ^{
      if (!calls.closed) {
        calls.task = nil;
        callback(context, tag, result);
      }
    });
  };
  NSError *error = nil;
  if (![task launchAndReturnError:&error]) {
    calls.task = nil;
    task.terminationHandler = nil;
    dispatch_async(dispatch_get_main_queue(), ^{
      if (!calls.closed)
        callback(context, tag, -3);
    });
  }
  return 1;
}
