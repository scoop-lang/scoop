#import <AppKit/AppKit.h>
#import <QuartzCore/QuartzCore.h>

/* Read the native event view's published snapshot, not the Surface candidate.
 */
typedef struct {
  double x, y, width, height;
  int32_t role;
} NativeHitRect;

int32_t scoop_test_ui_region(void *window_pointer, void *view_pointer,
                             int32_t field, double x, double y) {
  NSView *view = (__bridge NSView *)view_pointer;
  if (field == 2)
    return [view.layer isKindOfClass:CAMetalLayer.class] && view.layer.opaque;
  if (field != 0)
    return -1;
  id owner = [view valueForKey:@"owner"];
  NSData *data = [owner valueForKey:@"hitRegions"];
  const NativeHitRect *rects = data.bytes;
  for (NSUInteger index = 0; index < data.length / sizeof(*rects); ++index) {
    NativeHitRect r = rects[index];
    if (x >= r.x && y >= r.y && x < r.x + r.width && y < r.y + r.height)
      return r.role;
  }
  return 0;
}
