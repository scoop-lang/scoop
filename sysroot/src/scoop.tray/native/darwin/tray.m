#include "../tray.h"
#import <AppKit/AppKit.h>
#import <CoreFoundation/CoreFoundation.h>
#include <string.h>

@interface STTray : NSObject
@property(nonatomic) STCallback callback;
@property(nonatomic) void *context;
@property(nonatomic, strong) NSStatusItem *item;
@property(nonatomic, strong) NSMenu *menu, *trackedMenu;
@property(nonatomic, strong) NSImage *image, *pendingImage;
@property(nonatomic, strong) id beginObserver, endObserver;
@property(nonatomic) NSUInteger tracking;
@property(nonatomic) BOOL started, closing, presenting, finishScheduled;
- (void)activate:(id)sender;
@end

static STGeometry geometry(STTray *tray) {
  NSStatusBarButton *button = tray.item.button;
  if (!tray.started || tray.closing || !button.window || button.hidden)
    return (STGeometry){.error = -2};
  NSRect view = [button convertRect:button.bounds toView:nil];
  NSRect screen = [button.window convertRectToScreen:view];
  if (NSIsEmptyRect(screen))
    return (STGeometry){.error = -2};
  double top = NSMaxY(NSScreen.screens.firstObject.frame);
  return (STGeometry){.x = screen.origin.x,
                      .y = top - NSMaxY(screen),
                      .width = screen.size.width,
                      .height = screen.size.height};
}

static BOOL belongs_to(NSMenu *menu, NSMenu *root) {
  for (NSMenu *value = menu; value; value = value.supermenu)
    if (value == root)
      return YES;
  return NO;
}

static void emit(STTray *tray, STEvent event) {
  tray.callback(tray.context, &event);
}

@implementation STTray
- (void)activate:(id)sender {
  if (self.closing)
    return;
  NSEvent *input = NSApp.currentEvent;
  STEvent event = {.kind = 2, .action = 1};
  switch (input.type) {
  case NSEventTypeLeftMouseUp:
    event.source = 1;
    event.button = 1;
    event.clicks = (int32_t)input.clickCount;
    break;
  case NSEventTypeRightMouseUp:
    event.source = 1;
    event.action = 2;
    event.button = 2;
    event.clicks = (int32_t)input.clickCount;
    break;
  case NSEventTypeOtherMouseUp:
    event.source = 1;
    event.action = 3;
    event.button = input.buttonNumber == 2 ? 3 : 0;
    event.clicks = (int32_t)input.clickCount;
    break;
  case NSEventTypeKeyDown:
  case NSEventTypeKeyUp:
    event.source = 2;
    break;
  default:
    break;
  }
  STGeometry anchor = geometry(self);
  if (!anchor.error) {
    event.anchor = 1;
    event.x = anchor.x;
    event.y = anchor.y;
    event.width = anchor.width;
    event.height = anchor.height;
  }
  emit(self, event);
}
@end

STResult scoop_tray_create(STCallback callback, void *context,
                           int64_t identity) {
  STTray *tray = nil;
  @try {
    tray = [STTray new];
    tray.callback = callback;
    tray.context = context;
    tray.item = [NSStatusBar.systemStatusBar
        statusItemWithLength:NSSquareStatusItemLength];
    if (!tray.item)
      return (STResult){.error = -2};
    tray.item.visible = NO;
    tray.item.button.target = tray;
    tray.item.button.action = @selector(activate:);
    [tray.item.button sendActionOn:NSEventMaskLeftMouseUp |
                                   NSEventMaskRightMouseUp |
                                   NSEventMaskOtherMouseUp];
    __weak STTray *weak = tray;
    tray.beginObserver = [NSNotificationCenter.defaultCenter
        addObserverForName:NSMenuDidBeginTrackingNotification
                    object:nil
                     queue:nil
                usingBlock:^(NSNotification *notification) {
                  STTray *owner = weak;
                  if (!owner || !belongs_to(notification.object, owner.menu))
                    return;
                  owner.trackedMenu = owner.menu;
                  owner.tracking += 1;
                }];
    tray.endObserver = [NSNotificationCenter.defaultCenter
        addObserverForName:NSMenuDidEndTrackingNotification
                    object:nil
                     queue:nil
                usingBlock:^(NSNotification *notification) {
                  STTray *owner = weak;
                  if (!owner ||
                      !belongs_to(notification.object, owner.trackedMenu))
                    return;
                  if (owner.tracking)
                    owner.tracking -= 1;
                  if (owner.tracking || owner.presenting ||
                      owner.finishScheduled)
                    return;
                  owner.finishScheduled = YES;
                  // Automatic status-item tracking has no call frame owned by
                  // Scoop. Default mode runs after the native tracking loop has
                  // unwound.
                  CFRunLoopPerformBlock(CFRunLoopGetMain(),
                                        kCFRunLoopDefaultMode, ^{
                                          owner.finishScheduled = NO;
                                          if (!owner.tracking) {
                                            owner.trackedMenu = nil;
                                            emit(owner, (STEvent){.kind = 3});
                                          }
                                        });
                  CFRunLoopWakeUp(CFRunLoopGetMain());
                }];
    return (STResult){(__bridge_retained void *)tray, 0};
  } @catch (NSException *exception) {
    if (tray.beginObserver)
      [NSNotificationCenter.defaultCenter removeObserver:tray.beginObserver];
    if (tray.endObserver)
      [NSNotificationCenter.defaultCenter removeObserver:tray.endObserver];
    if (tray.item)
      [NSStatusBar.systemStatusBar removeStatusItem:tray.item];
    return (STResult){.error = 1};
  }
}

