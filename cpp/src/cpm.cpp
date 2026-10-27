#include <loadpath/cpm.hpp>

#include <algorithm>
#include <deque>

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
