#import <AppKit/AppKit.h>
#import <UniformTypeIdentifiers/UniformTypeIdentifiers.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
  uint8_t *bytes;
  int64_t length, error;
} SWClipboardText;

typedef struct {
  void *pointer;
  int64_t error;
} SWClipboardResult;

static NSString *clipboard_type(NSString *mime) {
  if ([mime isEqualToString:@"text/plain"])
    return NSPasteboardTypeString;
  return [UTType typeWithMIMEType:mime conformingToType:UTTypeText].identifier;
}

SWClipboardText scoop_window_clipboard_read(const uint8_t *type_bytes,
                                            int64_t type_length) {
  @autoreleasepool {
    @try {
      NSString *mime = [[NSString alloc] initWithBytes:type_bytes
                                                length:(NSUInteger)type_length
                                              encoding:NSUTF8StringEncoding];
      NSString *type = clipboard_type(mime);
      if (!type)
        return (SWClipboardText){.error = 1};
      NSPasteboard *pasteboard = NSPasteboard.generalPasteboard;
      NSData *data = [pasteboard dataForType:type];
      if (!data)
        return (SWClipboardText){0};
      if (data.length > INT64_MAX || data.length == SIZE_MAX)
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

SWClipboardResult scoop_window_clipboard_create(void) {
  @autoreleasepool {
    @try {
      NSPasteboardItem *item = [[NSPasteboardItem alloc] init];
      if (!item)
        return (SWClipboardResult){.error = 2};
      return (SWClipboardResult){(void *)CFBridgingRetain(item), 0};
    } @catch (NSException *exception) {
      return (SWClipboardResult){.error = 3};
    }
  }
}

int64_t scoop_window_clipboard_add(void *writer, const uint8_t *type_bytes,
                                   int64_t type_length, const uint8_t *bytes,
                                   int64_t length) {
  @autoreleasepool {
    @try {
      NSString *mime = [[NSString alloc] initWithBytes:type_bytes
                                                length:(NSUInteger)type_length
                                              encoding:NSUTF8StringEncoding];
      NSString *type = clipboard_type(mime);
      if (!type || length < 0)
        return 1;
      NSData *data = [NSData dataWithBytes:bytes length:(NSUInteger)length];
      NSPasteboardItem *item = (__bridge NSPasteboardItem *)writer;
      return [item setData:data forType:type] ? 0 : 2;
    } @catch (NSException *exception) {
      return 3;
    }
  }
}

int64_t scoop_window_clipboard_write(void *writer, void *application) {
  (void)application;
  @autoreleasepool {
    @try {
      NSPasteboard *pasteboard = NSPasteboard.generalPasteboard;
      NSPasteboardItem *item = (__bridge NSPasteboardItem *)writer;
      [pasteboard clearContents];
      return [pasteboard writeObjects:@[ item ]] ? 0 : 2;
    } @catch (NSException *exception) {
      return 3;
    }
  }
}

void scoop_window_clipboard_destroy(void *writer) { CFRelease(writer); }