int64_t scoop_tray_start(void *pointer) {
  STTray *tray = (__bridge STTray *)pointer;
  tray.started = YES;
  tray.item.visible = YES;
  return 0;
}

int32_t scoop_tray_available(void *pointer) {
  STTray *tray = (__bridge STTray *)pointer;
  return tray.started && !tray.closing;
}

int32_t scoop_tray_capabilities(void *pointer) {
  return 1 | 2 | 4 | 8 | 32 | 64 | 128 | 256;
}

int32_t scoop_tray_busy(void *pointer) {
  STTray *tray = (__bridge STTray *)pointer;
  return tray.presenting || tray.tracking || tray.finishScheduled;
}

STGeometry scoop_tray_geometry(void *pointer) {
  return geometry((__bridge STTray *)pointer);
}

int64_t scoop_tray_image_begin(void *pointer, double width, double height,
                               int32_t template) {
  @try {
    STTray *tray = (__bridge STTray *)pointer;
    tray.pendingImage =
        [[NSImage alloc] initWithSize:NSMakeSize(width, height)];
    tray.pendingImage.template = template != 0;
    return tray.pendingImage ? 0 : 1;
  } @catch (NSException *exception) {
    return 1;
  }
}

int64_t scoop_tray_image_add(void *pointer, int32_t width, int32_t height,
                             const uint8_t *pixels) {
  @try {
    STTray *tray = (__bridge STTray *)pointer;
    NSBitmapImageRep *representation = [[NSBitmapImageRep alloc]
        initWithBitmapDataPlanes:NULL
                      pixelsWide:width
                      pixelsHigh:height
                   bitsPerSample:8
                 samplesPerPixel:4
                        hasAlpha:YES
                        isPlanar:NO
                  colorSpaceName:NSDeviceRGBColorSpace
                    bitmapFormat:NSBitmapFormatAlphaNonpremultiplied
                     bytesPerRow:(NSInteger)width * 4
                    bitsPerPixel:32];
    if (!representation)
      return 1;
    memcpy(representation.bitmapData, pixels,
           (size_t)width * (size_t)height * 4);
    representation.size = tray.pendingImage.size;
    [tray.pendingImage addRepresentation:representation];
    return 0;
  } @catch (NSException *exception) {
    return 1;
  }
}

int64_t scoop_tray_image_commit(void *pointer) {
  @try {
    STTray *tray = (__bridge STTray *)pointer;
    tray.image = tray.pendingImage;
    tray.pendingImage = nil;
    tray.item.button.image = tray.image;
    return 0;
  } @catch (NSException *exception) {
    return 1;
  }
}

int64_t scoop_tray_tooltip(void *pointer, const uint8_t *text, int64_t length) {
  @try {
    STTray *tray = (__bridge STTray *)pointer;
    tray.item.button.toolTip =
        [[NSString alloc] initWithBytes:text
                                 length:(NSUInteger)length
                               encoding:NSUTF8StringEncoding];
    return 0;
  } @catch (NSException *exception) {
    return 1;
  }
}

int64_t scoop_tray_menu(void *pointer, void *menu, void *actions) {
  @try {
    STTray *tray = (__bridge STTray *)pointer;
    tray.menu = (__bridge NSMenu *)menu;
    tray.item.menu = tray.menu;
    return 0;
  } @catch (NSException *exception) {
    return 1;
  }
}

void scoop_tray_cancel(void *pointer) {
  STTray *tray = (__bridge STTray *)pointer;
  [tray.menu cancelTrackingWithoutAnimation];
  [tray.trackedMenu cancelTrackingWithoutAnimation];
}

STSelection scoop_tray_show_menu(void *pointer) {
  STTray *tray = (__bridge STTray *)pointer;
  if (!tray.menu || geometry(tray).error)
    return (STSelection){.error = -2};
  tray.presenting = YES;
  STSelection result = {0};
  @try {
    NSStatusBarButton *button = tray.item.button;
    NSPoint point = NSMakePoint(NSMinX(button.bounds),
                                button.isFlipped ? NSMaxY(button.bounds)
                                                 : NSMinY(button.bounds));
    [tray.menu popUpMenuPositioningItem:nil atLocation:point inView:button];
  } @catch (NSException *exception) {
    result.error = 1;
  } @finally {
    tray.presenting = NO;
    tray.tracking = 0;
    tray.trackedMenu = nil;
  }
  return result;
}

void scoop_tray_request_close(void *pointer) {
  STTray *tray = (__bridge STTray *)pointer;
  tray.closing = YES;
  scoop_tray_cancel(pointer);
  tray.item.visible = NO;
}

void scoop_tray_destroy(void *pointer) {
  STTray *tray = CFBridgingRelease(pointer);
  [NSNotificationCenter.defaultCenter removeObserver:tray.beginObserver];
  [NSNotificationCenter.defaultCenter removeObserver:tray.endObserver];
  tray.item.menu = nil;
  tray.item.button.target = nil;
  [NSStatusBar.systemStatusBar removeStatusItem:tray.item];
}
