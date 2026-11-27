#import "accessibility_collection.h"

static NSInteger columnIndex(SWAccessibilityState *state, id key) {
  if (!state || !key)
    return NSNotFound;
  for (NSUInteger index = 0; index < state.gridColumns.count; ++index)
    if ([state.gridColumns[index].key isEqual:key])
      return (NSInteger)index;
  return NSNotFound;
}

static SWAccessibilityNode *mountedCell(SWAccessibilityNode *node, id key) {
  if (!node.state)
    return nil;
  if ([node.state.cellColumnKey isEqual:key])
    return node;
  for (NSNumber *number in node.tree.children[@(node.number)]) {
    SWAccessibilityNode *found =
        mountedCell([node.tree element:number.longLongValue], key);
    if (found)
      return found;
  }
  return nil;
}

@interface SWAccessibilityVirtualCell : NSAccessibilityElement
@property(nonatomic, weak) SWAccessibilityVirtualItem *row;
@property(nonatomic, copy) id key;
@property(nonatomic) BOOL retired;
@property(nonatomic) uint64_t revision;
@property(nonatomic, strong) SWAccessibilityItemData *cached;
@end

@implementation SWAccessibilityVirtualCell
- (NSInteger)columnIndex {
  return columnIndex(self.row.collection.state, self.key);
}
- (BOOL)isAccessibilityElement {
  return !self.retired && self.row.isAccessibilityElement &&
         self.columnIndex != NSNotFound && self.row.hasGridCells;
}
- (SWAccessibilityItemData *)data {
  if (!self.isAccessibilityElement)
    return nil;
  SWAccessibilityState *state = self.row.collection.state;
  if (self.cached && self.revision == state.collectionRevision)
    return self.cached;
  SWAccessibilityItemData *data = [SWAccessibilityItemData new];
  if ([self.row.collection query:11
                           index:self.columnIndex
                             key:self.row.key
                           reply:data] != 1)
    return nil;
  self.cached = data;
  self.revision = state.collectionRevision;
  return data;
}
- (SWAccessibilityNode *)mounted {
  return self.isAccessibilityElement
             ? mountedCell([self.row.collection mountedRow:self.row.key],
                           self.key)
             : nil;
}
- (NSAccessibilityRole)accessibilityRole {
  return self.isAccessibilityElement ? NSAccessibilityCellRole : nil;
}
- (id)accessibilityParent {
  return self.isAccessibilityElement ? self.row : nil;
}
- (id)accessibilityWindow {
  return self.row.accessibilityWindow;
}
- (id)accessibilityTopLevelUIElement {
  return self.accessibilityWindow;
}
- (NSString *)accessibilityLabel {
  return self.data.name;
}
- (NSString *)accessibilityTitle {
  return self.data.name;
}
- (NSString *)accessibilityHelp {
  return self.data.help;
}
- (NSString *)accessibilityIdentifier {
  return self.data.identifier;
}
- (id)accessibilityValue {
  return self.data.value;
}
- (BOOL)isAccessibilityEnabled {
  return self.data.enabled && self.row.isAccessibilityEnabled;
}
- (BOOL)isAccessibilitySelected {
  return self.data.selected == 1;
}
- (NSRange)accessibilityRowIndexRange {
  SWAccessibilityItemData *row = self.row.itemData;
  return row && self.isAccessibilityElement
             ? NSMakeRange((NSUInteger)row.index, 1)
             : NSMakeRange(NSNotFound, 0);
}
- (NSRange)accessibilityColumnIndexRange {
  NSInteger index = self.columnIndex;
  if (!self.isAccessibilityElement)
    return NSMakeRange(NSNotFound, 0);
  if (self.row.collection.state.collectionActions & 128) {
    SWAccessibilityItemData *data = self.data;
    if (data)
      return NSMakeRange((NSUInteger)data.index, (NSUInteger)data.span);
  }
  SWAccessibilityNode *mounted = self.mounted;
  if (mounted)
    return mounted.accessibilityColumnIndexRange;
  return NSMakeRange((NSUInteger)index, 1);
}
- (NSArray *)accessibilityColumnHeaderUIElements {
  NSInteger index = self.columnIndex;
  if (!self.isAccessibilityElement || index == NSNotFound)
    return @[];
  NSArray *mounted = self.mounted.accessibilityColumnHeaderUIElements;
  if (mounted.count)
    return mounted;
  int64_t header =
      self.row.collection.state.gridColumns[(NSUInteger)index].header;
  SWAccessibilityNode *node = [self.row.collection.tree element:header];
  return header && node.isAccessibilityElement ? @[ node ] : @[];
}
- (NSArray *)accessibilityRowHeaderUIElements {
  return @[];
}
- (NSArray *)accessibilityChildren {
  return self.mounted.accessibilityChildren ?: @[];
}
- (NSArray *)accessibilityChildrenInNavigationOrder {
  return self.accessibilityChildren;
}
- (NSRect)accessibilityFrame {
  return self.mounted.accessibilityFrame;
}
- (id)accessibilityHitTest:(NSPoint)point {
  id result = [self.mounted accessibilityHitTest:point];
  return result == self.mounted ? self : result;
}
- (NSArray *)accessibilityCustomActions {
  if (!self.isAccessibilityEnabled)
    return @[];
  int32_t flags = self.row.collection.state.collectionActions;
  NSMutableArray *actions = [NSMutableArray new];
  if (flags & 16)
    [actions addObject:[[NSAccessibilityCustomAction alloc]
                           initWithName:@"Realize Cell"
                                 target:self
                               selector:@selector(realizeAccessibilityCell)]];
  if (flags & 32) {
    NSString *name = @"Scroll to Visible";
    if (@available(macOS 26.0, *)) {
      NSString *localized = NSAccessibilityActionDescription(
          NSAccessibilityScrollToVisibleAction);
      if (localized.length)
        name = localized;
    }
    [actions addObject:[[NSAccessibilityCustomAction alloc]
                           initWithName:name
                                 target:self
                               selector:@selector(revealAccessibilityCell)]];
  }
  return actions;
}
- (BOOL)performCell:(int32_t)operation {
  if (!self.isAccessibilityEnabled)
    return NO;
  int32_t flag = operation == 12 ? 16 : operation == 13 ? 32 : 64;
  if (!(self.row.collection.state.collectionActions & flag) ||
      (flag == 64 && !self.data.focusable))
    return NO;
  NSInteger column = self.columnIndex;
  if (column == NSNotFound)
    return NO;
  return [self.row.collection query:operation
                              index:column
                                key:self.row.key
                              reply:nil] == 1 &&
         self.isAccessibilityElement;
}
- (BOOL)realizeAccessibilityCell {
  return [self performCell:12];
}
- (BOOL)revealAccessibilityCell {
  return [self performCell:13];
}
- (void)setAccessibilityFocused:(BOOL)focused {
  [self performCell:focused ? 14 : 15];
}
- (BOOL)isAccessibilityFocused {
  if (!self.isAccessibilityElement ||
      !self.row.collection.tree.view.window.isKeyWindow)
    return NO;
  SWAccessibilityNode *cell = self.mounted;
  int64_t number = self.row.collection.tree.focused;
  while (number && cell) {
    if (number == cell.number)
      return YES;
    number = self.row.collection.tree.published[@(number)].data.parent;
  }
  return NO;
}
- (BOOL)isAccessibilitySelectorAllowed:(SEL)selector {
  if (!self.isAccessibilityElement)
    return NO;
  if (selector == @selector(accessibilityFrame))
    return self.mounted.state.data.geometry != 0;
  if (selector == @selector(isAccessibilitySelected))
    return self.data && self.data.selected >= 0;
  if (selector == @selector(setAccessibilityFocused:) ||
      selector == @selector(isAccessibilityFocused))
    return self.data.focusable &&
           (self.row.collection.state.collectionActions & 64);
  if (selector == @selector(accessibilityCustomActions))
    return (self.row.collection.state.collectionActions & 48) != 0;
  if (selector == @selector(setAccessibilitySelected:) ||
      selector == @selector(setAccessibilityValue:))
    return NO;
  return [super isAccessibilitySelectorAllowed:selector];
}
@end

