#ifndef SCOOP_WINDOW_DARWIN_ACCESSIBILITY_H
#define SCOOP_WINDOW_DARWIN_ACCESSIBILITY_H

#import "window.h"

typedef int32_t (*SWAccessibilityCallback)(void *, int64_t, int32_t,
                                           const uint8_t *, int64_t, int64_t,
                                           int64_t, uint64_t, double);
typedef int32_t (*SWAccessibilityCollectionCallback)(void *, int64_t, int32_t,
                                                     int64_t, int32_t, int64_t,
                                                     const uint8_t *, int64_t,
                                                     void *);

typedef struct {
  int64_t number, parent, order, scene_order;
  uint64_t content_revision;
  int64_t anchor, cursor;
  int32_t role, flags, actions, checked, geometry, visible, scroll_orientation;
  double x, y, width, height;
  double visible_x, visible_y, visible_width, visible_height;
  double scroll_offset, scroll_viewport, scroll_content;
} SWAccessibilityData;

typedef struct {
  void *context;
  void (*retain)(void *);
  void (*release)(void *);
  int32_t (*contains)(void *, double, double);
} SWAccessibilityHitData;

@interface SWAccessibilityHitGeometry : NSObject
- (instancetype)initWithValue:(SWAccessibilityHitData)value;
- (BOOL)containsPoint:(NSPoint)point;
@end

typedef struct {
  int64_t start, end;
  int32_t status;
} SWAccessibilityTextRange;

typedef void (*SWAccessibilityTextBoxVisitor)(void *, double, double, double,
                                              double);

typedef struct {
  void *context;
  void (*retain)(void *);
  void (*release)(void *);
  SWAccessibilityTextRange (*range)(void *, int64_t, int32_t, int32_t);
  SWAccessibilityTextRange (*line_range)(void *, int64_t);
  int64_t (*line_index)(void *, int64_t, int32_t);
  SWAccessibilityTextRange (*range_at_point)(void *, double, double);
  int32_t (*boxes)(void *, int64_t, int64_t, void *,
                   SWAccessibilityTextBoxVisitor);
  SWAccessibilityTextRange (*visible_range)(void *);
} SWAccessibilityTextData;

@interface SWAccessibilityTextGeometry : NSObject
@property(nonatomic, readonly) SWAccessibilityTextData value;
- (instancetype)initWithValue:(SWAccessibilityTextData)value;
@end

enum {
  SWAXEnabled = 1,
  SWAXReadOnly = 2,
  SWAXProtected = 4,
  SWAXHidden = 8,
  SWAXNamed = 16,
  SWAXIgnored = 32,
  SWAXRequired = 64,
  SWAXCursorUpstream = 128
};

@class SWAccessibilityTree;
@class SWAccessibilityCollection;

@interface SWAccessibilityState : NSObject
@property(nonatomic) SWAccessibilityData data;
@property(nonatomic) NSRange selection;
@property(nonatomic, copy) NSString *name, *help, *error, *identifier, *value;
@property(nonatomic, strong) NSMutableArray<NSNumber *> *labels, *controls,
    *titleFor;
@property(nonatomic, strong) SWAccessibilityHitGeometry *hit;
@property(nonatomic, strong) SWAccessibilityTextGeometry *textGeometry;
@property(nonatomic) int64_t itemCount;
@property(nonatomic) uint64_t collectionRevision;
@property(nonatomic) int32_t collectionActions;
@property(nonatomic, copy) id itemKey;
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

#endif
