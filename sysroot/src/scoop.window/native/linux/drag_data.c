#include "drag.h"

bool sw_drag_absolute_uri(const char *value) {
  GError *error = NULL;
  GUri *uri = g_uri_parse(value, G_URI_FLAGS_NONE, &error);
  bool valid = uri && g_uri_get_scheme(uri) && *g_uri_get_scheme(uri);
  if (uri)
    g_uri_unref(uri);
  g_clear_error(&error);
  return valid;
}

GPtrArray *sw_drop_decode(const uint8_t *bytes, size_t length, int format) {
  if (length > G_MAXSSIZE ||
      (length &&
       (!bytes || !g_utf8_validate((const char *)bytes, length, NULL))))
    return NULL;
  GPtrArray *items =
      g_ptr_array_new_with_free_func((GDestroyNotify)g_bytes_unref);
  if (format == 1) {
    g_ptr_array_add(items, g_bytes_new(bytes, length));
    return items;
  }
  if (!length) {
    g_ptr_array_unref(items);
    return NULL;
  }
  char *text = g_strndup((const char *)bytes, length);
  char **uris = g_uri_list_extract_uris(text);
  g_free(text);
  size_t count = g_strv_length(uris);
  bool valid = count > 0 && (format == 3 || (format == 2 && count == 1));
  for (size_t n = 0; valid && n < count; ++n) {
    if (format == 2) {
      valid = sw_drag_absolute_uri(uris[n]);
      if (valid)
        g_ptr_array_add(items, g_bytes_new(uris[n], strlen(uris[n])));
    } else {
      GError *error = NULL;
      char *host = NULL;
      char *path = g_filename_from_uri(uris[n], &host, &error);
      valid = path && g_path_is_absolute(path) &&
              (!host || !*host || g_ascii_strcasecmp(host, "localhost") == 0);
      if (valid)
        g_ptr_array_add(items, g_bytes_new(path, strlen(path)));
      g_free(path);
      g_free(host);
      g_clear_error(&error);
    }
  }
  g_strfreev(uris);
  if (!valid) {
    g_ptr_array_unref(items);
    return NULL;
  }
  return items;
}

int64_t scoop_window_drop_item_count(GPtrArray *payload) {
  return payload->len;
}
SWDropBytes scoop_window_drop_text(GPtrArray *payload, int64_t index) {
  if (index < 0 || index >= payload->len)
    return (SWDropBytes){NULL, -1};
  gsize length;
  const void *bytes =
      g_bytes_get_data(g_ptr_array_index(payload, index), &length);
  return (SWDropBytes){bytes, (int64_t)length};
}
SWDropBytes scoop_window_drop_path(GPtrArray *payload, int64_t index) {
  return scoop_window_drop_text(payload, index);
}