@implementation SWAccessibilityVirtualItem (Grid)
- (BOOL)hasGridCells {
  if (!(self.collection.state.collectionActions & 512))
    return YES;
  SWAccessibilityItemData *data = self.itemData;
  if (data)
    return data.hasCells != 0;
  SWAccessibilityItemData *shape = [SWAccessibilityItemData new];
  return [self.collection query:16 index:0 key:self.key reply:shape] != 2;
}
- (id)cellForKey:(id)key {
  NSInteger index = columnIndex(self.collection.state, key);
  if (!self.isAccessibilityElement || index == NSNotFound || !self.hasGridCells)
    return nil;
  SWAccessibilityItemData *data = nil;
  if (self.collection.state.collectionActions & 128) {
    data = [SWAccessibilityItemData new];
    if ([self.collection query:11 index:index key:self.key reply:data] == 1) {
      key = data.key;
    } else {
      // Focus publication can refer to an actual anchor while owner queries are
      // busy.
      if (!mountedCell([self.collection mountedRow:self.key], key))
        return nil;
      data = nil;
    }
  }
  if (!self.cells)
    self.cells = [NSMapTable strongToStrongObjectsMapTable];
  SWAccessibilityVirtualCell *cell = [self.cells objectForKey:key];
  if (!cell || cell.retired) {
    cell = [SWAccessibilityVirtualCell new];
    cell.row = self;
    cell.key = key;
    [self.cells setObject:cell forKey:key];
  }
  if (data) {
    cell.cached = data;
    cell.revision = self.collection.state.collectionRevision;
  }
  return cell;
}
- (NSArray *)gridCells {
  NSMutableArray *result = [NSMutableArray new];
  SWAccessibilityState *state = self.collection.state;
  for (NSUInteger index = 0; index < state.gridColumns.count;) {
    SWAccessibilityColumnData *column = state.gridColumns[index];
    id cell = [self cellForKey:column.key];
    if (cell)
      [result addObject:cell];
    SWAccessibilityItemData *data =
        state.collectionActions & 128
            ? ((SWAccessibilityVirtualCell *)cell).cached
            : nil;
    index =
        data ? MAX(index + 1, (NSUInteger)(data.index + data.span)) : index + 1;
  }
  return result;
}
- (void)refreshCells {
  for (SWAccessibilityVirtualCell *cell in self.cells.objectEnumerator
           .allObjects) {
    NSInteger index = columnIndex(self.collection.state, cell.key);
    BOOL absent = NO;
    if (!self.retired && index != NSNotFound &&
        (self.collection.state.collectionActions & (128 | 512))) {
      SWAccessibilityItemData *data = [SWAccessibilityItemData new];
      int32_t result = [self.collection query:16
                                        index:index
                                          key:self.key
                                        reply:data];
      absent = result == 2 || (result == 1 && ![data.key isEqual:cell.key]);
    }
    if (self.retired || index == NSNotFound || absent) {
      cell.retired = YES;
      cell.cached = nil;
      [self.cells removeObjectForKey:cell.key];
      NSAccessibilityPostNotification(
          cell, NSAccessibilityUIElementDestroyedNotification);
    }
  }
}
@end

