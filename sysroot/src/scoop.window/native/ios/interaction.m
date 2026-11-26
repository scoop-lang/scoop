#import "window.h"
#include <errno.h>

int64_t scoop_window_haptic_feedback(void *pointer, int32_t pattern) {
  SWIScene *scene = (__bridge SWIScene *)pointer;
  if (!scene.active || !scene.window.isKeyWindow)
    return 1;
  if (pattern == 1) {
    UISelectionFeedbackGenerator *generator =
        [[UISelectionFeedbackGenerator alloc] init];
    [generator selectionChanged];
  } else {
    UIImpactFeedbackStyle style =
        pattern == 2 ? UIImpactFeedbackStyleMedium : UIImpactFeedbackStyleLight;
    UIImpactFeedbackGenerator *generator =
        [[UIImpactFeedbackGenerator alloc] initWithStyle:style];
    [generator impactOccurred];
  }
  return 0;
}

int64_t scoop_window_drag_preview(void *pointer, int32_t width, int32_t height,
                                  const uint8_t *rgba, double logicalWidth,
                                  double logicalHeight, double hotX,
                                  double hotY) {
  SWIScene *scene = (__bridge SWIScene *)pointer;
  NSData *pixels = [NSData dataWithBytes:rgba
                                  length:(size_t)width * height * 4];
  CGDataProviderRef provider =
      CGDataProviderCreateWithCFData((__bridge CFDataRef)pixels);
  CGColorSpaceRef space = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
  CGImageRef image =
      CGImageCreate(width, height, 8, 32, (size_t)width * 4, space,
                    kCGImageAlphaLast | kCGBitmapByteOrder32Big, provider, NULL,
                    false, kCGRenderingIntentDefault);
  CGColorSpaceRelease(space);
  CGDataProviderRelease(provider);
  if (!image)
    return ENOMEM;
  UIImageView *preview =
      [[UIImageView alloc] initWithImage:[UIImage imageWithCGImage:image]];
  CGImageRelease(image);
  preview.frame = CGRectMake(0, 0, logicalWidth, logicalHeight);
  preview.layer.anchorPoint =
      CGPointMake(hotX / logicalWidth, hotY / logicalHeight);
  preview.userInteractionEnabled = NO;
  preview.accessibilityElementsHidden = YES;
  [scene.dragPreview removeFromSuperview];
  scene.dragPreview = preview;
  [scene.view addSubview:preview];
  return 0;
}

void scoop_window_ios_drag_position(void *pointer, double x, double y) {
  SWIScene *scene = (__bridge SWIScene *)pointer;
  scene.dragPreview.layer.position = CGPointMake(x, y);
}

void scoop_window_ios_drag_end(void *pointer) {
  SWIScene *scene = (__bridge SWIScene *)pointer;
  [scene.dragPreview removeFromSuperview];
  scene.dragPreview = nil;
}
