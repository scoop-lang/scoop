#import "edit_actions.h"
#import "text.h"

SWIEditAction swi_edit_action(SEL selector) {
  if (selector == @selector(cut:))
    return SWIEditCut;
  if (selector == @selector(copy:))
    return SWIEditCopy;
  if (selector == @selector(paste:))
    return SWIEditPaste;
  if (selector == @selector(selectAll:))
    return SWIEditSelectAll;
  if (selector == @selector(delete:))
    return SWIEditDelete;
  return SWIEditNone;
}

BOOL swi_can_edit(SWIView *view, SWIEditAction action) {
  return action != SWIEditNone &&
         (view.editFlags & (1 << (action - SWIEditCut))) != 0;
}

void swi_edit(SWIView *view, SWIEditAction action) {
  if (!swi_can_edit(view, action))
    return;
  SWIScene *scene = view.owner;
  SWIEvent *event = [SWIEvent new];
  event.header = (SWEventHeader){.window = scene.identity, .kind = 16};
  event.data = (SWEventData){.a = action};
  [scene.application enqueue:event];
}

@interface SWIUndoManager : NSUndoManager
@property(nonatomic, weak) SWIView *content;
@end
@implementation SWIUndoManager
- (BOOL)canUndo {
  return swi_can_edit(self.content, SWIEditUndo);
}
- (BOOL)canRedo {
  return swi_can_edit(self.content, SWIEditRedo);
}
- (void)undo {
  swi_edit(self.content, SWIEditUndo);
}
- (void)redo {
  swi_edit(self.content, SWIEditRedo);
}
@end

NSUndoManager *swi_edit_undo_manager(SWIView *view) {
  if (!view.editUndo) {
    SWIUndoManager *manager = [SWIUndoManager new];
    manager.content = view;
    // The Scoop document owns undo records, including edits made by the IME.
    [manager disableUndoRegistration];
    view.editUndo = manager;
  }
  return view.editUndo;
}

void scoop_window_text_commands(void *pointer, int32_t flags) {
  SWIView *view = ((__bridge SWIScene *)pointer).view;
  view.editFlags = flags;
  if (flags && view.window.isKeyWindow && !view.editor.isFirstResponder &&
      !view.isFirstResponder)
    [view becomeFirstResponder];
}
