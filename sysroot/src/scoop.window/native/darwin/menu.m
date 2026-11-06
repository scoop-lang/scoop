#import "menu.h"

@implementation SWMenuEntry
@end

int32_t scoop_window_menu_capabilities(void) { return 1 | 4 | 8; }

SWResult scoop_window_menu_create(void *application, int64_t identity) {
  @autoreleasepool {
    @try {
      SWMenu *owner = [SWMenu new];
      owner.application = (__bridge SWApplication *)application;
      owner.identity = identity;
      owner.binding = -1;
      owner.menu = [[NSMenu alloc] initWithTitle:@""];
      owner.menu.delegate = owner;
      owner.entries = [NSMutableDictionary new];
      return (SWResult){(__bridge_retained void *)owner, 0};
    } @catch (NSException *exception) {
      return (SWResult){NULL, 1};
    }
  }
}

void scoop_window_menu_destroy(void *pointer) {
  SWMenu *owner = (__bridge_transfer SWMenu *)pointer;
  owner.menu.delegate = nil;
  [owner.menu removeAllItems];
  [owner.entries removeAllObjects];
}

int64_t scoop_window_menu_clear(void *pointer) {
  SWMenu *owner = (__bridge SWMenu *)pointer;
  @try {
    [owner.menu removeAllItems];
    [owner.entries removeAllObjects];
    return 0;
  } @catch (NSException *exception) {
    return 1;
  }
}

int64_t scoop_window_menu_append(void *pointer, int64_t identity,
                                 int64_t parent, int32_t kind, int32_t checked,
                                 int32_t enabled, int32_t role,
                                 const uint8_t *text, int64_t length,
                                 int32_t key, int32_t modifiers) {
  @autoreleasepool {
    @try {
      SWMenu *owner = (__bridge SWMenu *)pointer;
      NSMenu *container =
          parent ? owner.entries[@(parent)].item.submenu : owner.menu;
      NSString *title = [[NSString alloc] initWithBytes:text
                                                 length:(NSUInteger)length
                                               encoding:NSUTF8StringEncoding];
      if (!container || !title)
        return 1;
      NSMenuItem *item = kind == 4 ? [NSMenuItem separatorItem]
                                   : [[NSMenuItem alloc] initWithTitle:title
                                                                action:nil
                                                         keyEquivalent:@""];
      SWMenuEntry *entry = [SWMenuEntry new];
      entry.item = item;
      entry.identity = identity;
      entry.kind = kind;
      entry.role = role;
      entry.enabled = enabled != 0;
      entry.live = YES;
      item.representedObject = entry;
      item.enabled = enabled != 0;
      item.state = checked ? NSControlStateValueOn : NSControlStateValueOff;
      if (kind == 2)
        item.onStateImage = [NSImage imageWithSystemSymbolName:@"circle.fill"
                                      accessibilityDescription:nil];
      if (kind == 3) {
        item.submenu = [[NSMenu alloc] initWithTitle:title];
        item.submenu.delegate = owner;
      } else if (kind != 4) {
        item.target = owner;
        item.action = @selector(activate:);
        item.keyEquivalent = sw_menu_key_string(key);
        item.keyEquivalentModifierMask = sw_menu_modifiers(modifiers);
      }
      [container addItem:item];
      owner.entries[@(identity)] = entry;
      return 0;
    } @catch (NSException *exception) {
      return 1;
    }
  }
}

int64_t scoop_window_menu_image(void *pointer, int64_t identity, int32_t width,
                                int32_t height, const uint8_t *pixels,
                                double logical_width, double logical_height,
                                int32_t template_image) {
  @autoreleasepool {
    @try {
      SWMenu *owner = (__bridge SWMenu *)pointer;
      NSMenuItem *item = owner.entries[@(identity)].item;
      if (!item)
        return 1;
      NSBitmapImageRep *bitmap = [[NSBitmapImageRep alloc]
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
      if (!bitmap)
        return 1;
      memcpy(bitmap.bitmapData, pixels, (size_t)width * (size_t)height * 4);
      bitmap.size = NSMakeSize(logical_width, logical_height);
      if (!item.image)
        item.image = [[NSImage alloc] initWithSize:bitmap.size];
      [item.image addRepresentation:bitmap];
      item.image.template = template_image != 0;
      return 0;
    } @catch (NSException *exception) {
      return 1;
    }
  }
}

int64_t scoop_window_menu_commit(void *pointer) {
  SWMenu *owner = (__bridge SWMenu *)pointer;
  @try {
    [owner.menu update];
    return 0;
  } @catch (NSException *exception) {
    return 1;
  }
}

void scoop_window_menu_live(void *pointer, int64_t identity, int32_t enabled) {
  SWMenuEntry *entry = ((__bridge SWMenu *)pointer).entries[@(identity)];
  if (enabled < 0)
    entry.live = NO;
  else
    entry.enabled = enabled != 0;
  entry.item.enabled = entry.live && entry.enabled;
}

void *scoop_window_menu_handle(void *pointer) {
  return (__bridge void *)((__bridge SWMenu *)pointer).menu;
}

void *scoop_window_menu_actions(void *pointer) { return NULL; }

int64_t scoop_window_menu_item_for_command(void *pointer, int32_t command) {
  return 0;
}
