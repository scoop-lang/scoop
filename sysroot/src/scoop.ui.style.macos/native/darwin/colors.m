#import <AppKit/AppKit.h>
#include <math.h>
#include <stdbool.h>
#include <stdint.h>

typedef struct {
  double red, green, blue, alpha;
} SystemColor;

typedef struct {
  int32_t status;
  SystemColor background, label, content_background, control, control_text;
  SystemColor secondary_label, accent, selected_text, selection;
  SystemColor unemphasized_selection, separator, focus, disabled;
  SystemColor red, blue, green, orange, purple, teal, shadow;
} SystemColors;

static SystemColor copy_color(NSColor *color, int32_t *status) {
  NSColor *rgb = [color colorUsingColorSpace:NSColorSpace.sRGBColorSpace];
  if (!rgb) {
    *status = 0;
    return (SystemColor){0};
  }
  CGFloat red, green, blue, alpha;
  [rgb getRed:&red green:&green blue:&blue alpha:&alpha];
  if (!isfinite(red) || !isfinite(green) || !isfinite(blue) ||
      !isfinite(alpha)) {
    *status = 0;
    return (SystemColor){0};
  }
  return (SystemColor){fmin(1, fmax(0, red)), fmin(1, fmax(0, green)),
                       fmin(1, fmax(0, blue)), fmin(1, fmax(0, alpha))};
}

SystemColors scoop_ui_macos_system_colors(bool dark) {
  @autoreleasepool {
    __block SystemColors colors = {0};
    if (!NSThread.isMainThread) {
      colors.status = -1;
      return colors;
    }
    @try {
      NSAppearance *appearance =
          [NSAppearance appearanceNamed:dark ? NSAppearanceNameDarkAqua
                                             : NSAppearanceNameAqua];
      if (!appearance)
        return colors;
      [appearance performAsCurrentDrawingAppearance:^{
        colors.status = 1;
        colors.background =
            copy_color(NSColor.windowBackgroundColor, &colors.status);
        colors.label = copy_color(NSColor.labelColor, &colors.status);
        colors.content_background =
            copy_color(NSColor.controlBackgroundColor, &colors.status);
        colors.control = copy_color(NSColor.controlColor, &colors.status);
        colors.control_text =
            copy_color(NSColor.controlTextColor, &colors.status);
        colors.secondary_label =
            copy_color(NSColor.secondaryLabelColor, &colors.status);
        colors.accent = copy_color(NSColor.controlAccentColor, &colors.status);
        colors.selected_text = copy_color(
            NSColor.alternateSelectedControlTextColor, &colors.status);
        colors.selection =
            copy_color(NSColor.selectedContentBackgroundColor, &colors.status);
        colors.unemphasized_selection = copy_color(
            NSColor.unemphasizedSelectedContentBackgroundColor, &colors.status);
        colors.separator = copy_color(NSColor.separatorColor, &colors.status);
        colors.focus =
            copy_color(NSColor.keyboardFocusIndicatorColor, &colors.status);
        colors.disabled =
            copy_color(NSColor.disabledControlTextColor, &colors.status);
        colors.red = copy_color(NSColor.systemRedColor, &colors.status);
        colors.blue = copy_color(NSColor.systemBlueColor, &colors.status);
        colors.green = copy_color(NSColor.systemGreenColor, &colors.status);
        colors.orange = copy_color(NSColor.systemOrangeColor, &colors.status);
        colors.purple = copy_color(NSColor.systemPurpleColor, &colors.status);
        colors.teal = copy_color(NSColor.systemTealColor, &colors.status);
        colors.shadow = copy_color(NSColor.shadowColor, &colors.status);
      }];
    } @catch (NSException *exception) {
      (void)exception;
      colors.status = 0;
    }
    return colors;
  }
}
