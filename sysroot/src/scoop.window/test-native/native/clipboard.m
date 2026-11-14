#import <AppKit/AppKit.h>
#import <objc/runtime.h>
#include <stdint.h>

static Method write_method;
static IMP original_write;

int32_t scoop_window_test_clipboard_text(const uint8_t *bytes, int64_t length) {
  @autoreleasepool {
    @try {
      NSString *text = [[NSString alloc] initWithBytes:bytes
                                                length:(NSUInteger)length
                                              encoding:NSUTF8StringEncoding];
      if (!text)
        return 1;
      NSPasteboard *pasteboard = NSPasteboard.generalPasteboard;
      [pasteboard clearContents];
      return [pasteboard writeObjects:@[ text ]] ? 0 : 2;
    } @catch (NSException *exception) {
      return 3;
    }
  }
}

static void restore_write(void) {
  if (original_write) {
    method_setImplementation(write_method, original_write);
    original_write = NULL;
    write_method = NULL;
  }
}

static BOOL fail_write(id self, SEL selector, NSArray *objects) {
  restore_write();
  return NO;
}

int32_t scoop_window_test_clipboard_fail_write(void) {
  if (original_write)
    return 1;
  write_method = class_getInstanceMethod(NSPasteboard.generalPasteboard.class,
                                         @selector(writeObjects:));
  if (!write_method)
    return 2;
  original_write = method_setImplementation(write_method, (IMP)fail_write);
  return 0;
}

void *scoop_window_test_clipboard_save(void) {
  @autoreleasepool {
    @try {
      NSMutableArray<NSPasteboardItem *> *copies = [NSMutableArray array];
      for (NSPasteboardItem *item in NSPasteboard.generalPasteboard
               .pasteboardItems) {
        NSPasteboardItem *copy = [[NSPasteboardItem alloc] init];
        for (NSPasteboardType type in item.types) {
          NSData *data = [item dataForType:type];
          if (!data || ![copy setData:data forType:type])
            return NULL;
        }
        [copies addObject:copy];
      }
      return (void *)CFBridgingRetain([copies copy]);
    } @catch (NSException *exception) {
      return NULL;
    }
  }
}

int32_t scoop_window_test_clipboard_restore(void *snapshot) {
  @autoreleasepool {
    NSArray<NSPasteboardItem *> *items = CFBridgingRelease(snapshot);
    restore_write();
    @try {
      NSPasteboard *pasteboard = NSPasteboard.generalPasteboard;
      [pasteboard clearContents];
      return items.count == 0 || [pasteboard writeObjects:items] ? 0 : 1;
    } @catch (NSException *exception) {
      return 2;
    }
  }
}

int32_t scoop_window_test_clipboard_nontext(void) {
  @autoreleasepool {
    @try {
      NSPasteboardItem *item = [[NSPasteboardItem alloc] init];
      NSData *data = [@"native-test" dataUsingEncoding:NSUTF8StringEncoding];
      if (![item setData:data forType:@"dev.scoop.clipboard-test"])
        return 1;
      NSPasteboard *pasteboard = NSPasteboard.generalPasteboard;
      [pasteboard clearContents];
      return [pasteboard writeObjects:@[ item ]] ? 0 : 2;
    } @catch (NSException *exception) {
      return 3;
    }
  }
}

int32_t scoop_window_test_clipboard_data(const uint8_t *type_bytes,
                                         int64_t type_length,
                                         const uint8_t *bytes, int64_t length) {
  @autoreleasepool {
    @try {
      NSString *type = [[NSString alloc] initWithBytes:type_bytes
                                                length:(NSUInteger)type_length
                                              encoding:NSUTF8StringEncoding];
      NSData *data = [NSData dataWithBytes:bytes length:(NSUInteger)length];
      NSPasteboardItem *item = [[NSPasteboardItem alloc] init];
      if (!type || ![item setData:data forType:type])
        return 1;
      NSPasteboard *pasteboard = NSPasteboard.generalPasteboard;
      [pasteboard clearContents];
      return [pasteboard writeObjects:@[ item ]] ? 0 : 2;
    } @catch (NSException *exception) {
      return 3;
    }
  }
}

int32_t scoop_window_test_clipboard_matches(const uint8_t *type_bytes,
                                            int64_t type_length,
                                            const uint8_t *bytes,
                                            int64_t length) {
  @autoreleasepool {
    @try {
      NSString *type = [[NSString alloc] initWithBytes:type_bytes
                                                length:(NSUInteger)type_length
                                              encoding:NSUTF8StringEncoding];
      NSData *actual = [NSPasteboard.generalPasteboard dataForType:type];
      NSData *expected = [NSData dataWithBytes:bytes length:(NSUInteger)length];
      return actual && [actual isEqualToData:expected] ? 0 : 1;
    } @catch (NSException *exception) {
      return 2;
    }
  }
}

int64_t scoop_window_test_clipboard_count(void) {
  @autoreleasepool {
    return (int64_t)NSPasteboard.generalPasteboard.pasteboardItems.count;
  }
}
