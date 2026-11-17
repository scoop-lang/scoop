#ifndef SCOOP_WINDOW_DARWIN_ACCESSIBILITY_H
#define SCOOP_WINDOW_DARWIN_ACCESSIBILITY_H

#include "../accessibility_text_document.h"
#import "window.h"

#include "../accessibility_abi.h"

@interface SWAccessibilityHitGeometry : NSObject
- (instancetype)initWithValue:(SWAccessibilityHitData)value;
- (BOOL)containsPoint:(NSPoint)point;
@end

@interface SWAccessibilityTextGeometry : NSObject
@property(nonatomic, readonly) SWAccessibilityTextData value;
- (instancetype)initWithValue:(SWAccessibilityTextData)value;
@end

@class SWAccessibilityTree;
@class SWAccessibilityCollection;
@class SWAccessibilityColumnData;

@interface SWAccessibilityState : NSObject
@property(nonatomic) SWAccessibilityData data;
@property(nonatomic) NSRange selection;
@property(nonatomic, copy) NSString *name, *help, *error, *identifier, *value,
    *url;
@property(nonatomic, strong) NSMutableArray<NSNumber *> *labels, *controls,
    *titleFor, *descriptions, *errors, *rowHeaders, *columnHeaders;
@property(nonatomic) int64_t activeNode, activeCollection;
@property(nonatomic, strong) NSMutableArray<NSDictionary *> *textChildren;
@property(nonatomic, copy) id activeKey;
@property(nonatomic, strong) SWAccessibilityHitGeometry *hit;
@property(nonatomic, strong) SWAccessibilityTextGeometry *textGeometry;
@property(nonatomic, readonly) SWTextDocument *textDocument;
@property(nonatomic, readonly) uint64_t documentRevision, documentEpoch;
- (void)setTextDocument:(SWTextDocument *)document
               revision:(uint64_t)revision
                  epoch:(uint64_t)epoch;
@property(nonatomic) int64_t itemCount;
@property(nonatomic) uint64_t collectionRevision;
@property(nonatomic) int32_t collectionActions;
@property(nonatomic, copy) id itemKey;
@property(nonatomic, copy) id cellColumnKey;
@property(nonatomic) int32_t selectionKind;
@property(nonatomic) BOOL selectionRequired;
@property(nonatomic, strong) NSMutableArray *selectedKeys;
@property(nonatomic, strong)
    NSMutableArray<SWAccessibilityColumnData *> *gridColumns;
@end

@interface SWAccessibilityNode : NSAccessibilityElement
@property(nonatomic, weak) SWAccessibilityTree *tree;
@property(nonatomic) int64_t number;
@property(nonatomic) BOOL retired;
- (SWAccessibilityState *)state;
- (BOOL)participatesInAccessibility;
- (void)appendChildrenOf:(int64_t)parent to:(NSMutableArray *)children;
- (int32_t)perform:(int32_t)action text:(NSString *)text range:(NSRange)range;
- (int32_t)perform:(int32_t)action
              text:(NSString *)text
             range:(NSRange)range
          position:(double)position;
@end

@interface SWAccessibilityNode (Text)
- (NSString *)accessibleText;
@end

@interface SWAccessibilityNode (TextGeometry)
- (SWAccessibilityTextGeometry *)accessibleTextGeometry;
@end

@interface SWAccessibilityTree : NSObject
@property(nonatomic, weak) SWView *view;
@property(nonatomic) SWAccessibilityCallback callback;
@property(nonatomic) void *context;
@property(nonatomic) SWAccessibilityCollectionCallback collectionCallback;
@property(nonatomic) void *collectionContext;
@property(nonatomic, strong)
    NSMutableDictionary<NSNumber *, SWAccessibilityCollection *> *collections;
@property(nonatomic, strong)
    NSMutableDictionary<NSNumber *, NSMapTable *> *tableColumns;
@property(nonatomic) BOOL closed;
@property(nonatomic) int64_t focused;
@property(nonatomic, strong) SWAccessibilityNode *root;
@property(nonatomic, strong) SWAccessibilityState *rootState;
@property(nonatomic, strong)
    NSDictionary<NSNumber *, SWAccessibilityState *> *published;
@property(nonatomic, strong)
    NSDictionary<NSNumber *, NSArray<NSNumber *> *> *children;
@property(nonatomic, strong) NSArray<NSNumber *> *hitOrder;
@property(nonatomic, strong)
    NSMutableDictionary<NSNumber *, SWAccessibilityState *> *pending;
@property(nonatomic, strong) NSMutableSet<NSNumber *> *removed;
@property(nonatomic, strong) NSMutableSet<NSNumber *> *retiring;
@property(nonatomic, strong)
    NSMutableDictionary<NSNumber *, SWAccessibilityNode *> *elements;
- (SWAccessibilityNode *)element:(int64_t)number;
@end

@interface SWAccessibilityTree (Collections)
- (SWAccessibilityCollection *)collection:(int64_t)number;
@end

@interface SWAccessibilityCollection : NSObject
@property(nonatomic, weak) SWAccessibilityTree *tree;
@property(nonatomic) int64_t number;
- (NSArray *)rowsFrom:(NSUInteger)index count:(NSUInteger)count;
- (NSArray *)visibleRows;
- (id)itemForKey:(id)key;
- (id)cachedItemForKey:(id)key;
- (void)refresh;
- (void)retire;
@end

BOOL sw_accessibility_collections_commit(
    SWAccessibilityTree *tree,
    NSDictionary<NSNumber *, SWAccessibilityState *> *previous,
    NSDictionary<NSNumber *, SWAccessibilityState *> *changes,
    NSSet<NSNumber *> *removed);

id sw_accessibility_focused(SWView *view);
void sw_accessibility_grid_commit(SWAccessibilityTree *tree);

#endif
