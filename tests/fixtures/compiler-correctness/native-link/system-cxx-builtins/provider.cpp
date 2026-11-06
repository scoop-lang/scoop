extern "C" int __popcountdi2(unsigned long long value);

extern "C" int scoop_test_native_popcount(unsigned long long value) {
  try {
    throw value;
  } catch (unsigned long long bits) {
    return __popcountdi2(bits);
  }
}