id sw_accessibility_virtual_cell(SWAccessibilityTree *tree, int64_t number) {
  SWAccessibilityState *cell = tree.published[@(number)];
  if (!cell.cellColumnKey)
    return nil;
  int64_t parent = cell.data.parent;
  while (parent) {
    SWAccessibilityState *state = tree.published[@(parent)];
    if (!state)
      return nil;
    if (state.itemKey) {
      SWAccessibilityVirtualItem *row =
          [[tree collection:state.data.parent] itemForKey:state.itemKey];
      return [row cellForKey:cell.cellColumnKey];
    }
    parent = state.data.parent;
  }
  return nil;
}

static void collectRole(SWAccessibilityNode *node, int32_t role, BOOL visible,
                        NSMutableArray *result) {
  for (NSNumber *number in node.tree.children[@(node.number)]) {
    SWAccessibilityNode *child = [node.tree element:number.longLongValue];
    if (![child participatesInAccessibility])
      continue;
    if (child.state.data.role == role ||
        (role == 28 && child.state.data.role == 30)) {
      if (!visible || (child.state.data.geometry && child.state.data.visible))
        [result addObject:child];
      if (role == 28 && child.state.data.role == 30 &&
          child.state.data.expanded == 1)
        collectRole(child, role, visible, result);
    } else if ((child.state.data.role != 22 && child.state.data.role != 29))
      collectRole(child, role, visible, result);
  }
}

NSArray *sw_accessibility_table_rows(SWAccessibilityNode *node, BOOL visible) {
  SWAccessibilityCollection *collection = [node.tree collection:node.number];
  if (collection)
    return visible ? collection.visibleRows
                   : [collection presentedRowsFrom:0 count:NSUIntegerMax];
  NSMutableArray *result = [NSMutableArray new];
  if (node.isAccessibilityElement)
    collectRole(node, 28, visible, result);
  return result;
}

