#include "../desktop.h"
#import <AppKit/AppKit.h>
#import <CoreFoundation/CoreFoundation.h>

extern int64_t scoop_window_application_dock_menu(void *menu);

@interface SDApplication : NSObject
@property(nonatomic) SDCallback callback;
@property(nonatomic) void *context;
@property(nonatomic) BOOL iconSet, badgeSet, destroyed, finishScheduled;
@property(nonatomic) NSUInteger tracking;
@property(nonatomic, strong) NSMenu *menu, *trackedMenu;
@property(nonatomic, strong) id beginObserver, endObserver;
@end
@implementation SDApplication
@end

@interface SDWindow : NSObject
@property(nonatomic, strong) NSWindow *window;
@property(nonatomic) BOOL iconSet, captionSet;
@end
@implementation SDWindow
@end

static NSString *sd_string(const uint8_t *text, int64_t length) {
  return [[NSString alloc] initWithBytes:text
                                  length:(NSUInteger)length
                                encoding:NSUTF8StringEncoding];
}

static NSImage *sd_image(SDImage *source) {
  if (!source)
    return nil;
  NSSize size = NSMakeSize(source->width, source->height);
  NSImage *image = [[NSImage alloc] initWithSize:size];
  image.template = source->is_template != 0;
  for (SDPixels *pixels = source->pixels; pixels; pixels = pixels->next) {
    NSBitmapImageRep *rep = [[NSBitmapImageRep alloc]
        initWithBitmapDataPlanes:NULL
                      pixelsWide:pixels->width
                      pixelsHigh:pixels->height
                   bitsPerSample:8
                 samplesPerPixel:4
                        hasAlpha:YES
                        isPlanar:NO
                  colorSpaceName:NSDeviceRGBColorSpace
                    bitmapFormat:NSBitmapFormatAlphaNonpremultiplied
                     bytesPerRow:(NSInteger)pixels->width * 4
                    bitsPerPixel:32];
    if (!rep)
      return nil;
    memcpy(rep.bitmapData, pixels->bytes,
           (size_t)pixels->width * pixels->height * 4);
    rep.size = size;
    [image addRepresentation:rep];
  }
  return image;
}

static BOOL belongs_to(NSMenu *menu, NSMenu *root) {
  if (!root)
    return NO;
  for (NSMenu *current = menu; current; current = current.supermenu)
    if (current == root)
      return YES;
  return NO;
}

SDResult scoop_desktop_application_create(SDCallback callback, void *context,
                                          const uint8_t *id, int64_t length) {
  SDApplication *owner = [SDApplication new];
  owner.callback = callback;
  owner.context = context;
  __weak SDApplication *weak = owner;
  owner.beginObserver = [NSNotificationCenter.defaultCenter
      addObserverForName:NSMenuDidBeginTrackingNotification
                  object:nil
                   queue:nil
              usingBlock:^(NSNotification *notification) {
                SDApplication *app = weak;
                if (!app || !belongs_to(notification.object, app.menu))
                  return;
                app.trackedMenu = app.menu;
                app.tracking += 1;
              }];
  owner.endObserver = [NSNotificationCenter.defaultCenter
      addObserverForName:NSMenuDidEndTrackingNotification
                  object:nil
                   queue:nil
              usingBlock:^(NSNotification *notification) {
                SDApplication *app = weak;
                if (!app || !belongs_to(notification.object, app.trackedMenu))
                  return;
                if (app.tracking)
                  app.tracking -= 1;
                if (app.tracking || app.finishScheduled)
                  return;
                app.finishScheduled = YES;
                // Retire after AppKit has unwound its automatic Dock tracking
                // loop.
                CFRunLoopPerformBlock(CFRunLoopGetMain(), kCFRunLoopDefaultMode,
                                      ^{
                                        app.finishScheduled = NO;
                                        if (!app.tracking) {
                                          app.trackedMenu = nil;
                                          if (!app.destroyed)
                                            app.callback(app.context, 0);
                                        }
                                      });
                CFRunLoopWakeUp(CFRunLoopGetMain());
              }];
  return (SDResult){(__bridge_retained void *)owner, 0};
}

