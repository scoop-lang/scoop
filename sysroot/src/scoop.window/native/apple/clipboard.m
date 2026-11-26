#import <TargetConditionals.h>
#if TARGET_OS_IOS
#import <UIKit/UIKit.h>
typedef NSMutableDictionary<NSString *, NSData *> SWClipboardItem;
#else
#import <AppKit/AppKit.h>
typedef NSPasteboardItem SWClipboardItem;
#endif
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
    return UTTypeUTF8PlainText.identifier;
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
#if TARGET_OS_IOS
      UIPasteboard *pasteboard = UIPasteboard.generalPasteboard;
      NSData *data = [pasteboard dataForPasteboardType:type];
      if (!data && [mime isEqualToString:@"text/plain"])
        data = [pasteboard.string dataUsingEncoding:NSUTF8StringEncoding];
#else
      NSData *data = [NSPasteboard.generalPasteboard dataForType:type];
#endif
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
      SWClipboardItem *item = [SWClipboardItem new];
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
      SWClipboardItem *item = (__bridge SWClipboardItem *)writer;
#if TARGET_OS_IOS
      item[type] = data;
      return 0;
#else
      return [item setData:data forType:type] ? 0 : 2;
#endif
    } @catch (NSException *exception) {
      return 3;
    }
  }
}

int64_t scoop_window_clipboard_write(void *writer, void *application) {
  (void)application;
  @autoreleasepool {
    @try {
      SWClipboardItem *item = (__bridge SWClipboardItem *)writer;
#if TARGET_OS_IOS
      UIPasteboard.generalPasteboard.items = @[ item ];
      return 0;
#else
      NSPasteboard *pasteboard = NSPasteboard.generalPasteboard;
      [pasteboard clearContents];
      return [pasteboard writeObjects:@[ item ]] ? 0 : 2;
#endif
    } @catch (NSException *exception) {
      return 3;
    }
  }
}

void scoop_window_clipboard_destroy(void *writer) { CFRelease(writer); }

// These platforms have no native X11/Wayland PRIMARY buffer.
typedef struct {
  int64_t generation, error;
} SWPrimaryClaim;

int32_t scoop_window_primary_flags(void *application) {
  (void)application;
  return 0;
}

SWClipboardText scoop_window_primary_read(void *application) {
  (void)application;
  return (SWClipboardText){0};
}

SWPrimaryClaim scoop_window_primary_claim(void *application,
                                          const uint8_t *text, int64_t length) {
  (void)application;
  (void)text;
  (void)length;
  return (SWPrimaryClaim){0};
}

int32_t scoop_window_primary_current(void *application, int64_t generation) {
  (void)application;
  (void)generation;
  return 0;
}

int64_t scoop_window_primary_release(void *application, int64_t generation) {
  (void)application;
  (void)generation;
  return 0;
}
