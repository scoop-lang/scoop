extern int optional_value(void) __attribute__((weak_import));
int facade_value(void) { return optional_value ? optional_value() : 42; }