int32_t scoop_desktop_application_capabilities(void *pointer) {
  return NSApp.activationPolicy == NSApplicationActivationPolicyRegular ? 7 : 0;
}
int32_t scoop_desktop_application_busy(void *pointer) {
  SDApplication *app = (__bridge SDApplication *)pointer;
  return app.tracking || app.finishScheduled;
}
void scoop_desktop_application_cancel(void *pointer) {
  SDApplication *app = (__bridge SDApplication *)pointer;
  scoop_window_application_dock_menu(NULL);
  [app.trackedMenu cancelTracking];
}
void scoop_desktop_application_destroy(void *pointer) {
  SDApplication *app = (__bridge_transfer SDApplication *)pointer;
  app.destroyed = YES;
  scoop_window_application_dock_menu(NULL);
  [NSNotificationCenter.defaultCenter removeObserver:app.beginObserver];
  [NSNotificationCenter.defaultCenter removeObserver:app.endObserver];
  if (app.iconSet)
    NSApp.applicationIconImage = nil;
  if (app.badgeSet)
    NSApp.dockTile.badgeLabel = nil;
  app.menu = nil;
  app.trackedMenu = nil;
}
int64_t scoop_desktop_application_icon(void *pointer, SDImage *source) {
  if (!(scoop_desktop_application_capabilities(pointer) & 1))
    return SD_UNSUPPORTED;
  NSImage *image = sd_image(source);
  if (source && !image)
    return SD_FAILURE;
  NSApp.applicationIconImage = image;
  ((__bridge SDApplication *)pointer).iconSet = YES;
  return 0;
}
int64_t scoop_desktop_application_badge(void *pointer, const uint8_t *text,
                                        int64_t length) {
  if (!(scoop_desktop_application_capabilities(pointer) & 2))
    return SD_UNSUPPORTED;
  NSString *value = sd_string(text, length);
  if (!value)
    return SD_INVALID;
  if (![NSApp.dockTile.badgeLabel isEqualToString:value])
    NSApp.dockTile.badgeLabel = value.length ? value : nil;
  ((__bridge SDApplication *)pointer).badgeSet = YES;
  return 0;
}
int64_t scoop_desktop_application_menu(void *pointer, void *menu) {
  SDApplication *app = (__bridge SDApplication *)pointer;
  if (menu && !(scoop_desktop_application_capabilities(pointer) & 4))
    return SD_UNSUPPORTED;
  if (scoop_desktop_application_busy(pointer))
    return SD_BUSY;
  int64_t error = scoop_window_application_dock_menu(menu);
  if (!error)
    app.menu = (__bridge NSMenu *)menu;
  return error;
}
SDResult scoop_desktop_window_create(void *application, void *window) {
  SDWindow *owner = [SDWindow new];
  owner.window = (__bridge NSWindow *)window;
  return (SDResult){(__bridge_retained void *)owner, 0};
}
int32_t scoop_desktop_window_capabilities(void *pointer) { return 1 | 8; }
void scoop_desktop_window_destroy(void *pointer) {
  SDWindow *owner = (__bridge_transfer SDWindow *)pointer;
  if (owner.iconSet)
    owner.window.miniwindowImage = nil;
  if (owner.captionSet)
    owner.window.miniwindowTitle = owner.window.title;
}
int64_t scoop_desktop_window_icon(void *pointer, SDImage *source) {
  NSImage *image = sd_image(source);
  if (source && !image)
    return SD_FAILURE;
  SDWindow *owner = (__bridge SDWindow *)pointer;
  owner.window.miniwindowImage = image;
  owner.iconSet = YES;
  return 0;
}
int64_t scoop_desktop_window_caption(void *pointer, int32_t thumbnail,
                                     const uint8_t *text, int64_t length) {
  if (thumbnail)
    return SD_UNSUPPORTED;
  NSString *value = sd_string(text, length);
  if (!value)
    return SD_INVALID;
  SDWindow *owner = (__bridge SDWindow *)pointer;
  NSString *title = value.length ? value : owner.window.title;
  if (![owner.window.miniwindowTitle isEqualToString:title])
    owner.window.miniwindowTitle = title;
  owner.captionSet = YES;
  return 0;
}
int64_t scoop_desktop_window_overlay(void *window, SDImage *image,
                                     const uint8_t *text, int64_t length) {
  return SD_UNSUPPORTED;
}
int64_t scoop_desktop_window_progress(void *window, int32_t state,
                                      uint64_t completed, uint64_t total) {
  return SD_UNSUPPORTED;
}
