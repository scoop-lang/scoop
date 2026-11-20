#import "../../native/darwin/drag.h"
#include <string.h>

#define DRAG_CHECK(value)                                                      \
  do {                                                                         \
    if (!(value))                                                              \
      return __LINE__;                                                         \
  } while (0)

int32_t scoop_window_test_drag_data(void) {
  @autoreleasepool {
    NSPasteboard *board = [NSPasteboard pasteboardWithUniqueName];
    NSURL *directory = [NSURL
        fileURLWithPath:[NSTemporaryDirectory()
                            stringByAppendingPathComponent:NSUUID.UUID
                                                               .UUIDString]
            isDirectory:YES];
    @try {
      NSData *text = [@"A\r\n🫖e\u0301" dataUsingEncoding:NSUTF8StringEncoding];
      NSDraggingItem *item = sw_drag_item(1, text.bytes, (int64_t)text.length);
      DRAG_CHECK(item);
      DRAG_CHECK([board writeObjects:@[ item.item ]]);
      DRAG_CHECK([[board stringForType:NSPasteboardTypeString]
          isEqualToString:@"A\r\n🫖e\u0301"]);
      [board clearContents];
      item = sw_drag_item(1, NULL, 0);
      DRAG_CHECK(item && [board writeObjects:@[ item.item ]]);
      DRAG_CHECK(
          [[board stringForType:NSPasteboardTypeString] isEqualToString:@""]);
      [board clearContents];
      NSData *address = [@"https://example.test/a%20b?q=caf%C3%A9"
          dataUsingEncoding:NSUTF8StringEncoding];
      item = sw_drag_item(2, address.bytes, (int64_t)address.length);
      DRAG_CHECK(item && [board writeObjects:@[ item.item ]]);
      DRAG_CHECK(board.pasteboardItems.count == 1);
      DRAG_CHECK([[board stringForType:NSPasteboardTypeURL]
          isEqualToString:[board stringForType:NSPasteboardTypeString]]);
      const uint8_t invalid[] = {0xff};
      DRAG_CHECK(!sw_drag_item(1, invalid, 1));
      DRAG_CHECK(!sw_drag_item(2, (const uint8_t *)"relative", 8));
      DRAG_CHECK(!sw_drag_item(3, (const uint8_t *)"relative", 8));
      DRAG_CHECK(!sw_drag_item(3, (const uint8_t *)"/a\0b", 4));
      DRAG_CHECK(!sw_drag_item(9, text.bytes, (int64_t)text.length));
      DRAG_CHECK([NSFileManager.defaultManager createDirectoryAtURL:directory
                                        withIntermediateDirectories:NO
                                                         attributes:nil
                                                              error:nil]);
      uint8_t pixels[4 * 6 * 4] = {17, 29, 189, 127};
      NSSize size = NSMakeSize(2, 3);
      NSBitmapImageRep *rep = sw_rgba_bitmap(4, 6, pixels, size);
      memset(pixels, 0, sizeof pixels);
      DRAG_CHECK(rep.bitmapData[0] == 17 && rep.bitmapData[3] == 127);
      NSImage *preview = [[NSImage alloc] initWithSize:size];
      [preview addRepresentation:rep];
      NSMutableArray<NSDraggingItem *> *items = [NSMutableArray new];
      NSMutableArray *writers = [NSMutableArray new];
      for (NSString *name in @[ @"café 空白.txt", @"second.txt" ]) {
        NSURL *file = [directory URLByAppendingPathComponent:name];
        DRAG_CHECK([text writeToURL:file options:0 error:nil]);
        const char *path = file.fileSystemRepresentation;
        item = sw_drag_item(3, (const uint8_t *)path, (int64_t)strlen(path));
        DRAG_CHECK(item);
        [writers addObject:item.item];
        [items addObject:item];
      }
      [board clearContents];
      sw_drag_item_images(items, preview, NSMakePoint(0.5, 2),
                          NSMakePoint(50, 20));
      DRAG_CHECK(
          NSEqualRects(items[0].draggingFrame, NSMakeRect(49.5, 18, 2, 3)));
      DRAG_CHECK(NSEqualSizes(items[1].draggingFrame.size, NSMakeSize(1, 1)));
      DRAG_CHECK(items[0].imageComponents.firstObject.contents == preview);
      DRAG_CHECK([board writeObjects:writers]);
      NSArray<NSURL *> *paths =
          [board readObjectsForClasses:@[ NSURL.class ]
                               options:@{
                                 NSPasteboardURLReadingFileURLsOnlyKey : @YES
                               }];
      DRAG_CHECK(paths.count == 2 && board.pasteboardItems.count == 2);
      for (NSUInteger index = 0; index < paths.count; ++index) {
        NSURL *expected = writers[index];
        DRAG_CHECK(!strcmp(paths[index].fileSystemRepresentation,
                           expected.fileSystemRepresentation));
      }
      return 0;
    } @finally {
      [board releaseGlobally];
      [NSFileManager.defaultManager removeItemAtURL:directory error:nil];
    }
  }
}

void scoop_window_test_drag_input(void *pointer, int32_t kind) {
  SWView *view = (__bridge SWView *)pointer;
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!view.owner || !view.window)
      return;
    NSEventType type =
        kind == 0 ? NSEventTypeLeftMouseDown : NSEventTypeLeftMouseDragged;
    NSPoint point = [view convertPoint:NSMakePoint(40.0 + kind * 12.0, 45.0)
                                toView:nil];
    NSEvent *event =
        [NSEvent mouseEventWithType:type
                           location:point
                      modifierFlags:0
                          timestamp:NSProcessInfo.processInfo.systemUptime
                       windowNumber:view.window.windowNumber
                            context:nil
                        eventNumber:100 + kind
                         clickCount:1
                           pressure:1];
    if (kind == 0)
      [view mouseDown:event];
    else
      [view mouseDragged:event];
  });
}
