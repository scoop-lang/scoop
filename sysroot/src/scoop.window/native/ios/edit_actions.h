#pragma once
#import "window.h"

typedef enum {
  SWIEditNone,
  SWIEditCut = 4,
  SWIEditCopy,
  SWIEditPaste,
  SWIEditSelectAll,
  SWIEditUndo,
  SWIEditRedo,
  SWIEditDelete
} SWIEditAction;

SWIEditAction swi_edit_action(SEL selector);
BOOL swi_can_edit(SWIView *view, SWIEditAction action);
void swi_edit(SWIView *view, SWIEditAction action);
NSUndoManager *swi_edit_undo_manager(SWIView *view);
