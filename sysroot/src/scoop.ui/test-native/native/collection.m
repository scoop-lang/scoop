#import <AppKit/AppKit.h>

@interface ScoopCollectionProbe : NSObject
@property(nonatomic, strong) id root, list, row, retiredRow;
@property(nonatomic, weak) id exportedRow;
@property(nonatomic, strong) id gridTable, retiredCell, retiredColumn;
@property(nonatomic, weak) id gridRow, gridCell, gridColumn;
@property(nonatomic) BOOL closed;
@end
@implementation ScoopCollectionProbe
@end

static id findRole(id node, NSAccessibilityRole role) {
  if ([[node accessibilityRole] isEqual:role])
    return node;
  for (id child in [node accessibilityChildren]) {
    id found = findRole(child, role);
    if (found)
      return found;
  }
  return nil;
}

static id at(id list, NSUInteger index) {
  NSArray *rows =
      [list accessibilityArrayAttributeValues:NSAccessibilityRowsAttribute
                                        index:index
                                     maxCount:1];
  return rows.count == 1 ? rows[0] : nil;
}

#define REQUIRE(condition)                                                     \
  do {                                                                         \
    if (!(condition))                                                          \
      return __LINE__;                                                         \
  } while (0)

static int32_t checkCollection(ScoopCollectionProbe *probe, int32_t phase) {
  if (phase == 0)
    probe.list = findRole(probe.root, NSAccessibilityListRole);
  id list = probe.list;
  REQUIRE(list);
  id window = [list accessibilityWindow];
  id responder = [window firstResponder];
  switch (phase) {
  case 0: {
    REQUIRE([list accessibilityRowCount] == 100000);
    REQUIRE(
        [list accessibilityArrayAttributeCount:NSAccessibilityRowsAttribute] ==
        100000);
    REQUIRE([list accessibilityVisibleRows].count > 0);
    REQUIRE([list accessibilityVisibleRows].count < 20);
    probe.row = at(list, 50000);
    REQUIRE(probe.row);
    REQUIRE([[probe.row accessibilityLabel] isEqual:@"Row 50000"]);
    REQUIRE([[probe.row accessibilityIdentifier] isEqual:@"row-50000"]);
    REQUIRE([probe.row accessibilityIndex] == 50000);
    REQUIRE([probe.row accessibilityParent] == list);
    REQUIRE([probe.row accessibilityChildren].count == 0);
    REQUIRE(![probe.row
        isAccessibilitySelectorAllowed:@selector(accessibilityFrame)]);
    @autoreleasepool {
      id temporary = at(list, 42);
      REQUIRE(temporary);
      probe.exportedRow = temporary;
    }
    REQUIRE(probe.exportedRow);
    REQUIRE(at(list, 42) == probe.exportedRow);
    REQUIRE([[probe.exportedRow accessibilityLabel] isEqual:@"Row 42"]);
    REQUIRE([list accessibilityArrayAttributeValues:NSAccessibilityRowsAttribute
                                              index:42
                                           maxCount:3] == nil);
    break;
  }
  case 1: {
    REQUIRE(at(list, 49999) == probe.row);
    REQUIRE([probe.row accessibilityIndex] == 49999);
    REQUIRE([[probe.row accessibilityLabel] isEqual:@"Row 50000"]);
    id emptyRow = at(list, 1);
    REQUIRE([[emptyRow accessibilityLabel] isEqual:@"Row 99998"]);
    REQUIRE([emptyRow accessibilityChildren].count == 0);
    NSRect frame = [emptyRow accessibilityFrame];
    REQUIRE(frame.size.width > 0 && frame.size.height > 0);
    REQUIRE([list accessibilityHitTest:NSMakePoint(NSMidX(frame),
                                                   NSMidY(frame))] == emptyRow);
    break;
  }
  case 2:
    REQUIRE([list accessibilityRowCount] == 99999);
    REQUIRE(![probe.row isAccessibilityElement]);
    REQUIRE(![probe.row accessibilityRole]);
    REQUIRE([[at(list, 49999) accessibilityLabel] isEqual:@"Row 49999"]);
    probe.retiredRow = probe.row;
    break;
  case 3:
    probe.row = at(list, 49999);
    REQUIRE(probe.row && probe.row != probe.retiredRow);
    REQUIRE(![probe.retiredRow isAccessibilityElement]);
    REQUIRE([[probe.row accessibilityLabel] isEqual:@"Row 50000"]);
    break;
  case 4: {
    REQUIRE(at(list, 49999) == probe.row);
    REQUIRE([probe.row
        isAccessibilitySelectorAllowed:@selector(accessibilityFrame)]);
    NSRect frame = [probe.row accessibilityFrame];
    REQUIRE(frame.size.width > 0 && frame.size.height > 0);
    NSArray *children = [probe.row accessibilityChildren];
    REQUIRE(children.count == 1);
    REQUIRE([children[0] accessibilityParent] == probe.row);
    REQUIRE([[children[0] accessibilityLabel] isEqual:@"Row 50000"]);
    REQUIRE([probe.row accessibilityHitTest:NSMakePoint(NSMidX(frame),
                                                        NSMidY(frame))] ==
            children[0]);
    REQUIRE([[list accessibilityVisibleRows] containsObject:probe.row]);
    break;
  }
  case 5:
    REQUIRE([probe.row isAccessibilityElement]);
    REQUIRE([[probe.row accessibilityLabel] isEqual:@"Row 50000"]);
    REQUIRE([probe.row accessibilityChildren].count == 0);
    REQUIRE(![probe.row
        isAccessibilitySelectorAllowed:@selector(accessibilityFrame)]);
    REQUIRE(![[list accessibilityVisibleRows] containsObject:probe.row]);
    break;
  case 6: {
    REQUIRE(![list isAccessibilityElement]);
    REQUIRE(!probe.exportedRow);
    REQUIRE(![probe.row isAccessibilityElement]);
    REQUIRE(![probe.row accessibilityRole]);
    REQUIRE([probe.row accessibilityChildren].count == 0);
    probe.gridTable = findRole(probe.root, NSAccessibilityTableRole);
    REQUIRE(probe.gridTable);
    REQUIRE([probe.gridTable accessibilityColumnCount] == 2);
    @autoreleasepool {
      id row = at(probe.gridTable, 49999);
      REQUIRE(row);
      probe.gridRow = row;
      probe.gridCell = [probe.gridTable accessibilityCellForColumn:1 row:49999];
      probe.gridColumn = [probe.gridTable accessibilityColumns][1];
    }
    REQUIRE(probe.gridRow && probe.gridCell && probe.gridColumn);
    REQUIRE([[probe.gridRow accessibilityLabel] isEqual:@"Row 50000"]);
    REQUIRE([[probe.gridCell accessibilityLabel] isEqual:@"Cell 1"]);
    REQUIRE([[probe.gridColumn accessibilityLabel] isEqual:@"Detail"]);
    REQUIRE([probe.gridCell accessibilityParent] == probe.gridRow);
    REQUIRE([probe.gridTable accessibilityCellForColumn:1 row:49999] ==
            probe.gridCell);
    REQUIRE([probe.gridTable accessibilityColumns][1] == probe.gridColumn);
    probe.retiredCell = probe.gridCell;
    probe.retiredColumn = probe.gridColumn;
    break;
  }
  case 7:
    REQUIRE([probe.gridTable accessibilityColumnCount] == 1);
    REQUIRE(at(probe.gridTable, 49999) == probe.gridRow);
    REQUIRE(![probe.retiredCell isAccessibilityElement]);
    REQUIRE(![probe.retiredColumn isAccessibilityElement]);
    REQUIRE(![probe.retiredCell accessibilityRole]);
    REQUIRE(![probe.retiredColumn accessibilityRole]);
    break;
  case 8:
    @autoreleasepool {
      probe.gridCell = [probe.gridTable accessibilityCellForColumn:1 row:49999];
      probe.gridColumn = [probe.gridTable accessibilityColumns][1];
    }
    REQUIRE(probe.gridCell && probe.gridColumn);
    REQUIRE(probe.gridCell != probe.retiredCell);
    REQUIRE(probe.gridColumn != probe.retiredColumn);
    REQUIRE(![probe.retiredCell isAccessibilityElement]);
    REQUIRE(![probe.retiredColumn isAccessibilityElement]);
    REQUIRE([[probe.gridCell accessibilityLabel] isEqual:@"Cell 1"]);
    break;
  case 9:
    REQUIRE(![probe.gridTable isAccessibilityElement]);
    REQUIRE(!probe.gridRow && !probe.gridCell && !probe.gridColumn);
    REQUIRE(![probe.retiredCell isAccessibilityElement]);
    REQUIRE(![probe.retiredColumn isAccessibilityElement]);
    break;
  default:
    return -1;
  }
  REQUIRE(!window || [window firstResponder] == responder);
  return 0;
}

void *scoop_ui_collection_probe(void *root) {
  ScoopCollectionProbe *probe = [ScoopCollectionProbe new];
  probe.root = (__bridge id)root;
  return (__bridge_retained void *)probe;
}
void scoop_ui_collection_post(void *pointer, int32_t phase,
                              int32_t (*callback)(void *, int32_t, int32_t),
                              void *context) {
  ScoopCollectionProbe *probe = (__bridge ScoopCollectionProbe *)pointer;
  dispatch_async(dispatch_get_main_queue(), ^{
    if (!probe.closed) {
      @autoreleasepool {
        callback(context, phase, checkCollection(probe, phase));
      }
    }
  });
}
void scoop_ui_collection_close(void *pointer) {
  ScoopCollectionProbe *probe =
      (__bridge_transfer ScoopCollectionProbe *)pointer;
  probe.closed = YES;
}
