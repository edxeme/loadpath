#pragma once

// Minimal test harness: registry, CHECK and CHECK_NEAR. No dependencies.

#include <cmath>
#include <cstdio>
#include <stdexcept>
#include <vector>

namespace testkit {

struct TestCase {
    const char* name;
    void (*fn)();
};

inline std::vector<TestCase>& registry() {
    static std::vector<TestCase> registry;
    return registry;
}

inline int& failures() {
    static int count = 0;
    return count;
}

struct Registrar {
    Registrar(const char* name, void (*fn)()) { registry().push_back({name, fn}); }
};

inline void check(bool ok, const char* expr, const char* file, int line) {
    if (!ok) {
        std::printf("  FAIL %s:%d  %s\n", file, line, expr);
        ++failures();
    }
}

inline void check_near(double actual, double expected, double tolerance,
                       const char* expr, const char* file, int line) {
    if (!(std::fabs(actual - expected) <= tolerance)) {
        std::printf("  FAIL %s:%d  %s (got %.9f, want %.9f)\n", file, line, expr, actual, expected);
        ++failures();
    }
}

inline int run_all() {
    for (const TestCase& test : registry()) {
        std::printf("[ RUN ] %s\n", test.name);
        test.fn();
    }
    std::printf("%s: %zu test(s), %d failure(s)\n",
                failures() == 0 ? "PASSED" : "FAILED", registry().size(), failures());
    return failures() == 0 ? 0 : 1;
}

}  // namespace testkit

#define TEST(name)                                                        \
    static void test_##name();                                            \
    static testkit::Registrar reg_##name{#name, &test_##name};            \
    static void test_##name()

#define CHECK(expr) testkit::check(static_cast<bool>(expr), #expr, __FILE__, __LINE__)

#define CHECK_NEAR(actual, expected, tolerance) \
    testkit::check_near((actual), (expected), (tolerance), #actual " ~= " #expected, __FILE__, __LINE__)

template <typename Fn>
bool throws_any(Fn&& fn) {
    try {
        fn();
        return false;
    } catch (const std::exception&) {
        return true;
    }
}
