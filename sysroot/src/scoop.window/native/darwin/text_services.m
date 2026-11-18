#include "../text_services.h"
#import "window.h"
#include <errno.h>

@interface SWCheckedText : NSObject
@property(nonatomic) int64_t start, end;
@property(nonatomic) int32_t kind;
@property(nonatomic, strong) NSData *descriptionText;
@property(nonatomic, strong) NSArray<NSData *> *suggestions;
@end
@implementation SWCheckedText
@end

static SWCheckedText *row(NSString *text, NSRange range, int32_t kind,
                          NSString *description,
                          NSArray<NSString *> *suggestions) {
  SWCheckedText *result = [SWCheckedText new];
  result.start = (int64_t)[[text substringToIndex:range.location]
      lengthOfBytesUsingEncoding:NSUTF8StringEncoding];
  result.end =
      result.start + (int64_t)[[text substringWithRange:range]
                         lengthOfBytesUsingEncoding:NSUTF8StringEncoding];
  result.kind = kind;
  result.descriptionText =
      [(description ?: @"") dataUsingEncoding:NSUTF8StringEncoding];
  NSMutableArray *values = [NSMutableArray new];
  for (NSString *suggestion in suggestions)
    [values addObject:[suggestion dataUsingEncoding:NSUTF8StringEncoding]];
  result.suggestions = values;
  return result;
}

int32_t scoop_window_text_service_flags(void) { return 31; }

SWTextServiceResult scoop_window_text_check(const uint8_t *bytes,
                                            int64_t length, int32_t flags,
                                            const uint8_t *language,
                                            int64_t language_length) {
  @autoreleasepool {
    NSInteger tag = 0;
    @try {
      NSMutableArray *result = [NSMutableArray new];
      NSString *text = [[NSString alloc] initWithBytes:bytes
                                                length:(NSUInteger)length
                                              encoding:NSUTF8StringEncoding];
      NSString *locale =
          language_length
              ? [[NSString alloc] initWithBytes:language
                                         length:(NSUInteger)language_length
                                       encoding:NSUTF8StringEncoding]
              : nil;
      if (!text || (language_length && !locale))
        return (SWTextServiceResult){NULL, EINVAL};
      NSSpellChecker *checker = NSSpellChecker.sharedSpellChecker;
      tag = NSSpellChecker.uniqueSpellDocumentTag;
      NSTextCheckingTypes types = 0;
      if (flags & 1)
        types |= NSTextCheckingTypeSpelling;
      if (flags & 2)
        types |= NSTextCheckingTypeGrammar;
      if (flags & 4) {
        if (NSSpellChecker.automaticSpellingCorrectionEnabled)
          types |= NSTextCheckingTypeCorrection;
        if (NSSpellChecker.automaticQuoteSubstitutionEnabled)
          types |= NSTextCheckingTypeQuote;
        if (NSSpellChecker.automaticDashSubstitutionEnabled)
          types |= NSTextCheckingTypeDash;
        if (NSSpellChecker.automaticTextReplacementEnabled)
          types |= NSTextCheckingTypeReplacement;
      }
      NSDictionary *options = locale ? @{
        NSTextCheckingOrthographyKey :
            [NSOrthography defaultOrthographyForLanguage:locale]
      }
                                     : nil;
      for (NSTextCheckingResult *finding in
           [checker checkString:text
                                range:NSMakeRange(0, text.length)
                                types:types
                              options:options
               inSpellDocumentWithTag:tag
                          orthography:NULL
                            wordCount:NULL]) {
        if (!sw_text_range(text, finding.range))
          continue;
        if (finding.resultType == NSTextCheckingTypeSpelling) {
          [result addObject:row(text, finding.range, 1, @"",
                                [checker guessesForWordRange:finding.range
                                                    inString:text
                                                    language:locale
                                      inSpellDocumentWithTag:tag])];
        } else if (finding.resultType == NSTextCheckingTypeGrammar) {
          for (NSDictionary *detail in finding.grammarDetails) {
            NSRange range = [detail[NSGrammarRange] rangeValue];
            if (range.location > finding.range.length ||
                range.length > finding.range.length - range.location)
              continue;
            range.location += finding.range.location;
            if (sw_text_range(text, range))
              [result addObject:row(text, range, 2,
                                    detail[NSGrammarUserDescription],
                                    detail[NSGrammarCorrections])];
          }
        } else if (finding.replacementString) {
          [result addObject:row(text, finding.range, 3, @"",
                                @[ finding.replacementString ])];
        }
      }
      return (SWTextServiceResult){(__bridge_retained void *)result, 0};
    } @catch (NSException *exception) {
      return (SWTextServiceResult){NULL, EIO};
    } @finally {
      if (tag)
        [NSSpellChecker.sharedSpellChecker closeSpellDocumentWithTag:tag];
    }
  }
}

