#import <AppKit/AppKit.h>
#include <stdint.h>

#define CHECK(value)                                                           \
  do {                                                                         \
    if (!(value))                                                              \
      return __LINE__;                                                         \
  } while (0)

@interface SWDropInfo : NSObject <NSDraggingInfo>
@property(nonatomic, strong) NSWindow *draggingDestinationWindow;
@property(nonatomic) NSDragOperation draggingSourceOperationMask;
@property(nonatomic) NSPoint draggingLocation, draggedImageLocation;
@property(nonatomic, strong) NSImage *draggedImage;
@property(nonatomic, strong) NSPasteboard *draggingPasteboard;
@property(nonatomic, weak) id draggingSource;
@property(nonatomic) NSInteger draggingSequenceNumber;
@property NSInteger numberOfValidItemsForDrop;
@property NSDraggingFormation draggingFormation;
@property BOOL animatesToDestination;
@property(nonatomic) NSSpringLoadingHighlight springLoadingHighlight;
@end

@class SWDropProbe;
@interface SWDropData : NSObject <NSPasteboardItemDataProvider>
@property(nonatomic, weak) SWDropProbe *probe;
@property(nonatomic, strong) NSDictionary<NSPasteboardType, NSString *> *values;
@property(nonatomic) int32_t reads, mutation;
@end

typedef int32_t (*SWDropTestCallback)(void *, int32_t, int32_t);
@interface SWDropProbe : NSObject
@property(nonatomic, strong) NSView *view, *oldView;
@property(nonatomic, strong) SWDropInfo *info;
@property(nonatomic, strong) SWDropData *data;
@property(nonatomic, strong) NSURL *directory;
@property(nonatomic) NSInteger sequence;
@property(nonatomic) BOOL closed;
@property(nonatomic) int32_t lastStep, lastResult;
@property(nonatomic) SWDropTestCallback callback;
@property(nonatomic) void *context;
@end

BOOL sw_drop_test_prepare(SWDropProbe *probe, int32_t kind,
                          NSDragOperation mask);
