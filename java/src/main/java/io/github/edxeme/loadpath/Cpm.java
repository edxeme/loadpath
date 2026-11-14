package io.github.edxeme.loadpath;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Deque;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;

/**
 * Critical-path analysis over a weighted task DAG.
 *
 * <p>Insertion order of tasks and edges is preserved so results are stable
 * across runs and across language ports.
 */
public final class Cpm {

    private final Map<String, Task> tasks = new LinkedHashMap<>();
    private final List<Edge> edges = new ArrayList<>();
    private final Map<String, List<String>> successors = new HashMap<>();
    private final Map<String, Integer> indegree = new HashMap<>();

    public Cpm addTask(Task task) {
        if (tasks.putIfAbsent(task.name(), task) != null) {
            throw new IllegalArgumentException("duplicate task: " + task.name());
        }
        successors.put(task.name(), new ArrayList<>());
        indegree.put(task.name(), 0);
        return this;
    }

    public Cpm addDependency(Edge edge) {
        requireTask(edge.before());
        requireTask(edge.after());
        edges.add(edge);
        successors.get(edge.before()).add(edge.after());
        indegree.merge(edge.after(), 1, Integer::sum);
        return this;
    }

    /** Earliest finish time per task, in topological order. */
    public Map<String, Double> earliestFinish() {
        Map<String, Double> finish = new LinkedHashMap<>();
        Map<String, Integer> pending = new HashMap<>(indegree);
        Deque<String> ready = new ArrayDeque<>();
        for (String name : pending.keySet()) {
            if (pending.get(name) == 0) {
                ready.add(name);
            }
        }
        int visited = 0;
        while (!ready.isEmpty()) {
            String current = ready.poll();
            visited++;
            double start = 0.0;
            for (Edge edge : edges) {
                if (edge.after().equals(current)) {
                    start = Math.max(start, finish.get(edge.before()));
                }
            }
            finish.put(current, start + tasks.get(current).duration());
            for (String next : successors.get(current)) {
                if (pending.merge(next, -1, Integer::sum) == 0) {
                    ready.add(next);
                }
            }
        }
        if (visited != tasks.size()) {
            throw new DependencyCycleException("dependency cycle detected");
        }
        return finish;
    }

    /**
     * The critical path as a list of task names from start to end, walking
     * backwards from the task with the largest earliest finish.
     */
    public List<String> criticalPath() {
        Map<String, Double> finish = earliestFinish();
        String last = null;
        double makespan = -1.0;
        for (Map.Entry<String, Double> entry : finish.entrySet()) {
            if (entry.getValue() > makespan) {
                makespan = entry.getValue();
                last = entry.getKey();
            }
        }
        List<String> path = new ArrayList<>();
        String step = last;
        while (step != null) {
            path.add(step);
            step = dominatingPredecessor(step, finish);
        }
        return path.reversed();
    }

    /** Total project duration: the largest earliest finish time. */
    public double makespan() {
        return earliestFinish().values().stream().mapToDouble(Double::doubleValue).max().orElse(0.0);
    }

    /**
     * Full schedule in insertion order: early and late start/finish per task,
     * from the forward and backward passes.
     */
    public List<TaskTimes> schedule() {
        Map<String, Double> finish = earliestFinish();
        double makespan = 0.0;
        for (double value : finish.values()) {
            makespan = Math.max(makespan, value);
        }

        // Backward pass: reverse Kahn over out-degree so every successor of a
        // task carries a late start before the task itself is processed.
        Map<String, Integer> outdegree = new HashMap<>();
        for (String name : tasks.keySet()) {
            outdegree.put(name, successors.get(name).size());
        }
        Map<String, Double> lateStart = new HashMap<>();
        Deque<String> ready = new ArrayDeque<>();
        for (String name : outdegree.keySet()) {
            if (outdegree.get(name) == 0) {
                ready.add(name);
            }
        }
        while (!ready.isEmpty()) {
            String current = ready.poll();
            double lateFinish = makespan;
            for (String next : successors.get(current)) {
                lateFinish = Math.min(lateFinish, lateStart.get(next));
            }
            lateStart.put(current, lateFinish - tasks.get(current).duration());
            for (Edge edge : edges) {
                if (edge.after().equals(current) && outdegree.merge(edge.before(), -1, Integer::sum) == 0) {
                    ready.add(edge.before());
                }
            }
        }

        List<TaskTimes> rows = new ArrayList<>();
        for (Task task : tasks.values()) {
            double earlyFinish = finish.get(task.name());
            double earlyStart = earlyFinish - task.duration();
            double ls = lateStart.get(task.name());
            rows.add(new TaskTimes(task.name(), earlyStart, earlyFinish,
                    ls, ls + task.duration(), ls - earlyStart));
        }
        return rows;
    }