NSArray *sw_accessibility_table_headers(SWAccessibilityNode *node, BOOL row) {
  NSMutableArray *result = [NSMutableArray new];
  if (!node.isAccessibilityElement)
    return result;
  if (node.state.itemCount >= 0 && !row) {
    for (SWAccessibilityColumnData *column in node.state.gridColumns) {
      SWAccessibilityNode *header = [node.tree element:column.header];
      if (column.header && header.isAccessibilityElement &&
          ![result containsObject:header])
        [result addObject:header];
    }
  } else
    collectRole(node, row ? 24 : 25, NO, result);
  return result;
}

static id staticCell(SWAccessibilityNode *node, NSInteger column,
                     NSInteger row) {
  for (NSNumber *number in node.tree.children[@(node.number)]) {
    SWAccessibilityNode *child = [node.tree element:number.longLongValue];
    if (![child participatesInAccessibility] ||
        (child.state.data.role == 22 || child.state.data.role == 29))
      continue;
    SWAccessibilityData data = child.state.data;
    if (data.cell_row >= 0 && row >= data.cell_row &&
        row - data.cell_row < data.row_span && column >= data.cell_column &&
        column - data.cell_column < data.column_span)
      return child;
    id result = staticCell(child, column, row);
    if (result)
      return result;
  }
  return nil;
}

@interface SWAccessibilityTableColumn : NSAccessibilityElement
@property(nonatomic, weak) SWAccessibilityTree *tree;
@property(nonatomic) int64_t owner;
@property(nonatomic, copy) id key;
@property(nonatomic) BOOL logical, retired;
@end

@implementation SWAccessibilityTableColumn
- (SWAccessibilityNode *)table {
  return self.retired ? nil : [self.tree element:self.owner];
}
- (NSInteger)accessibilityIndex {
  if (self.logical)
    return columnIndex(self.table.state, self.key);
  NSInteger index = [self.key integerValue];
  return index >= 0 && index < self.table.state.data.table_columns ? index
                                                                   : NSNotFound;
}
- (BOOL)isAccessibilityElement {
  return self.table.isAccessibilityElement &&
         self.accessibilityIndex != NSNotFound;
}
- (NSAccessibilityRole)accessibilityRole {
  return self.isAccessibilityElement ? NSAccessibilityColumnRole : nil;
}
- (id)accessibilityParent {
  return self.isAccessibilityElement ? self.table : nil;
}
- (id)accessibilityWindow {
  return self.table.accessibilityWindow;
}
- (id)accessibilityTopLevelUIElement {
  return self.accessibilityWindow;
}
- (NSArray *)accessibilityChildren {
  if (!self.isAccessibilityElement)
    return @[];
  NSMutableArray *result = [NSMutableArray new];
  for (NSInteger row = 0; row < self.table.accessibilityRowCount; ++row) {
    id cell = [self.table accessibilityCellForColumn:self.accessibilityIndex
                                                 row:row];
    if (cell)
      [result addObject:cell];
  }
  return result;
}
- (id)accessibilityHeader {
  NSInteger index = self.accessibilityIndex;
  if (!self.isAccessibilityElement || index == NSNotFound)
    return nil;
  if (self.logical) {
    int64_t header = self.table.state.gridColumns[(NSUInteger)index].header;
    return header ? [self.tree element:header] : nil;
  }
  NSMutableArray *headers = [NSMutableArray new];
  collectRole(self.table, 25, NO, headers);
  for (SWAccessibilityNode *header in headers) {
    SWAccessibilityData data = header.state.data;
    if (index >= data.cell_column &&
        index - data.cell_column < data.column_span)
      return header;
  }
  return nil;
}
- (NSString *)accessibilityLabel {
  if (!self.isAccessibilityElement)
    return nil;
  return self.logical
             ? self.table.state.gridColumns[(NSUInteger)self.accessibilityIndex]
                   .name
             : [self.accessibilityHeader accessibilityLabel];
}
- (BOOL)isAccessibilitySelectorAllowed:(SEL)selector {
  if (!self.isAccessibilityElement)
    return NO;
  if (selector == @selector(accessibilityFrame) ||
      selector == @selector(setAccessibilityFocused:) ||
      selector == @selector(setAccessibilitySelected:))
    return NO;
  return [super isAccessibilitySelectorAllowed:selector];
}
@end

