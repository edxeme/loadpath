#include <loadpath/cpm.hpp>

#include <algorithm>
#include <deque>
#include <sstream>

namespace loadpath {

Cpm& Cpm::add_task(std::string name, double duration) {
    if (duration < 0.0) {
        throw std::invalid_argument("duration must be >= 0: " + name);
    }
    if (index_.count(name) != 0) {
        throw std::invalid_argument("duplicate task: " + name);
    }
    index_.emplace(name, tasks_.size());
    tasks_.push_back(Task{std::move(name), duration});
    return *this;
}

Cpm& Cpm::add_dependency(std::string before, std::string after) {
    if (index_.count(before) == 0) {
        throw UnknownTaskError(before);
    }
    if (index_.count(after) == 0) {
        throw UnknownTaskError(after);
    }
    edges_.push_back(Edge{std::move(before), std::move(after)});
    return *this;
}

std::unordered_map<std::string, double> Cpm::earliest_finish() const {
    std::unordered_map<std::string, int> indegree;
    for (const Task& task : tasks_) {
        indegree.emplace(task.name, 0);
    }
    for (const Edge& edge : edges_) {
        indegree[edge.after] += 1;
    }

    // Work on a copy so the analysis stays repeatable.
    std::unordered_map<std::string, int> pending = indegree;

    std::deque<std::string> ready;
    for (const Task& task : tasks_) {
        if (pending[task.name] == 0) {
            ready.push_back(task.name);
        }
    }

    std::unordered_map<std::string, double> finish;
    std::size_t visited = 0;
    while (!ready.empty()) {
        const std::string current = ready.front();
        ready.pop_front();
        ++visited;

        double start = 0.0;
        for (const Edge& edge : edges_) {
            if (edge.after == current) {
                start = std::max(start, finish[edge.before]);
            }
        }
        finish[current] = start + tasks_[index_.at(current)].duration;

        for (const Edge& edge : edges_) {
            if (edge.before == current && --pending[edge.after] == 0) {
                ready.push_back(edge.after);
            }
        }
    }

    if (visited != tasks_.size()) {
        throw DependencyCycleError();
    }
    return finish;
}

double Cpm::makespan() const {
    double best = 0.0;
    for (const auto& [name, value] : earliest_finish()) {
        best = std::max(best, value);
    }
    return best;
}

std::vector<std::string> Cpm::critical_path() const {
    const std::unordered_map<std::string, double> finish = earliest_finish();

    const std::string* last = nullptr;
    double makespan = -1.0;
    for (const Task& task : tasks_) {
        const double value = finish.at(task.name);
        if (value > makespan) {
            makespan = value;
            last = &task.name;
        }
    }

    std::vector<std::string> path;
    const std::string* step = last;
    while (step != nullptr) {
        path.push_back(*step);
        step = dominating_predecessor(*step, finish);
    }
    std::reverse(path.begin(), path.end());
    return path;
}

std::vector<TaskTimes> Cpm::schedule() const {
    const std::unordered_map<std::string, double> finish = earliest_finish();
    double makespan = 0.0;
    for (const auto& [name, value] : finish) {
        makespan = std::max(makespan, value);
    }

    // Backward pass: reverse Kahn over out-degree so every successor of a
    // task carries a late start before the task itself is processed.
    std::unordered_map<std::string, int> outdegree;
    for (const Task& task : tasks_) {
        outdegree.emplace(task.name, 0);
    }
    for (const Edge& edge : edges_) {
        outdegree[edge.before] += 1;
    }

    std::unordered_map<std::string, double> late_start;
    std::deque<std::string> ready;
    for (const Task& task : tasks_) {
        if (outdegree[task.name] == 0) {
            ready.push_back(task.name);
        }
    }
    while (!ready.empty()) {
        const std::string current = ready.front();
        ready.pop_front();

        double late_finish = makespan;
        for (const Edge& edge : edges_) {
            if (edge.before == current) {
                late_finish = std::min(late_finish, late_start.at(edge.after));
            }
        }
        late_start[current] = late_finish - tasks_.at(index_.at(current)).duration;

        for (const Edge& edge : edges_) {
            if (edge.after == current && --outdegree[edge.before] == 0) {
                ready.push_back(edge.before);
            }
        }
    }

    std::vector<TaskTimes> rows;
    for (const Task& task : tasks_) {
        const double early_finish = finish.at(task.name);
        const double early_start = early_finish - task.duration;
        const double ls = late_start.at(task.name);
        rows.push_back(TaskTimes{task.name, early_start, early_finish,
                                 ls, ls + task.duration, ls - early_start});
    }
    return rows;
}

std::vector<std::string> Cpm::critical_tasks() const {
    std::vector<std::string> names;
    for (const TaskTimes& row : schedule()) {
        if (row.is_critical()) {
            names.push_back(row.name);
        }
    }
    return names;
}

std::vector<std::vector<std::string>> Cpm::critical_paths() const {
    const std::vector<TaskTimes> rows = schedule();
    const FinishMap finish = earliest_finish();

    std::unordered_set<std::string> critical;
    for (const TaskTimes& row : rows) {
        if (row.is_critical()) {
            critical.insert(row.name);
        }
    }

    std::unordered_set<std::string> has_critical_predecessor;
    for (const Edge& edge : edges_) {
        if (critical.count(edge.before) != 0 && critical.count(edge.after) != 0
            && is_binding_edge(edge, finish)) {
            has_critical_predecessor.insert(edge.after);
        }
    }

    std::vector<std::vector<std::string>> paths;
    std::unordered_set<std::string> visited;
    std::vector<std::string> path;
    for (const Task& task : tasks_) {
        if (critical.count(task.name) != 0
            && has_critical_predecessor.count(task.name) == 0) {
            visited.insert(task.name);
            path.push_back(task.name);
            extend_from(task.name, critical, finish, visited, path, paths);
            path.pop_back();
            visited.erase(task.name);
        }
    }
    return paths;
}

void Cpm::extend_from(
    const std::string& current,
    const std::unordered_set<std::string>& critical,
    const FinishMap& finish,
    std::unordered_set<std::string>& visited,
    std::vector<std::string>& path,
    std::vector<std::vector<std::string>>& paths) const {
    bool extended = false;
    for (const Edge& edge : edges_) {
        if (edge.before == current && critical.count(edge.after) != 0
            && visited.count(edge.after) == 0 && is_binding_edge(edge, finish)) {
            visited.insert(edge.after);
            path.push_back(edge.after);
            extend_from(edge.after, critical, finish, visited, path, paths);
            path.pop_back();
            visited.erase(edge.after);
            extended = true;
        }
    }
    if (!extended) {
        paths.push_back(path);
    }
}

bool Cpm::is_binding_edge(const Edge& edge, const FinishMap& finish) const {
    const double succ_early_start =
        finish.at(edge.after) - tasks_.at(index_.at(edge.after)).duration;
    return std::fabs(finish.at(edge.before) - succ_early_start) <= 1e-9;
}

namespace {

std::string fmt_number(double value) {
    std::ostringstream out;
    if (value == static_cast<double>(static_cast<long long>(value))) {
        out << static_cast<long long>(value);
    } else {
        out << value;
    }
    return out.str();
}

}  // namespace

std::string Cpm::to_mermaid() const {
    const std::vector<TaskTimes> rows = schedule();
    const FinishMap finish = earliest_finish();
    std::unordered_set<std::string> critical;
    for (const TaskTimes& row : rows) {
        if (row.is_critical()) {
            critical.insert(row.name);
        }
    }

    std::string out = "graph LR\n";
    std::unordered_set<std::string> emitted;
    for (const Edge& edge : edges_) {
        const std::string key = edge.before + " -> " + edge.after;
        if (!emitted.insert(key).second) {
            continue;
        }
        const bool heavy = critical.count(edge.before) != 0
            && critical.count(edge.after) != 0 && is_binding_edge(edge, finish);
        const std::string arrow = heavy ? " ==> " : " --> ";
        out += "    ";
        out += edge.before;
        out += "[\"";
        out += edge.before;
        out += " (";
        out += fmt_number(tasks_.at(index_.at(edge.before)).duration);
        out += ")\"]";
        out += arrow;
        out += edge.after;
        out += "[\"";
        out += edge.after;
        out += " (";
        out += fmt_number(tasks_.at(index_.at(edge.after)).duration);
        out += ")\"]";
        out += '\n';
    }
    return out;
}

const std::string* Cpm::dominating_predecessor(
    const std::string& name,
    const std::unordered_map<std::string, double>& finish) const {
    const std::string* best = nullptr;
    double best_finish = -1.0;
    for (const Edge& edge : edges_) {
        if (edge.after == name) {
            const double value = finish.at(edge.before);
            if (value > best_finish) {
                best_finish = value;
                best = &edge.before;
            }
        }
    }
    return best;
}

}  // namespace loadpath
