#import "window.h"
#import <objc/runtime.h>
#include <string.h>

// Codes are local bridge values decoded by platform/darwin/commands.scoop.
#define MOTION(name, code)                                                     \
  {#name ":", code}, {#name "AndModifySelection:", code | 256}

int32_t sw_text_command(SEL selector) {
  static const struct {
    const char *name;
    int32_t code;
  } commands[] = {
      MOTION(moveBackward, 1),
      MOTION(moveForward, 2),
      MOTION(moveLeft, 3),
      MOTION(moveRight, 4),
      MOTION(moveUp, 5),
      MOTION(moveDown, 6),
      MOTION(moveWordBackward, 7),
      MOTION(moveWordForward, 8),
      MOTION(moveWordLeft, 9),
      MOTION(moveWordRight, 10),
      MOTION(moveToBeginningOfLine, 11),
      MOTION(moveToEndOfLine, 12),
      MOTION(moveToLeftEndOfLine, 13),
      MOTION(moveToRightEndOfLine, 14),
      MOTION(moveToBeginningOfParagraph, 15),
      MOTION(moveToEndOfParagraph, 16),
      MOTION(moveToBeginningOfDocument, 17),
      MOTION(moveToEndOfDocument, 18),
      MOTION(pageUp, 19),
      MOTION(pageDown, 20),
      {"deleteBackward:", 32},
      {"deleteForward:", 33},
      {"deleteWordBackward:", 34},
      {"deleteWordForward:", 35},
      {"deleteToBeginningOfLine:", 36},
      {"deleteToEndOfLine:", 37},
      {"deleteToBeginningOfParagraph:", 38},
      {"deleteToEndOfParagraph:", 39},
      {"copy:", 40},
      {"cut:", 41},
      {"paste:", 42},
      {"selectAll:", 43},
      {"undo:", 44},
      {"redo:", 45},
      {"insertNewline:", 46},
      {"insertNewlineIgnoringFieldEditor:", 46},
      {"insertLineBreak:", 46},
      {"insertParagraphSeparator:", 46},
      {"insertTab:", 47},
      {"insertTabIgnoringFieldEditor:", 47},
      {"insertBacktab:", 48},
      {"cancelOperation:", 49},
  };
  const char *name = sel_getName(selector);
  for (size_t index = 0; index < sizeof(commands) / sizeof(commands[0]);
       ++index)
    if (strcmp(name, commands[index].name) == 0)
      return commands[index].code;
  return 0;
}

int64_t scoop_window_command_count(void *pointer) {
  return (int64_t)((__bridge SWEvent *)pointer).commands.count;
}

int32_t scoop_window_command(void *pointer, int64_t index) {
  return ((__bridge SWEvent *)pointer).commands[(NSUInteger)index].intValue;
}