int64_t scoop_window_text_service_count(void *pointer) {
  return (int64_t)((__bridge NSArray *)pointer).count;
}
SWTextServiceItem scoop_window_text_service_item(void *pointer, int64_t index) {
  SWCheckedText *value = ((__bridge NSArray *)pointer)[(NSUInteger)index];
  return (SWTextServiceItem){value.start, value.end, value.kind,
                             (int64_t)value.suggestions.count};
}
SWTextServiceString scoop_window_text_service_string(void *pointer,
                                                     int64_t index,
                                                     int64_t suggestion) {
  SWCheckedText *value = ((__bridge NSArray *)pointer)[(NSUInteger)index];
  NSData *data = suggestion < 0 ? value.descriptionText
                                : value.suggestions[(NSUInteger)suggestion];
  return (SWTextServiceString){data.length ? data.bytes : (const uint8_t *)"",
                               (int64_t)data.length};
}
void scoop_window_text_service_destroy(void *pointer) {
  (void)(__bridge_transfer NSArray *)pointer;
}

SWTextServiceOutput scoop_window_text_service_perform(const uint8_t *name,
                                                      int64_t name_length,
                                                      const uint8_t *bytes,
                                                      int64_t length) {
  @autoreleasepool {
    NSPasteboard *pasteboard = nil;
    @try {
      NSString *service =
          [[NSString alloc] initWithBytes:name
                                   length:(NSUInteger)name_length
                                 encoding:NSUTF8StringEncoding];
      NSString *text = [[NSString alloc] initWithBytes:bytes
                                                length:(NSUInteger)length
                                              encoding:NSUTF8StringEncoding];
      if (!service || !text)
        return (SWTextServiceOutput){NULL, 0, EINVAL};
      pasteboard = NSPasteboard.pasteboardWithUniqueName;
      [pasteboard declareTypes:@[ NSPasteboardTypeString ] owner:nil];
      if (![pasteboard setString:text forType:NSPasteboardTypeString] ||
          !NSPerformService(service, pasteboard))
        return (SWTextServiceOutput){0};
      NSData *data = [[pasteboard stringForType:NSPasteboardTypeString]
          dataUsingEncoding:NSUTF8StringEncoding];
      if (!data)
        return (SWTextServiceOutput){0};
      uint8_t *copy = malloc(MAX(data.length, 1ul));
      if (!copy)
        return (SWTextServiceOutput){NULL, 0, ENOMEM};
      if (data.length)
        memcpy(copy, data.bytes, data.length);
      return (SWTextServiceOutput){copy, (int64_t)data.length, 0};
    } @catch (NSException *exception) {
      return (SWTextServiceOutput){NULL, 0, EIO};
    } @finally {
      [pasteboard releaseGlobally];
    }
  }
}

int64_t scoop_window_text_definition(void *pointer, const uint8_t *bytes,
                                     int64_t length, double x, double y,
                                     double width, double height) {
  @autoreleasepool {
    @try {
      SWWindow *window = (__bridge SWWindow *)pointer;
      if (window.destroying || window.inputBlocked || !window.window.visible)
        return 0;
      NSString *text = [[NSString alloc] initWithBytes:bytes
                                                length:(NSUInteger)length
                                              encoding:NSUTF8StringEncoding];
      if (!text)
        return EINVAL;
      NSRect anchor = NSIntersectionRect(NSMakeRect(x, y, width, height),
                                         window.view.bounds);
      if (NSIsEmptyRect(anchor))
        return 0;
      [window.view
          showDefinitionForAttributedString:[[NSAttributedString alloc]
                                                initWithString:text]
                                    atPoint:NSMakePoint(NSMinX(anchor),
                                                        NSMaxY(anchor))];
      return 1;
    } @catch (NSException *exception) {
      return EIO;
    }
  }
}
