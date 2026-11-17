#ifndef SCOOP_WINDOW_CONTACTS_H
#define SCOOP_WINDOW_CONTACTS_H

#include "input.h"
#include <stdlib.h>

typedef struct SWContactEntry {
  SWContactData data;
  struct SWContactEntry *next;
} SWContactEntry;

static inline SWContactEntry *sw_contact_find(SWContactEntry *head,
                                              int64_t id) {
  for (SWContactEntry *entry = head; entry; entry = entry->next)
    if (entry->data.id == id)
      return entry;
  return NULL;
}

static inline SWContactEntry *sw_contact_add(SWContactEntry **head,
                                             SWContactData data) {
  SWContactEntry *entry = (SWContactEntry *)malloc(sizeof(*entry));
  if (entry) {
    *entry = (SWContactEntry){data, *head};
    *head = entry;
  }
  return entry;
}

static inline void sw_contact_remove(SWContactEntry **head,
                                     SWContactEntry *entry) {
  while (*head && *head != entry)
    head = &(*head)->next;
  if (*head) {
    *head = entry->next;
    free(entry);
  }
}

static inline void sw_contacts_clear(SWContactEntry **head) {
  while (*head)
    sw_contact_remove(head, *head);
}

#endif
