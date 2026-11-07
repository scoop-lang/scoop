#import "window.h"

@interface SWModal : NSObject
@property(nonatomic, strong) SWWindow *owner;
@property(nonatomic) int scope, presentation;
@property(nonatomic) BOOL started, ending, finished, running;
@end
@implementation SWModal
@end

void sw_show_owned_window(SWWindow *owner, BOOL activate) {
  if (owner.owningWindow && !owner.window.sheetParent &&
      !owner.window.parentWindow)
    [owner.owningWindow.window addChildWindow:owner.window
                                      ordered:NSWindowAbove];
  if (activate)
    [owner.window makeKeyAndOrderFront:nil];
  else
    [owner.window orderFront:nil];
}

static SWWindow *window_owner(NSWindow *window) {
  return [window.delegate isKindOfClass:SWWindow.class]
             ? (SWWindow *)window.delegate
             : nil;
}
BOOL sw_window_input_allowed(NSWindow *window) {
  SWWindow *owner = window_owner(window);
  return !owner || (!owner.inputBlocked && !owner.destroying);
}
BOOL sw_window_modal_eligible(NSWindow *window) {
  SWWindow *owner = window_owner(window);
  return owner && owner.application.modalStack.count && !owner.inputBlocked &&
         !owner.destroying;
}
BOOL sw_window_input_event(NSEvent *event) {
  NSEventMask mask = NSEventMaskLeftMouseDown | NSEventMaskLeftMouseUp |
                     NSEventMaskRightMouseDown | NSEventMaskRightMouseUp |
                     NSEventMaskMouseMoved | NSEventMaskLeftMouseDragged |
                     NSEventMaskRightMouseDragged | NSEventMaskMouseEntered |
                     NSEventMaskMouseExited | NSEventMaskKeyDown |
                     NSEventMaskKeyUp | NSEventMaskFlagsChanged |
                     NSEventMaskScrollWheel | NSEventMaskOtherMouseDown |
                     NSEventMaskOtherMouseUp | NSEventMaskOtherMouseDragged |
                     NSEventMaskTabletPoint | NSEventMaskTabletProximity |
                     NSEventMaskGesture | NSEventMaskMagnify |
                     NSEventMaskSwipe | NSEventMaskRotate;
  return (mask & (1ULL << event.type)) != 0;
}

int32_t scoop_window_management_capabilities(void) {
  return 1 | 2 | 4 | 8 | 16 | 64 | 128 | 256 | 512;
}
int64_t scoop_window_raise(void *pointer) {
  [((__bridge SWWindow *)pointer).window orderFront:nil];
  return 0;
}
int32_t scoop_window_observed_level(void *pointer) {
  NSInteger level = ((__bridge SWWindow *)pointer).window.level;
  return level > NSNormalWindowLevel ? 1 : level < NSNormalWindowLevel ? -1 : 0;
}
void scoop_window_input_blocked(void *pointer, int32_t blocked) {
  SWWindow *owner = (__bridge SWWindow *)pointer;
  if (owner.inputBlocked == (blocked != 0))
    return;
  owner.inputBlocked = blocked != 0;
  if (blocked) {
    owner.savedIgnoreMouse = owner.window.ignoresMouseEvents;
    [owner.view cancelPointer:3];
    [owner.view resetInput];
    if (owner.view.imeAllowed)
      [owner.view.inputContext deactivate];
    owner.window.ignoresMouseEvents = YES;
  } else {
    owner.window.ignoresMouseEvents = owner.savedIgnoreMouse;
    if (owner.window.keyWindow && owner.view.imeAllowed)
      [owner.view.inputContext activate];
  }
  [owner emit:19];
}

void sw_modal_resume_ending(SWApplication *application) {
  SWModal *session = application.modalStack.lastObject;
  if (!session.ending || !session.running ||
      NSApp.modalWindow != session.owner.window)
    return;
  [NSApp stopModal];
  [NSApp postEvent:[NSEvent otherEventWithType:NSEventTypeApplicationDefined
                                      location:NSZeroPoint
                                 modifierFlags:0
                                     timestamp:0
                                  windowNumber:0
                                       context:nil
                                       subtype:0
                                         data1:0
                                         data2:0]
           atStart:YES];
  sw_wake(application);
}
static void finish_sheet(SWModal *session) {
  SWWindow *owner = session.owner;
  [owner.window orderOut:nil];
  session.finished = YES;
  sw_wake(owner.application);
}

SWResult scoop_window_modal_create(void *pointer, int32_t scope,
                                   int32_t presentation) {
  SWModal *session = [SWModal new];
  session.owner = (__bridge SWWindow *)pointer;
  session.scope = scope;
  session.presentation = presentation;
  return (SWResult){(__bridge_retained void *)session, 0};
}
int64_t scoop_window_modal_start(void *pointer) {
  SWModal *session = (__bridge SWModal *)pointer;
  if (session.ending) {
    session.finished = YES;
    return 0;
  }
  SWWindow *owner = session.owner;
  session.started = YES;
  @try {
    if (session.presentation) {
      NSWindow *parent = owner.owningWindow.window;
      if (!parent || parent.attachedSheet) {
        session.finished = YES;
        return -5;
      }
      if (owner.window.parentWindow) {
        [owner.window.parentWindow removeChildWindow:owner.window];
      }
      [parent beginSheet:owner.window
          completionHandler:^(NSModalResponse response) {
            finish_sheet(session);
          }];
    } else if (session.scope) {
      if (!owner.application.modalStack)
        owner.application.modalStack = [NSMutableArray new];
      [owner.application.modalStack addObject:session];
      session.running = YES;
      sw_show_owned_window(owner, NO);
      @try {
        [NSApp runModalForWindow:owner.window];
      } @finally {
        session.running = NO;
        [owner.window orderOut:nil];
        session.finished = YES;
        [owner.application.modalStack removeObjectIdenticalTo:session];
        sw_modal_resume_ending(owner.application);
      }
    } else {
      sw_show_owned_window(owner, YES);
    }
    return 0;
  } @catch (NSException *exception) {
    if (owner.window.sheetParent) {
      session.ending = YES;
      [owner.window.sheetParent endSheet:owner.window];
    } else {
      [owner.window orderOut:nil];
      session.finished = YES;
    }
    return 1;
  }
}
void scoop_window_modal_end(void *pointer) {
  SWModal *session = (__bridge SWModal *)pointer;
  if (session.ending)
    return;
  session.ending = YES;
  if (!session.started)
    session.finished = YES;
  else if (session.presentation && session.owner.window.sheetParent)
    [session.owner.window.sheetParent endSheet:session.owner.window];
  else if (session.running)
    sw_modal_resume_ending(session.owner.application);
  else {
    [session.owner.window orderOut:nil];
    session.finished = YES;
  }
  sw_wake(session.owner.application);
}
int32_t scoop_window_modal_finished(void *pointer) {
  return ((__bridge SWModal *)pointer).finished;
}
void scoop_window_modal_release(void *pointer) {
  (void)CFBridgingRelease(pointer);
}
