#import "drag.h"
#include <string.h>

NSDraggingItem *sw_drag_item(int32_t format, const uint8_t *bytes,
                             int64_t length) {
  if (length < 0 || (length && !bytes))
    return nil;
  id<NSPasteboardWriting> writer = nil;
  if (format == 3) {
    if (!length || bytes[0] != '/' || memchr(bytes, 0, (size_t)length))
      return nil;
    NSMutableData *path = [NSMutableData dataWithLength:(NSUInteger)length + 1];
    memcpy(path.mutableBytes, bytes, (size_t)length);
    NSURL *url = [NSURL fileURLWithFileSystemRepresentation:path.bytes
                                                isDirectory:NO
                                              relativeToURL:nil];
    const char *copied = url.fileSystemRepresentation;
    if (!copied || strlen(copied) != (size_t)length ||
        memcmp(copied, bytes, (size_t)length))
      return nil;
    writer = url;
  } else if (format == 1 || format == 2) {
    NSString *text = [[NSString alloc] initWithBytes:bytes
                                              length:(NSUInteger)length
                                            encoding:NSUTF8StringEncoding];
    if (!text)
      return nil;
    NSPasteboardItem *item = [NSPasteboardItem new];
    if (format == 2) {
      NSURL *url = [NSURL URLWithString:text];
      if (!url || !url.scheme.length)
        return nil;
      if (![item setString:url.absoluteString forType:NSPasteboardTypeURL])
        return nil;
    }
    if (![item setString:text forType:NSPasteboardTypeString])
      return nil;
    writer = item;
  }
  return writer ? [[NSDraggingItem alloc] initWithPasteboardWriter:writer]
                : nil;
}
