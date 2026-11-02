#include <stdexcept>
#include <string>

static std::string initialized = "native C++";
struct Base {
    virtual ~Base() = default;
};
struct Derived : Base {
    std::string value = initialized;
};

extern "C" int scoop_cpp_probe(int n) {
    Derived derived;
    Base *base = &derived;
    try {
        if (n)
            throw std::runtime_error(derived.value);
        return dynamic_cast<Derived *>(base)->value.size();
    } catch (const std::exception &error) {
        return std::string(error.what()).size();
    }
}
