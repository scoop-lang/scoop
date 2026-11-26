#import <Foundation/Foundation.h>

int32_t scoop_window_shortcut_equal(const uint8_t *left, int64_t left_size,
                                    const uint8_t *right, int64_t right_size) {
  @autoreleasepool {
    @try {
      NSString *a = [[NSString alloc] initWithBytes:left
                                             length:(NSUInteger)left_size
                                           encoding:NSUTF8StringEncoding];
      NSString *b = [[NSString alloc] initWithBytes:right
                                             length:(NSUInteger)right_size
                                           encoding:NSUTF8StringEncoding];
      if (!a || !b)
        return -1;
      NSMutableString *folded_a = [a mutableCopy];
      NSMutableString *folded_b = [b mutableCopy];
      if (!folded_a || !folded_b)
        return -1;
      CFStringFold((__bridge CFMutableStringRef)folded_a,
                   kCFCompareCaseInsensitive, CFLocaleGetSystem());
      CFStringFold((__bridge CFMutableStringRef)folded_b,
                   kCFCompareCaseInsensitive, CFLocaleGetSystem());
      CFStringNormalize((__bridge CFMutableStringRef)folded_a,
                        kCFStringNormalizationFormC);
      CFStringNormalize((__bridge CFMutableStringRef)folded_b,
                        kCFStringNormalizationFormC);
      return CFStringCompare((__bridge CFStringRef)folded_a,
                             (__bridge CFStringRef)folded_b,
                             0) == kCFCompareEqualTo;
    } @catch (NSException *exception) {
      return -1;
    }
  }
}
