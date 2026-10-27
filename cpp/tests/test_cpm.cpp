#include <loadpath/cpm.hpp>

#include <fstream>
#include <sstream>
#include <string>
#include <vector>

#include "test_kit.hpp"

using loadpath::Cpm;

namespace {

std::vector<std::string> split(const std::string& line, char delimiter) {
    std::vector<std::string> parts;
    std::string item;
    std::istringstream stream(line);
    while (std::getline(stream, item, delimiter)) {
        parts.push_back(item);
    }
    return parts;
}

std::string join(const std::vector<std::string>& parts, char delimiter) {
    std::string out;
    for (std::size_t i = 0; i < parts.size(); ++i) {
        if (i != 0) {
            out.push_back(delimiter);
        }
        out += parts[i];
    }
    return out;
}

/// Runs every TSV case under the repo-root fixtures directory.
void check_shared_fixtures() {
    for (const char* base : {"fixtures", "../fixtures"}) {
        const std::string dir = std::string(base) + "/cases";
        // Probe with a known case; directories are not portable to open
        // with ifstream, so discovery stays explicit.
        std::ifstream probe(dir + std::string("/parallel-branches.tsv"));
        if (!probe.good()) {
            continue;
        }
        probe.close();
        const char* names[] = {"parallel-branches.tsv", "small-house.tsv"};
        for (const char* name : names) {
            std::ifstream file(dir + "/" + name);
            CHECK(file.good());
            if (!file.good()) {
                continue;
            }
            Cpm cpm;
            std::string expected_path;
            double expected_makespan = 0.0;
            std::string line;
            while (std::getline(file, line)) {
                if (line.empty() || line[0] == '#') {
                    continue;
                }
                const std::vector<std::string> parts = split(line, '\t');
                if (parts[0] == "task") {
                    cpm.add_task(parts[1], std::stod(parts[2]));
                } else if (parts[0] == "edge") {
                    cpm.add_dependency(parts[1], parts[2]);
                } else if (parts[0] == "expect_path") {
                    expected_path = parts[1];
                } else if (parts[0] == "expect_makespan") {
                    expected_makespan = std::stod(parts[1]);
                }
            }
            CHECK_NEAR(cpm.makespan(), expected_makespan, 1e-9);
            CHECK(join(cpm.critical_path(), ',') == expected_path);
        }
        return;
    }
    CHECK(false);  // fixtures directory not found
}

}  // namespace

TEST(textbook_case_matches_closed_form) {
    Cpm cpm;
    cpm.add_task("start", 0)
        .add_task("excavate", 4)
        .add_task("foundation", 6)
        .add_task("walls", 8)
        .add_task("roof", 3)
        .add_task("finish", 5)
        .add_dependency("start", "excavate")
        .add_dependency("excavate", "foundation")
        .add_dependency("foundation", "walls")
        .add_dependency("walls", "roof")
        .add_dependency("roof", "finish");

    CHECK_NEAR(cpm.makespan(), 26.0, 1e-9);
    CHECK(join(cpm.critical_path(), ',') == "start,excavate,foundation,walls,roof,finish");
}

TEST(parallel_branches_pick_the_longer_one) {
    Cpm cpm;
    cpm.add_task("a", 2)
        .add_task("short", 3)
        .add_task("long", 7)
        .add_task("z", 1)
        .add_dependency("a", "short")
        .add_dependency("a", "long")
        .add_dependency("short", "z")
        .add_dependency("long", "z");

    CHECK_NEAR(cpm.makespan(), 10.0, 1e-9);
    CHECK(join(cpm.critical_path(), ',') == "a,long,z");
}

TEST(diamond_slack_is_not_critical) {
    Cpm cpm;
    cpm.add_task("s", 1)
        .add_task("heavy", 9)
        .add_task("light", 2)
        .add_dependency("s", "heavy")
        .add_dependency("s", "light");

    const auto finish = cpm.earliest_finish();
    CHECK_NEAR(finish.at("heavy"), 10.0, 1e-9);
    CHECK_NEAR(finish.at("light"), 3.0, 1e-9);
    CHECK(join(cpm.critical_path(), ',') == "s,heavy");
}

TEST(cycle_is_rejected) {
    Cpm cpm;
    cpm.add_task("a", 1)
        .add_task("b", 2)
        .add_task("c", 3)
        .add_dependency("a", "b")
        .add_dependency("b", "c")
        .add_dependency("c", "a");

    CHECK(throws_any([&] { (void)cpm.earliest_finish(); }));
}

TEST(analysis_is_repeatable) {
    Cpm cpm;
    cpm.add_task("a", 2)
        .add_task("long", 7)
        .add_task("z", 1)
        .add_dependency("a", "long")
        .add_dependency("long", "z");

    const auto first = cpm.critical_path();
    const auto second = cpm.critical_path();
    CHECK(first == second);
}

TEST(shared_fixtures_stay_consistent) {
    check_shared_fixtures();
}

int main() {
    return testkit::run_all();
}
