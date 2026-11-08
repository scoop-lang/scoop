#ifndef SCOOP_NOTIFICATION_COMMON_H
#define SCOOP_NOTIFICATION_COMMON_H

#include "../desktop.h"

#ifdef __cplusplus
extern "C" {
#endif

enum {
  SN_SETTINGS = 1,
  SN_PERMISSION = 2,
  SN_POST = 3,
  SN_WITHDRAW = 4,
  SN_ACTIVATED = 5,
  SN_CLOSED = 6,
  SN_FAILED = 7
};
enum {
  SN_OK = 0,
  SN_UNSUPPORTED = 1,
  SN_PERMISSION_REQUIRED = 2,
  SN_DISABLED = 3,
  SN_NOT_CONFIGURED = 4,
  SN_UNAVAILABLE = 5,
  SN_INVALID = 6,
  SN_NATIVE_FAILURE = 7
};
enum {
  SN_AVAILABLE = 0,
  SN_NEEDS_PERMISSION = 1,
  SN_SETTING_DISABLED = 2,
  SN_UNCONFIGURED = 3,
  SN_SERVICE_UNAVAILABLE = 4
};
enum {
  SN_ACTIONS = 1,
  SN_REPLACE = 2,
  SN_CAN_WITHDRAW = 4,
  SN_ACTIVATION = 8,
  SN_CLOSE_EVENTS = 16,
  SN_COLD_ACTIVATION = 32
};
typedef struct {
  int32_t kind, detail;
  int64_t request;
  int32_t status, capabilities;
  int64_t code;
} SNData;
typedef struct {
  const uint8_t *bytes;
  int64_t length;
} SNText;
typedef struct SNEvent {
  SNData data;
  char *text[4];
  struct SNEvent *next;
} SNEvent;
typedef struct {
  char *id, *title;
} SNAction;
typedef struct {
  char *text[4];
  SNAction *actions;
  size_t count;
  int32_t silent;
} SNContent;

SNEvent *sn_event(int32_t kind, int64_t request);
void sn_event_set(SNEvent *event, int field, const char *text);
void sn_event_error(SNEvent *event, int status, int64_t code,
                    const char *message);
const char *sn_content_text(const SNContent *content, int field);
SNContent *sn_content_copy(const SNContent *content);
char *sn_copy(const uint8_t *bytes, int64_t length);

SDResult scoop_notification_create(const uint8_t *id, int64_t length);
int64_t scoop_notification_attach(void *pointer, SDCallback callback,
                                  void *context);
void scoop_notification_destroy(void *pointer);
int64_t scoop_notification_settings(void *pointer, int64_t request,
                                    int32_t permission);
int64_t scoop_notification_post(void *pointer, int64_t request,
                                SNContent *content);
int64_t scoop_notification_withdraw(void *pointer, int64_t request,
                                    const uint8_t *id, int64_t length);
SNEvent *scoop_notification_next_event(void *pointer);
SNData scoop_notification_event_data(SNEvent *event);
SNText scoop_notification_event_text(SNEvent *event, int32_t field);
void scoop_notification_event_destroy(SNEvent *event);
SDResult scoop_notification_content_create(int32_t silent);
int64_t scoop_notification_content_text(SNContent *content, int32_t field,
                                        const uint8_t *text, int64_t length);
int64_t scoop_notification_content_action(SNContent *content, const uint8_t *id,
                                          int64_t id_length,
                                          const uint8_t *title,
                                          int64_t title_length);
void scoop_notification_content_destroy(SNContent *content);

#ifdef __cplusplus
}
#endif
#endif
