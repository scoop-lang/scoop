#import "probe.h"

@implementation SWDropInfo
- (void)slideDraggedImageTo:(NSPoint)point {
  self.draggedImageLocation = point;
}
- (NSArray<NSString *> *)namesOfPromisedFilesDroppedAtDestination:(NSURL *)url {
  return nil;
}
- (void)enumerateDraggingItemsWithOptions:
            (NSDraggingItemEnumerationOptions)options
                                  forView:(NSView *)view
                                  classes:(NSArray<Class> *)classes
                            searchOptions:(NSDictionary *)searchOptions
                               usingBlock:(void (^)(NSDraggingItem *, NSInteger,
                                                    BOOL *))block {
  NSArray *items =
      [self.draggingPasteboard readObjectsForClasses:classes
                                             options:searchOptions];
  BOOL stop = NO;
  NSInteger index = 0;
  for (id item in items) {
    NSDraggingItem *drag =
        [[NSDraggingItem alloc] initWithPasteboardWriter:item];
    block(drag, index++, &stop);
    if (stop)
      break;
  }
}
- (void)resetSpringLoading {
  self.springLoadingHighlight = NSSpringLoadingHighlightNone;
}
@end

@implementation SWDropData
- (void)pasteboard:(NSPasteboard *)board
                  item:(NSPasteboardItem *)item
    provideDataForType:(NSPasteboardType)type {
  self.reads += 1;
  if (self.mutation == 1 && self.probe.callback)
    self.probe.callback(self.probe.context, -1, 0);
  if (self.mutation == 2) {
    [board clearContents];
    [board setString:@"replacement" forType:NSPasteboardTypeString];
    return;
  }
  NSString *value = self.values[type];
  if (value)
    [item setString:value forType:type];
}
@end

@implementation SWDropProbe
@end

static NSURL *file(SWDropProbe *probe, NSString *name) {
  return [probe.directory URLByAppendingPathComponent:name];
}

BOOL sw_drop_test_prepare(SWDropProbe *probe, int32_t kind,
                          NSDragOperation mask) {
  [probe.info.draggingPasteboard releaseGlobally];
  NSPasteboard *board = [NSPasteboard pasteboardWithUniqueName];
  SWDropData *data = [SWDropData new];
  data.probe = probe;
  data.values =
      @{NSPasteboardTypeString : kind == 1 ? @"" : @"中文😀\r\nvalue"};
  if (kind == 2 || kind == 7)
    data.values = @{
      NSPasteboardTypeString : @"fallback",
      NSPasteboardTypeURL : kind == 7 ? @"relative/path"
                                      : @"https://example.test/path?q=1"
    };
  NSPasteboardItem *item = [NSPasteboardItem new];
  [item setDataProvider:data forTypes:data.values.allKeys];
  NSArray *items = @[ item ];
  if (kind == 3) {
    items = @[ file(probe, @"one 中文.txt"), file(probe, @"two 😀.txt") ];
  } else if (kind == 6) {
    NSPasteboardItem *second = [NSPasteboardItem new];
    [second setDataProvider:data forTypes:@[ NSPasteboardTypeString ]];
    items = @[ item, second ];
  } else if (kind == 8) {
    NSPasteboardItem *first = [NSPasteboardItem new];
    NSPasteboardItem *second = [NSPasteboardItem new];
    [first setString:file(probe, @"one 中文.txt").absoluteString
             forType:NSPasteboardTypeFileURL];
    [second setString:@"https://example.test/not-a-file"
              forType:NSPasteboardTypeFileURL];
    items = @[ first, second ];
  }
  if (![board writeObjects:items]) {
    [board releaseGlobally];
    return NO;
  }
  SWDropInfo *info = [SWDropInfo new];
  info.draggingDestinationWindow = probe.view.window;
  info.draggingLocation = [probe.view convertPoint:NSMakePoint(35, 45)
                                            toView:nil];
  info.draggingPasteboard = board;
  info.draggingSourceOperationMask = mask;
  info.draggingSequenceNumber = ++probe.sequence;
  info.draggingSource = kind == 2 ? probe : nil;
  probe.info = info;
  probe.data = data;
  return YES;
}

void *scoop_drop_test_open(void *view) {
  SWDropProbe *probe = [SWDropProbe new];
  probe.view = (__bridge NSView *)view;
  probe.directory = [[NSURL fileURLWithPath:NSTemporaryDirectory()
                                isDirectory:YES]
      URLByAppendingPathComponent:[@"scoop-drop-"
                                      stringByAppendingString:NSUUID.UUID
                                                                  .UUIDString]
                      isDirectory:YES];
  NSFileManager *manager = NSFileManager.defaultManager;
  if (![manager createDirectoryAtURL:probe.directory
          withIntermediateDirectories:NO
                           attributes:nil
                                error:NULL])
    return NULL;
  BOOL written = [[@"first" dataUsingEncoding:NSUTF8StringEncoding]
                     writeToURL:file(probe, @"one 中文.txt")
                     atomically:YES] &&
                 [[@"second" dataUsingEncoding:NSUTF8StringEncoding]
                     writeToURL:file(probe, @"two 😀.txt")
                     atomically:YES];
  if (!written) {
    [manager removeItemAtURL:probe.directory error:NULL];
    return NULL;
  }
  probe.lastStep = -1;
  return (__bridge_retained void *)probe;
}

void scoop_drop_test_rebind(void *pointer, void *view) {
  SWDropProbe *probe = (__bridge SWDropProbe *)pointer;
  probe.oldView = probe.view;
  probe.view = (__bridge NSView *)view;
}

int32_t scoop_drop_test_reenter(void *pointer) {
  SWDropProbe *probe = (__bridge SWDropProbe *)pointer;
  CHECK([probe.view draggingEntered:probe.info] == NSDragOperationNone);
  CHECK(![probe.view prepareForDragOperation:probe.info]);
  CHECK(![probe.view performDragOperation:probe.info]);
  [probe.view draggingExited:probe.info];
  return 0;
}

int32_t scoop_drop_test_last(void *pointer, int32_t step) {
  SWDropProbe *probe = (__bridge SWDropProbe *)pointer;
  CHECK(probe.lastStep == step);
  return probe.lastResult;
}

void scoop_drop_test_close(void *pointer) {
  SWDropProbe *probe = (__bridge_transfer SWDropProbe *)pointer;
  probe.closed = YES;
  probe.callback = NULL;
  probe.context = NULL;
  [probe.info.draggingPasteboard releaseGlobally];
  [NSFileManager.defaultManager removeItemAtURL:probe.directory error:NULL];
}
