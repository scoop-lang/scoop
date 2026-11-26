#import "text.h"

@implementation SWITextView (Keyboard)
- (void)observeKeyboard {
  NSNotificationCenter *center = NSNotificationCenter.defaultCenter;
  for (NSNotificationName name in @[
         UIKeyboardDidShowNotification, UIKeyboardDidHideNotification,
         UIKeyboardDidChangeFrameNotification
       ])
    [center addObserver:self
               selector:@selector(keyboardChanged:)
                   name:name
                 object:nil];
}
- (void)keyboardChanged:(NSNotification *)note {
  if (!self.isFirstResponder && !self.keyboardVisible)
    return;
  if ([note.object isKindOfClass:UIScreen.class] &&
      note.object != self.window.windowScene.screen)
    return;
  NSNumber *local = note.userInfo[UIKeyboardIsLocalUserInfoKey];
  if (local && !local.boolValue)
    return;
  NSValue *end = note.userInfo[UIKeyboardFrameEndUserInfoKey];
  UIScreen *screen = self.content.window.windowScene.screen;
  BOOL onScreen = !end || CGRectIntersectsRect(screen.coordinateSpace.bounds,
                                               end.CGRectValue);
  self.keyboardVisible =
      self.isFirstResponder && onScreen &&
      ![note.name isEqualToString:UIKeyboardDidHideNotification];
  [self updateKeyboardGeometry];
}
- (void)updateKeyboardGeometry {
  SWSoftwareKeyboard next = {.state = 1};
  if (self.keyboardVisible && self.isFirstResponder) {
    CGRect overlap = CGRectIntersection(
        self.content.bounds, self.content.keyboardLayoutGuide.layoutFrame);
    next.state = 2;
    if (!CGRectIsEmpty(overlap))
      next = (SWSoftwareKeyboard){3, overlap.origin.x, overlap.origin.y,
                                  overlap.size.width, overlap.size.height};
  }
  SWSoftwareKeyboard previous = self.observedKeyboard;
  if (next.state == previous.state && next.x == previous.x &&
      next.y == previous.y && next.width == previous.width &&
      next.height == previous.height)
    return;
  self.observedKeyboard = next;
  SWIEvent *event = [SWIEvent new];
  event.header =
      (SWEventHeader){.window = self.content.owner.identity, .kind = 22};
  event.data = (SWEventData){.flags = next.state,
                             .x = next.x,
                             .y = next.y,
                             .z = next.width,
                             .w = next.height};
  [self.content.owner.application enqueue:event];
}
@end

int64_t scoop_window_text_purpose(void *pointer, int64_t connection,
                                  int32_t purpose) {
  SWITextView *editor = ((__bridge SWIScene *)pointer).view.editor;
  if (!editor.connected || editor.connection != connection)
    return -4;
  UIKeyboardType type = UIKeyboardTypeDefault;
  switch (purpose) {
  case 0:
  case 6:
  case 7:
    break;
  case 1:
    type = UIKeyboardTypeNumberPad;
    break;
  case 2:
    type = UIKeyboardTypeDecimalPad;
    break;
  case 3:
    type = UIKeyboardTypeEmailAddress;
    break;
  case 4:
    type = UIKeyboardTypeURL;
    break;
  case 5:
    type = UIKeyboardTypePhonePad;
    break;
  default:
    return -4;
  }
  editor.secureTextEntry = purpose == 6;
  editor.keyboardType = type;
  editor.returnKeyType = purpose == 7 ? UIReturnKeySearch : UIReturnKeyDefault;
  if (editor.isFirstResponder)
    [editor reloadInputViews];
  return 0;
}
int64_t scoop_window_keyboard_request(void *pointer, int64_t connection,
                                      int32_t visible, void *input) {
  SWITextView *editor = ((__bridge SWIScene *)pointer).view.editor;
  if (!editor.connected || editor.connection != connection || !editor.window)
    return 1;
  BOOL accepted =
      visible ? (editor.isFirstResponder || [editor becomeFirstResponder])
              : (!editor.isFirstResponder || [editor resignFirstResponder]);
  return accepted ? 0 : 1;
}
SWSoftwareKeyboard scoop_window_keyboard_state(void *pointer) {
  SWITextView *editor = ((__bridge SWIScene *)pointer).view.editor;
  return editor ? editor.observedKeyboard : (SWSoftwareKeyboard){.state = 1};
}
