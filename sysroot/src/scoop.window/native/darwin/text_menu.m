#import "window.h"

@interface SWTextMenuRow : NSObject
@property(nonatomic) int32_t role;
@property(nonatomic, strong) NSData *text;
@end
@implementation SWTextMenuRow
@end

@interface SWTextMenuTemplate : NSObject
@property(nonatomic) BOOL system;
@property(nonatomic, strong) NSMutableArray<SWTextMenuRow *> *rows;
@end
@implementation SWTextMenuTemplate
@end

static int32_t role(SEL selector) {
  if (selector == @selector(cut:))
    return 4;
  if (selector == @selector(copy:))
    return 5;
  if (selector == @selector(paste:))
    return 6;
  if (selector == @selector(selectAll:))
    return 7;
  if (selector == @selector(undo:))
    return 8;
  if (selector == @selector(redo:))
    return 9;
  if (selector == @selector(delete:))
    return 10;
  return 0;
}

static void append(SWTextMenuTemplate *value, int32_t command,
                   NSString *label) {
  if (!command && (!value.rows.count || !value.rows.lastObject.role))
    return;
  SWTextMenuRow *row = [SWTextMenuRow new];
  row.role = command;
  row.text = [label dataUsingEncoding:NSUTF8StringEncoding];
  [value.rows addObject:row];
}

SWResult scoop_window_text_menu_create(int32_t prefer_system) {
  @autoreleasepool {
    @try {
      SWTextMenuTemplate *value = [SWTextMenuTemplate new];
      value.rows = [NSMutableArray new];
      NSMenu *system = NSTextView.defaultMenu;
      NSUndoManager *undo = [NSUndoManager new];
      NSString *labels[11] = {nil,
                              nil,
                              nil,
                              nil,
                              @"Cut",
                              @"Copy",
                              @"Paste",
                              @"Select All",
                              undo.undoMenuItemTitle,
                              undo.redoMenuItemTitle,
                              @"Delete"};
      for (NSMenuItem *item in system.itemArray) {
        int32_t command = role(item.action);
        if (command && item.title.length)
          labels[command] = item.title;
      }
      if (prefer_system) {
        BOOL seen[11] = {NO};
        append(value, 8, labels[8]);
        append(value, 9, labels[9]);
        append(value, 0, nil);
        seen[8] = seen[9] = YES;
        for (NSMenuItem *item in system.itemArray) {
          int32_t command = role(item.action);
          if (command && !seen[command]) {
            append(value, command, labels[command]);
            seen[command] = YES;
            value.system = YES;
          } else if (item.separatorItem) {
            append(value, 0, nil);
          }
        }
        for (int32_t command = 4; command <= 10; ++command) {
          if (seen[command])
            continue;
          if (command == 7)
            continue;
          append(value, command, labels[command]);
        }
        if (!seen[7]) {
          append(value, 0, nil);
          append(value, 7, labels[7]);
        }
      } else {
        const int32_t commands[] = {8, 9, 0, 4, 5, 6, 10, 0, 7};
        for (size_t index = 0; index < sizeof(commands) / sizeof(commands[0]);
             ++index)
          append(value, commands[index], labels[commands[index]]);
      }
      if (!value.rows.lastObject.role)
        [value.rows removeLastObject];
      return (SWResult){(__bridge_retained void *)value, 0};
    } @catch (NSException *exception) {
      return (SWResult){NULL, 1};
    }
  }
}

int64_t scoop_window_text_menu_count(void *pointer) {
  return (int64_t)((__bridge SWTextMenuTemplate *)pointer).rows.count;
}

typedef struct {
  int32_t role;
  const uint8_t *bytes;
  int64_t length;
} SWTextMenuEntry;

SWTextMenuEntry scoop_window_text_menu_entry(void *pointer, int64_t index) {
  SWTextMenuRow *row =
      ((__bridge SWTextMenuTemplate *)pointer).rows[(NSUInteger)index];
  return (SWTextMenuEntry){row.role, row.text.bytes, (int64_t)row.text.length};
}

int32_t scoop_window_text_menu_uses_system(void *pointer) {
  return ((__bridge SWTextMenuTemplate *)pointer).system;
}

void scoop_window_text_menu_release(void *pointer) {
  SWTextMenuTemplate *value = (__bridge_transfer SWTextMenuTemplate *)pointer;
  (void)value;
}
