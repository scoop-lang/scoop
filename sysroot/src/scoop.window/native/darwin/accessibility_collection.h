#ifndef SCOOP_WINDOW_DARWIN_ACCESSIBILITY_COLLECTION_H
#define SCOOP_WINDOW_DARWIN_ACCESSIBILITY_COLLECTION_H

#import "accessibility.h"

@interface SWAccessibilityItemData : NSObject
@property(nonatomic, copy) id key, parentKey;
@property(nonatomic, strong) NSData *search;
@property(nonatomic, copy) NSString *name, *help, *identifier, *value;
@property(nonatomic) int64_t index, count, span, hierarchyLevel, siblingIndex,
    siblingCount;
@property(nonatomic) int32_t enabled, selected, expanded, focusable, hasCells,
    checked, busy;
@end

@interface SWAccessibilityColumnData : NSObject
@property(nonatomic, copy) id key;
@property(nonatomic, copy) NSString *name;
@property(nonatomic) int64_t header;
@end

@interface SWAccessibilityVirtualItem : NSAccessibilityElement
@property(nonatomic, weak) SWAccessibilityCollection *collection;
@property(nonatomic, copy) id key;
@property(nonatomic) BOOL retired;
@property(nonatomic) uint64_t revision;
@property(nonatomic, strong) SWAccessibilityItemData *cached;
@property(nonatomic, strong) NSMapTable *cells;
- (SWAccessibilityItemData *)itemData;
@end

enum { SWAccessibilityListPageSize = 100 };

@interface SWAccessibilityListPageGroup : NSAccessibilityElement
@property(nonatomic, weak) SWAccessibilityCollection *collection;
@property(nonatomic, readonly) NSRange range;
- (void)updateViewport;
- (void)retire;
- (BOOL)containsVisibleKey:(id)key;
@end

BOOL sw_accessibility_list_needs_pages(SWAccessibilityState *state);

@interface SWAccessibilityCollection ()
@property(nonatomic, strong)
    NSMapTable<id, SWAccessibilityVirtualItem *> *items;
@property(nonatomic, strong) SWAccessibilityListPageGroup *pageGroup;
- (NSRange)presentedRange;
- (NSArray *)presentedRowsFrom:(NSUInteger)index count:(NSUInteger)count;
- (NSInteger)publishedIndexForKey:(id)key;
- (SWAccessibilityState *)state;
- (SWAccessibilityNode *)mountedRow:(id)key;
- (int32_t)query:(int32_t)operation
           index:(int64_t)index
             key:(id)key
           reply:(SWAccessibilityItemData *)reply;
@end

@interface SWAccessibilityVirtualItem (Grid)
- (BOOL)hasGridCells;
- (NSArray *)gridCells;
- (id)cellForKey:(id)key;
- (void)refreshCells;
@end

NSArray *sw_accessibility_table_rows(SWAccessibilityNode *node, BOOL visible);
NSArray *sw_accessibility_table_headers(SWAccessibilityNode *node, BOOL row);
id sw_accessibility_virtual_cell(SWAccessibilityTree *tree, int64_t number);

#endif