@implementation SWAccessibilityNode (Table)
- (NSInteger)accessibilityColumnCount {
  SWAccessibilityState *state = self.state;
  return state.itemCount >= 0 ? (NSInteger)state.gridColumns.count
                              : MAX(state.data.table_columns, 0);
}
- (id)accessibilityCellForColumn:(NSInteger)column row:(NSInteger)row {
  if (!self.isAccessibilityElement || column < 0 ||
      column >= self.accessibilityColumnCount || row < 0 ||
      row >= self.accessibilityRowCount)
    return nil;
  SWAccessibilityCollection *collection = [self.tree collection:self.number];
  if (!collection)
    return staticCell(self, column, row);
  NSArray *rows = [collection rowsFrom:(NSUInteger)row count:1 selectedOnly:NO];
  return rows.count
             ? [rows[0]
                   cellForKey:self.state.gridColumns[(NSUInteger)column].key]
             : nil;
}
- (NSArray *)accessibilityColumns {
  if (!self.isAccessibilityElement)
    return @[];
  if (!self.tree.tableColumns)
    self.tree.tableColumns = [NSMutableDictionary new];
  NSMapTable *cache = self.tree.tableColumns[@(self.number)];
  if (!cache) {
    cache = [NSMapTable strongToStrongObjectsMapTable];
    self.tree.tableColumns[@(self.number)] = cache;
  }
  NSMutableArray *result = [NSMutableArray new];
  for (NSInteger index = 0; index < self.accessibilityColumnCount; ++index) {
    BOOL logical = self.state.itemCount >= 0;
    id key = logical ? self.state.gridColumns[(NSUInteger)index].key : @(index);
    SWAccessibilityTableColumn *column = [cache objectForKey:key];
    if (!column || column.retired || column.logical != logical) {
      column = [SWAccessibilityTableColumn new];
      column.tree = self.tree;
      column.owner = self.number;
      column.key = key;
      column.logical = logical;
      [cache setObject:column forKey:key];
    }
    [result addObject:column];
  }
  return result;
}
- (NSArray *)accessibilitySelectedRows {
  NSMutableArray *rows = [NSMutableArray new];
  for (id child in self.accessibilitySelectedChildren)
    if ([[child accessibilityRole] isEqualToString:NSAccessibilityRowRole])
      [rows addObject:child];
  return rows;
}
- (NSInteger)accessibilityIndex {
  if (!self.isAccessibilityElement ||
      (self.state.data.role != 28 && self.state.data.role != 30))
    return NSNotFound;
  SWAccessibilityNode *parent = [self.tree element:self.state.data.parent];
  while (parent &&
         (parent.state.data.role != 22 && parent.state.data.role != 29))
    parent = parent.number ? [self.tree element:parent.state.data.parent] : nil;
  NSUInteger index =
      [sw_accessibility_table_rows(parent, NO) indexOfObjectIdenticalTo:self];
  return index == NSNotFound ? NSNotFound : (NSInteger)index;
}
- (NSRange)accessibilityRowIndexRange {
  SWAccessibilityData data = self.state.data;
  return self.state && data.cell_row >= 0
             ? NSMakeRange((NSUInteger)data.cell_row, (NSUInteger)data.row_span)
             : NSMakeRange(NSNotFound, 0);
}
- (NSRange)accessibilityColumnIndexRange {
  SWAccessibilityData data = self.state.data;
  return self.state && data.cell_column >= 0
             ? NSMakeRange((NSUInteger)data.cell_column,
                           (NSUInteger)data.column_span)
             : NSMakeRange(NSNotFound, 0);
}
@end

void sw_accessibility_grid_commit(SWAccessibilityTree *tree) {
  for (NSNumber *owner in tree.tableColumns.allKeys) {
    NSMapTable *columns = tree.tableColumns[owner];
    for (SWAccessibilityTableColumn *column in columns.objectEnumerator
             .allObjects) {
      if (column.accessibilityIndex == NSNotFound || !column.table.state) {
        column.retired = YES;
        [columns removeObjectForKey:column.key];
        NSAccessibilityPostNotification(
            column, NSAccessibilityUIElementDestroyedNotification);
      }
    }
    if (!tree.published[owner])
      [tree.tableColumns removeObjectForKey:owner];
  }
}
