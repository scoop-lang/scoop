#ifndef SCOOP_WINDOW_IOS_ACCESSIBILITY_H
#define SCOOP_WINDOW_IOS_ACCESSIBILITY_H

#include "../accessibility_abi.h"
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
  double x, y, width, height;
} SWIAccessibilityRect;

enum SWIAccessibilityQuery { SWIVisible, SWIEnabled, SWIFocused };
enum SWIAccessibilitySpace { SWIViewBounds, SWIToScreen, SWIFromScreen };

void *swi_ax_attach(void *scene, void *tree);
void swi_ax_detach(void *projection);
int swi_ax_on_main(void);
int swi_ax_query(void *projection, enum SWIAccessibilityQuery query);
int swi_ax_geometry(void *projection, enum SWIAccessibilitySpace space,
                    SWIAccessibilityRect *rect);
void swi_ax_update(void *projection, const SWAccessibilityData *data,
                   const SWAccessibilityBytes fields[6]);
void swi_ax_remove(void *projection, int64_t number);
void swi_ax_order(void *projection, const int64_t *numbers, size_t count);
void swi_ax_announce(void *projection, SWAccessibilityBytes text, int priority);
int swi_ax_perform(void *tree, int64_t number, int32_t action);
int swi_ax_scroll(void *tree, int64_t number, int axis, int forward);

#ifdef __cplusplus
}
#endif
#endif
