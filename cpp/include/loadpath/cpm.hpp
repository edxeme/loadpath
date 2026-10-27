#pragma once

#include <cstddef>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <vector>

namespace loadpath {

/// A task with a non-negative duration.
struct Task {
    std::string name;
    double duration;
};

/// Precedence edge: `before` must finish before `after` starts.
struct Edge {
    std::string before;
    std::string after;
};

/// Thrown when the task graph contains a dependency cycle.
class DependencyCycleError : public std::runtime_error {
public:
    explicit DependencyCycleError()
        : std::runtime_error("dependency cycle detected") {}
};

/// Thrown when referencing a task that was never added.
class UnknownTaskError : public std::runtime_error {
public:
    explicit UnknownTaskError(const std::string& name)
        : std::runtime_error("unknown task: " + name) {}
};

/// Critical-path analysis over a weighted task DAG.
///
/// Insertion order of tasks and edges is preserved so results are stable
/// across runs and across language ports.
class Cpm {
public:
    Cpm& add_task(std::string name, double duration);
    Cpm& add_dependency(std::string before, std::string after);

    /// Earliest finish time per task, keyed by name.
    [[nodiscard]] std::unordered_map<std::string, double> earliest_finish() const;

    /// Total project duration: the largest earliest finish time.
    [[nodiscard]] double makespan() const;

    /// The critical path as task names from start to end.
    [[nodiscard]] std::vector<std::string> critical_path() const;

    [[nodiscard]] std::size_t task_count() const { return tasks_.size(); }

private:
    [[nodiscard]] const std::string* dominating_predecessor(
        const std::string& name,
        const std::unordered_map<std::string, double>& finish) const;

    std::vector<Task> tasks_;
    std::vector<Edge> edges_;
    std::unordered_map<std::string, std::size_t> index_;
};

}  // namespace loadpath
