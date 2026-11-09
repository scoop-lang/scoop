#include <unwind.h>

extern "C" _Unwind_Reason_Code __gxx_personality_v0(
    int, _Unwind_Action, _Unwind_Exception_Class, _Unwind_Exception *, _Unwind_Context *);

static int cleanups;
static int calls[2];

struct Guard {
    ~Guard() { ++cleanups; }
};

extern "C" void native_throw() {
    Guard guard;
    throw 41;
}

// Two distinct wrappers plus __gxx_personality_v0 occupy all three compact slots.
extern "C" _Unwind_Reason_Code native_personality_two(
    int version, _Unwind_Action actions, _Unwind_Exception_Class kind,
    _Unwind_Exception *exception, _Unwind_Context *context) {
    ++calls[0];
    return __gxx_personality_v0(version, actions, kind, exception, context);
}

extern "C" _Unwind_Reason_Code native_personality_three(
    int version, _Unwind_Action actions, _Unwind_Exception_Class kind,
    _Unwind_Exception *exception, _Unwind_Context *context) {
    ++calls[1];
    return __gxx_personality_v0(version, actions, kind, exception, context);
}

extern "C" int native_cleanups() { return cleanups; }

extern "C" int native_personalities_called() { return calls[0] > 0 && calls[1] > 0; }