    /** Names of zero-float tasks, in insertion order. */
    public List<String> criticalTasks() {
        List<String> names = new ArrayList<>();
        for (TaskTimes row : schedule()) {
            if (row.isCritical()) {
                names.add(row.name());
            }
        }
        return names;
    }

    /**
     * Every critical path from a zero-float task with no critical
     * predecessors to a zero-float task with no critical successors.
     * An edge binds two critical tasks when it is the only way to keep
     * them edge-to-edge on a schedule, i.e. the predecessor's earliest
     * finish equals the successor's earliest start.
     */
    public List<List<String>> criticalPaths() {
        Map<String, TaskTimes> times = new LinkedHashMap<>();
        for (TaskTimes row : schedule()) {
            times.put(row.name(), row);
        }

        Set<String> critical = new LinkedHashSet<>();
        for (TaskTimes row : times.values()) {
            if (row.isCritical()) {
                critical.add(row.name());
            }
        }

        Set<String> hasCriticalPredecessor = new LinkedHashSet<>();
        for (Edge edge : edges) {
            if (critical.contains(edge.before()) && critical.contains(edge.after())
                    && isBinding(edge, times)) {
                hasCriticalPredecessor.add(edge.after());
            }
        }

        List<List<String>> paths = new ArrayList<>();
        for (String name : critical) {
            if (!hasCriticalPredecessor.contains(name)) {
                extendPath(name, critical, times, new LinkedHashSet<>(List.of(name)),
                        new ArrayList<>(List.of(name)), paths);
            }
        }
        return paths;
    }

    private void extendPath(String current, Set<String> critical, Map<String, TaskTimes> times,
                            Set<String> visited, List<String> path, List<List<String>> paths) {
        boolean extended = false;
        for (Edge edge : edges) {
            if (edge.before().equals(current) && critical.contains(edge.after())
                    && !visited.contains(edge.after()) && isBinding(edge, times)) {
                visited.add(edge.after());
                path.add(edge.after());
                extendPath(edge.after(), critical, times, visited, path, paths);
                path.remove(path.size() - 1);
                visited.remove(edge.after());
                extended = true;
            }
        }
        if (!extended) {
            paths.add(new ArrayList<>(path));
        }
    }

    private boolean isBinding(Edge edge, Map<String, TaskTimes> times) {
        double succEarlyStart = times.get(edge.after()).earlyFinish()
                - tasks.get(edge.after()).duration();
        return Math.abs(times.get(edge.before()).earlyFinish() - succEarlyStart) <= 1e-9;
    }

    /**
     * Mermaid flowchart of the schedule: heavy arrows on binding critical
     * edges, plain arrows everywhere else.
     */
    public String toMermaid() {
        Map<String, TaskTimes> times = new LinkedHashMap<>();
        Set<String> critical = new LinkedHashSet<>();
        for (TaskTimes row : schedule()) {
            times.put(row.name(), row);
            if (row.isCritical()) {
                critical.add(row.name());
            }
        }
        StringBuilder out = new StringBuilder("graph LR\n");
        Set<String> emitted = new LinkedHashSet<>();
        for (Edge edge : edges) {
            String key = edge.before() + " -> " + edge.after();
            if (!emitted.add(key)) {
                continue;
            }
            String arrow = critical.contains(edge.before()) && critical.contains(edge.after())
                    && isBinding(edge, times) ? " ==> " : " --> ";
            out.append("    ").append(edge.before())
                    .append("[\"").append(edge.before())
                    .append(" (").append(trim(tasks.get(edge.before()).duration()))
                    .append(")\"]")
                    .append(arrow)
                    .append(edge.after())
                    .append("[\"").append(edge.after())
                    .append(" (").append(trim(tasks.get(edge.after()).duration()))
                    .append(")\"]")
                    .append('\n');
        }
        return out.toString();
    }

    private static String trim(double value) {
        return value == Math.rint(value)
                ? String.valueOf((long) value)
                : String.valueOf(value);
    }

    private String dominatingPredecessor(String name, Map<String, Double> finish) {
        String best = null;
        double bestFinish = -1.0;
        for (Edge edge : edges) {
            if (edge.after().equals(name) && finish.get(edge.before()) > bestFinish) {
                bestFinish = finish.get(edge.before());
                best = edge.before();
            }
        }
        return best;
    }

    private void requireTask(String name) {
        if (!tasks.containsKey(name)) {
            throw new IllegalArgumentException("unknown task: " + name);
        }
    }
}
