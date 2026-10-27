package io.github.edxeme.loadpath;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Deque;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

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
        Deque<String> ready = new ArrayDeque<>();
        for (String name : indegree.keySet()) {
            if (indegree.get(name) == 0) {
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
                if (indegree.merge(next, -1, Integer::sum) == 0) {
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
