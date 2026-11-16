#ifndef SCOOP_WINDOW_DARWIN_ACCESSIBILITY_H
#define SCOOP_WINDOW_DARWIN_ACCESSIBILITY_H

#import "window.h"

typedef int32_t (*SWAccessibilityCallback)(void *, int64_t, int32_t,
                                           const uint8_t *, int64_t);

typedef struct {
  int64_t number, parent, order;
  int32_t role, flags, actions, checked, geometry, visible;
  double x, y, width, height;
  double visible_x, visible_y, visible_width, visible_height;
} SWAccessibilityData;

enum {
  SWAXEnabled = 1,
  SWAXReadOnly = 2,
  SWAXProtected = 4,
  SWAXHidden = 8,
  SWAXNamed = 16
};

@class SWAccessibilityTree;

@interface SWAccessibilityState : NSObject
@property(nonatomic) SWAccessibilityData data;
@property(nonatomic, copy) NSString *name, *help, *error, *identifier, *value;
@property(nonatomic, strong) NSMutableArray<NSNumber *> *labels, *controls,
    *titleFor;
@end

@interface SWAccessibilityNode : NSAccessibilityElement
@property(nonatomic, weak) SWAccessibilityTree *tree;
@property(nonatomic) int64_t number;
@property(nonatomic) BOOL retired;
- (SWAccessibilityState *)state;
@end

@interface SWAccessibilityTree : NSObject
@property(nonatomic, weak) SWView *view;
@property(nonatomic) SWAccessibilityCallback callback;
@property(nonatomic) void *context;
@property(nonatomic) BOOL closed;
@property(nonatomic) int64_t focused;
@property(nonatomic, strong) SWAccessibilityNode *root;
@property(nonatomic, strong) SWAccessibilityState *rootState;
@property(nonatomic, strong)
    NSDictionary<NSNumber *, SWAccessibilityState *> *published;
@property(nonatomic, strong)
    NSMutableDictionary<NSNumber *, SWAccessibilityState *> *pending;
@property(nonatomic, strong) NSMutableSet<NSNumber *> *removed;
@property(nonatomic, strong) NSMutableSet<NSNumber *> *retiring;
@property(nonatomic, strong)
    NSMutableDictionary<NSNumber *, SWAccessibilityNode *> *elements;
- (SWAccessibilityNode *)element:(int64_t)number;
@end

id sw_accessibility_focused(SWView *view);

#endif
