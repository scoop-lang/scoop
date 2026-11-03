#import <AppKit/AppKit.h>
#import <Carbon/Carbon.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
  uint8_t *bytes;
  int64_t length, error;
} SWKeyLabel;

SWKeyLabel scoop_window_key_label(uint32_t scan) {
  @autoreleasepool {
    TISInputSourceRef source = TISCopyCurrentKeyboardLayoutInputSource();
    if (!source)
      return (SWKeyLabel){0};
    @try {
      CFDataRef layout =
          TISGetInputSourceProperty(source, kTISPropertyUnicodeKeyLayoutData);
      if (!layout)
        return (SWKeyLabel){0};
      UniChar characters[255];
      UniCharCount count = 0;
      UInt32 dead_state = 0;
      OSStatus status = UCKeyTranslate(
          (const UCKeyboardLayout *)CFDataGetBytePtr(layout), (UInt16)scan,
          kUCKeyActionDisplay, 0, LMGetKbdType(), kUCKeyTranslateNoDeadKeysMask,
          &dead_state, 255, &count, characters);
      if (status != noErr)
        return (SWKeyLabel){.error = 1};
      if (!count)
        return (SWKeyLabel){0};
      NSString *text = [[NSString alloc] initWithCharacters:characters
                                                     length:count];
      NSData *data = [text dataUsingEncoding:NSUTF8StringEncoding
                        allowLossyConversion:NO];
      if (!data || data.length > INT64_MAX || data.length == SIZE_MAX)
        return (SWKeyLabel){.error = 2};
      uint8_t *bytes = malloc(data.length + 1);
      if (!bytes)
        return (SWKeyLabel){.error = 3};
      if (data.length)
        memcpy(bytes, data.bytes, data.length);
      bytes[data.length] = 0;
      return (SWKeyLabel){bytes, (int64_t)data.length, 0};
    } @catch (NSException *exception) {
      return (SWKeyLabel){.error = 4};
    } @finally {
      CFRelease(source);
    }
  }
}

void scoop_window_key_label_release(uint8_t *bytes) { free(bytes); }
