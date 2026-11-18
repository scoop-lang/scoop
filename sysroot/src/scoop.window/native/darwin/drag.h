#ifndef SCOOP_WINDOW_DARWIN_DRAG_H
#define SCOOP_WINDOW_DARWIN_DRAG_H

#import "window.h"

NSDraggingItem *sw_drag_item(int32_t format, const uint8_t *bytes,
                             int64_t length);

#endif
