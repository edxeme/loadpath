#pragma once

#include <cstddef>
#include <stdexcept>
#include <string>
#include <unordered_map>
#include <unordered_set>
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

/// Start/finish bounds and total float for one task.
struct TaskTimes {
    std::string name;
    double early_start;
    double early_finish;
    double late_start;
    double late_finish;
    double total_float;

    /// A task is critical when its total float is zero.
    [[nodiscard]] bool is_critical() const { return total_float <= 1e-9; }
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

    /// Full schedule in insertion order, from the forward and backward passes.
    [[nodiscard]] std::vector<TaskTimes> schedule() const;

    /// Names of zero-float tasks, in insertion order.
    [[nodiscard]] std::vector<std::string> critical_tasks() const;

    /// Every critical path from a zero-float task with no critical
    /// predecessors, through binding critical edges.
    [[nodiscard]] std::vector<std::vector<std::string>> critical_paths() const;

    /// Mermaid flowchart: heavy arrows on binding critical edges.
    [[nodiscard]] std::string to_mermaid() const;

    [[nodiscard]] std::size_t task_count() const { return tasks_.size(); }

private:
    using FinishMap = std::unordered_map<std::string, double>;

    [[nodiscard]] const std::string* dominating_predecessor(
        const std::string& name,
        const FinishMap& finish) const;

    /// True when the predecessor's earliest finish equals the successor's
    /// earliest start, i.e. the edge pins the successor's start time.
    [[nodiscard]] bool is_binding_edge(const Edge& edge, const FinishMap& finish) const;

    void extend_from(
        const std::string& current,
        const std::unordered_set<std::string>& critical,
        const FinishMap& finish,
        std::unordered_set<std::string>& visited,
        std::vector<std::string>& path,
        std::vector<std::vector<std::string>>& paths) const;

    std::vector<Task> tasks_;
    std::vector<Edge> edges_;
    std::unordered_map<std::string, std::size_t> index_;
};

}  // namespace loadpath
