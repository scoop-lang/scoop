#import <AppKit/AppKit.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
  uint8_t *bytes;
  int64_t length, error;
} SWClipboardText;

SWClipboardText scoop_window_clipboard_read(void) {
  @autoreleasepool {
    @try {
      NSString *text =
          [NSPasteboard.generalPasteboard stringForType:NSPasteboardTypeString];
      if (!text)
        return (SWClipboardText){0};
      NSData *data = [text dataUsingEncoding:NSUTF8StringEncoding
                        allowLossyConversion:NO];
      if (!data || data.length > INT64_MAX || data.length == SIZE_MAX)
        return (SWClipboardText){.error = 1};
      uint8_t *bytes = malloc(data.length + 1);
      if (!bytes)
        return (SWClipboardText){.error = 2};
      if (data.length)
        memcpy(bytes, data.bytes, data.length);
      bytes[data.length] = 0;
      return (SWClipboardText){bytes, (int64_t)data.length, 0};
    } @catch (NSException *exception) {
      return (SWClipboardText){.error = 3};
    }
  }
}

void scoop_window_clipboard_release(uint8_t *bytes) { free(bytes); }

int64_t scoop_window_clipboard_write(const uint8_t *bytes, int64_t length) {
  @autoreleasepool {
    @try {
      if (length < 0)
        return 1;
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
