#include "common.h"
#include <errno.h>
#include <stdlib.h>
#include <string.h>

char *sn_copy(const uint8_t *bytes, int64_t length) {
  if (length < 0 || (uint64_t)length >= SIZE_MAX)
    return NULL;
  char *copy = malloc((size_t)length + 1);
  if (!copy)
    return NULL;
  if (length)
    memcpy(copy, bytes, (size_t)length);
  copy[length] = 0;
  return copy;
}

SNEvent *sn_event(int32_t kind, int64_t request) {
  SNEvent *event = calloc(1, sizeof(*event));
  if (!event)
    abort();
  event->data.kind = kind;
  event->data.request = request;
  return event;
}
void sn_event_set(SNEvent *event, int field, const char *text) {
  char *value = sn_copy((const uint8_t *)text, (int64_t)strlen(text));
  if (!value)
    abort();
  free(event->text[field]);
  event->text[field] = value;
}
void sn_event_error(SNEvent *event, int status, int64_t code,
                    const char *message) {
  event->data.status = status;
  event->data.code = code;
  if (message)
    sn_event_set(event, 3, message);
}
SNData scoop_notification_event_data(SNEvent *event) { return event->data; }
SNText scoop_notification_event_text(SNEvent *event, int32_t field) {
  const char *text = event->text[field];
  return (SNText){(const uint8_t *)text, text ? (int64_t)strlen(text) : 0};
}
void scoop_notification_event_destroy(SNEvent *event) {
  if (!event)
    return;
  for (int i = 0; i < 4; ++i)
    free(event->text[i]);
  free(event);
}

SDResult scoop_notification_content_create(int32_t silent) {
  SNContent *content = calloc(1, sizeof(*content));
  if (!content)
    return (SDResult){NULL, ENOMEM};
  content->silent = silent;
  return (SDResult){content, 0};
}
const char *sn_content_text(const SNContent *content, int field) {
  return content->text[field] ? content->text[field] : "";
}
int64_t scoop_notification_content_text(SNContent *content, int32_t field,
                                        const uint8_t *text, int64_t length) {
  char *copy = sn_copy(text, length);
  if (!copy)
    return ENOMEM;
  free(content->text[field]);
  content->text[field] = copy;
  return 0;
}
int64_t scoop_notification_content_action(SNContent *content, const uint8_t *id,
                                          int64_t id_length,
                                          const uint8_t *title,
                                          int64_t title_length) {
  if (content->count >= SIZE_MAX / sizeof(SNAction) - 1)
    return ENOMEM;
  char *key = sn_copy(id, id_length), *label = sn_copy(title, title_length);
  if (!key || !label) {
    free(key);
    free(label);
    return ENOMEM;
  }
  SNAction *actions =
      realloc(content->actions, (content->count + 1) * sizeof(*actions));
  if (!actions) {
    free(key);
    free(label);
    return ENOMEM;
  }
  content->actions = actions;
  content->actions[content->count++] = (SNAction){key, label};
  return 0;
}
void scoop_notification_content_destroy(SNContent *content) {
  if (!content)
    return;
  for (int i = 0; i < 4; ++i)
    free(content->text[i]);
  for (size_t i = 0; i < content->count; ++i) {
    free(content->actions[i].id);
    free(content->actions[i].title);
  }
  free(content->actions);
  free(content);
}
SNContent *sn_content_copy(const SNContent *source) {
  SNContent *copy = scoop_notification_content_create(source->silent).pointer;
  if (!copy)
    return NULL;
  for (int i = 0; i < 4; ++i) {
    const char *text = sn_content_text(source, i);
    if (scoop_notification_content_text(copy, i, (const uint8_t *)text,
                                        (int64_t)strlen(text)))
      goto failed;
  }
  for (size_t i = 0; i < source->count; ++i) {
    const SNAction *action = &source->actions[i];
    if (scoop_notification_content_action(
            copy, (const uint8_t *)action->id, (int64_t)strlen(action->id),
            (const uint8_t *)action->title, (int64_t)strlen(action->title)))
      goto failed;
  }
  return copy;
failed:
  scoop_notification_content_destroy(copy);
  return NULL;
}
