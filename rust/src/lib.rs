//! Critical-path analysis over a weighted task DAG.
//!
//! Mirrors the Java module: same API shape, same tie-breaking by insertion
//! order, same validation vectors under `fixtures/`.

use std::collections::HashMap;
use std::fmt;

/// A task with a non-negative duration.
#[derive(Debug, Clone)]
pub struct Task {
    pub name: String,
    pub duration: f64,
}

/// Errors raised while building or analysing a task graph.
#[derive(Debug, Clone, PartialEq)]
pub enum CpmError {
    UnknownTask(String),
    DuplicateTask(String),
    Cycle,
    NegativeDuration(String),
}

impl fmt::Display for CpmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CpmError::UnknownTask(name) => write!(f, "unknown task: {name}"),
            CpmError::DuplicateTask(name) => write!(f, "duplicate task: {name}"),
            CpmError::Cycle => write!(f, "dependency cycle detected"),
            CpmError::NegativeDuration(name) => write!(f, "duration must be >= 0: {name}"),
        }
    }
}

impl std::error::Error for CpmError {}

/// Precedence edge: `before` must finish before `after` starts.
#[derive(Debug, Clone)]
pub struct Edge {
    pub before: String,
    pub after: String,
}

/// Start/finish bounds and total float for one task.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskTimes {
    pub name: String,
    pub early_start: f64,
    pub early_finish: f64,
    pub late_start: f64,
    pub late_finish: f64,
    pub total_float: f64,
}

impl TaskTimes {
    /// A task is critical when its total float is zero.
    pub fn is_critical(&self) -> bool {
        self.total_float <= 1e-9
    }
}

/// Critical-path planner over a weighted task DAG.
#[derive(Debug, Default, Clone)]
pub struct Cpm {
    order: Vec<String>,
    durations: HashMap<String, f64>,
    edges: Vec<Edge>,
}

impl Cpm {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a task, preserving insertion order for stable results.
    pub fn add_task(&mut self, name: &str, duration: f64) -> Result<&mut Self, CpmError> {
        if duration < 0.0 {
            return Err(CpmError::NegativeDuration(name.to_string()));
        }
        if self.durations.contains_key(name) {
            return Err(CpmError::DuplicateTask(name.to_string()));
        }
        self.order.push(name.to_string());
        self.durations.insert(name.to_string(), duration);
        Ok(self)
    }

    /// Adds a precedence edge between two known tasks.
    pub fn add_dependency(&mut self, before: &str, after: &str) -> Result<&mut Self, CpmError> {
        if !self.durations.contains_key(before) {
            return Err(CpmError::UnknownTask(before.to_string()));
        }
        if !self.durations.contains_key(after) {
            return Err(CpmError::UnknownTask(after.to_string()));
        }
        self.edges.push(Edge {
            before: before.to_string(),
            after: after.to_string(),
        });
        Ok(self)
    }

    /// Earliest finish time per task, in insertion order.
    pub fn earliest_finish(&self) -> Result<HashMap<String, f64>, CpmError> {
        let mut indegree: HashMap<&str, usize> = HashMap::new();
        for name in &self.order {
            indegree.entry(name).or_insert(0);
        }
        for edge in &self.edges {
            *indegree.entry(edge.after.as_str()).or_insert(0) += 1;
        }

        let mut ready: Vec<&str> = self
            .order
            .iter()
            .map(String::as_str)
            .filter(|name| indegree[name] == 0)
            .collect();
        let mut finish: HashMap<String, f64> = HashMap::new();
        let mut visited = 0;

        while let Some(current) = ready.pop() {
            visited += 1;
            let start = self
                .edges
                .iter()
                .filter(|edge| edge.after == current)
                .filter_map(|edge| finish.get(edge.before.as_str()))
                .copied()
                .fold(0.0_f64, f64::max);
            finish.insert(current.to_string(), start + self.durations[current]);
            for edge in &self.edges {
                if edge.before == current {
                    let entry = indegree.get_mut(edge.after.as_str()).unwrap();
                    *entry -= 1;
                    if *entry == 0 {
                        ready.push(edge.after.as_str());
                    }
                }
            }
        }

        if visited != self.order.len() {
            return Err(CpmError::Cycle);
        }
        Ok(finish)
    }

    /// Total project duration: the largest earliest finish time.
    pub fn makespan(&self) -> Result<f64, CpmError> {
        Ok(self
            .earliest_finish()?
            .values()
            .copied()
            .fold(0.0_f64, f64::max))
    }

    /// The critical path from start to end, walking back through the
    /// dominating predecessor of each task.
    pub fn critical_path(&self) -> Result<Vec<String>, CpmError> {
        let finish = self.earliest_finish()?;

        let mut last: Option<&str> = None;
        let mut makespan = -1.0;
        for name in &self.order {
            if finish[name] > makespan {
                makespan = finish[name];
                last = Some(name);
            }
        }
        let last = last.ok_or(CpmError::Cycle)?;

        let mut path = vec![last.to_string()];
        let mut step = last;
        while let Some(pred) = self.dominating_predecessor(step, &finish) {
            path.push(pred.to_string());
            step = pred;
        }
        path.reverse();
        Ok(path)
    }

    /// Full schedule in insertion order, from the forward and backward passes.
    pub fn schedule(&self) -> Result<Vec<TaskTimes>, CpmError> {
        let finish = self.earliest_finish()?;
        let makespan = finish.values().copied().fold(0.0_f64, f64::max);

        // Backward pass: reverse Kahn over out-degree so every successor of a
        // task carries a late start before the task itself is processed.
        let mut outdegree: HashMap<&str, usize> = HashMap::new();
        for name in &self.order {
            outdegree.insert(name.as_str(), 0);
        }
        for edge in &self.edges {
            *outdegree.get_mut(edge.before.as_str()).unwrap() += 1;
        }

        let mut late_start: HashMap<&str, f64> = HashMap::new();
        let mut ready: Vec<&str> = self
            .order
            .iter()
            .map(String::as_str)
            .filter(|name| outdegree[name] == 0)
            .collect();
        while let Some(current) = ready.pop() {
            let successors: Vec<&Edge> = self
                .edges
                .iter()
                .filter(|edge| edge.before == current)
                .collect();
            let late_finish = if successors.is_empty() {
                makespan
            } else {
                successors
                    .iter()
                    .filter_map(|edge| late_start.get(edge.after.as_str()))
                    .copied()
                    .fold(f64::INFINITY, f64::min)
            };
            late_start.insert(current, late_finish - self.durations[current]);
            for edge in &self.edges {
                if edge.after == current {
                    let entry = outdegree.get_mut(edge.before.as_str()).unwrap();
                    *entry -= 1;
                    if *entry == 0 {
                        ready.push(edge.before.as_str());
                    }
                }
            }
        }

        let mut rows = Vec::new();
        for name in &self.order {
            let early_finish = finish[name];
            let early_start = early_finish - self.durations[name];
            let ls = late_start[name.as_str()];
            rows.push(TaskTimes {
                name: name.clone(),
                early_start,
                early_finish,
                late_start: ls,
                late_finish: ls + self.durations[name],
                total_float: ls - early_start,
            });
        }
        Ok(rows)
    }

    /// Names of zero-float tasks, in insertion order.
    pub fn critical_tasks(&self) -> Result<Vec<String>, CpmError> {
        Ok(self
            .schedule()?
            .into_iter()
            .filter(|row| row.is_critical())
            .map(|row| row.name)
            .collect())
    }

    /// Every critical path from a zero-float task with no critical
    /// predecessors, through binding critical edges (earliest finish of the
    /// predecessor equals earliest start of the successor).
    pub fn critical_paths(&self) -> Result<Vec<Vec<String>>, CpmError> {
        let schedule = self.schedule()?;
        let finish = self.earliest_finish()?;

        let critical: std::collections::HashSet<String> = schedule
            .iter()
            .filter(|row| row.is_critical())
            .map(|row| row.name.clone())
            .collect();

        let mut has_critical_predecessor: std::collections::HashSet<&str> =
            std::collections::HashSet::new();
        for edge in &self.edges {
            if critical.contains(edge.before.as_str())
                && critical.contains(edge.after.as_str())
                && self.is_binding_edge(edge, &finish)
            {
                has_critical_predecessor.insert(edge.after.as_str());
            }
        }

        let mut paths = Vec::new();
        let mut visited: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let mut path = Vec::new();
        for name in &self.order {
            if critical.contains(name.as_str()) && !has_critical_predecessor.contains(name.as_str())
            {
                visited.insert(name.as_str());
                path.push(name.clone());
                self.extend_from(
                    name.as_str(),
                    &critical,
                    &finish,
                    &mut visited,
                    &mut path,
                    &mut paths,
                );
                path.pop();
                visited.remove(name.as_str());
            }
        }
        Ok(paths)
    }

    /// Recursive DFS over binding critical edges: `path` and `visited` grow
    /// in place, and each leaf state is cloned into `paths`.
    fn extend_from<'a>(
        &'a self,
        current: &str,
        critical: &std::collections::HashSet<String>,
        finish: &HashMap<String, f64>,
        visited: &mut std::collections::HashSet<&'a str>,
        path: &mut Vec<String>,
        paths: &mut Vec<Vec<String>>,
    ) {
        let mut extended = false;
        for edge in &self.edges {
            if edge.before == current
                && critical.contains(edge.after.as_str())
                && !visited.contains(edge.after.as_str())
                && self.is_binding_edge(edge, finish)
            {
                visited.insert(edge.after.as_str());
                path.push(edge.after.clone());
                self.extend_from(edge.after.as_str(), critical, finish, visited, path, paths);
                path.pop();
                visited.remove(edge.after.as_str());
                extended = true;
            }
        }
        if !extended {
            paths.push(path.clone());
        }
    }

    /// Mermaid flowchart of the schedule: heavy arrows on binding critical
    /// edges, plain arrows everywhere else.
    pub fn to_mermaid(&self) -> Result<String, CpmError> {
        let schedule = self.schedule()?;
        let finish = self.earliest_finish()?;
        let is_critical: std::collections::HashSet<String> = schedule
            .iter()
            .filter(|row| row.is_critical())
            .map(|row| row.name.clone())
            .collect();

        let label = |name: &str| {
            format!(
                "{}[\"{} ({})\"]",
                name,
                name,
                fmt_number(self.durations[name])
            )
        };

        let mut out = String::from("graph LR\n");
        let mut emitted: std::collections::HashSet<(&str, &str)> = std::collections::HashSet::new();
        for edge in &self.edges {
            if !emitted.insert((edge.before.as_str(), edge.after.as_str())) {
                continue;
            }
            let critical_and_binding = is_critical.contains(edge.before.as_str())
                && is_critical.contains(edge.after.as_str())
                && self.is_binding_edge(edge, &finish);
            let arrow = if critical_and_binding {
                " ==> "
            } else {
                " --> "
            };
            out.push_str("    ");
            out.push_str(&label(&edge.before));
            out.push_str(arrow);
            out.push_str(&label(&edge.after));
            out.push('\n');
        }
        Ok(out)
    }

    /// An edge binds two critical tasks when the predecessor's earliest
    /// finish equals the successor's earliest start.
    fn is_binding_edge(&self, edge: &Edge, finish: &HashMap<String, f64>) -> bool {
        let succ_early_start = finish[&edge.after] - self.durations[&edge.after];
        (finish[&edge.before] - succ_early_start).abs() <= 1e-9
    }

    /// The predecessor with the largest earliest finish, if any.
    fn dominating_predecessor<'a>(
        &'a self,
        name: &str,
        finish: &HashMap<String, f64>,
    ) -> Option<&'a str> {
        let mut best: Option<&str> = None;
        let mut best_finish = -1.0;
        for edge in &self.edges {
            if edge.after == name && finish[&edge.before] > best_finish {
                best_finish = finish[&edge.before];
                best = Some(&edge.before);
            }
        }
        best
    }
}

/// Renders integral values without a decimal point.
fn fmt_number(value: f64) -> String {
    if value == value.trunc() {
        format!("{}", value as i64)
    } else {
        format!("{}", value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Cpm {
        let mut cpm = Cpm::new();
        cpm.add_task("a", 2.0).unwrap();
        cpm.add_task("short", 3.0).unwrap();
        cpm.add_task("long", 7.0).unwrap();
        cpm.add_task("z", 1.0).unwrap();
        cpm.add_dependency("a", "short").unwrap();
        cpm.add_dependency("a", "long").unwrap();
        cpm.add_dependency("short", "z").unwrap();
        cpm.add_dependency("long", "z").unwrap();
        cpm
    }

    #[test]
    fn parallel_branches_pick_the_longer_one() {
        let cpm = sample();
        let finish = cpm.earliest_finish().unwrap();
        assert!((finish["long"] - 9.0).abs() < 1e-9);
        assert!((finish["short"] - 5.0).abs() < 1e-9);
        assert!((cpm.makespan().unwrap() - 10.0).abs() < 1e-9);
        assert_eq!(cpm.critical_path().unwrap(), vec!["a", "long", "z"]);
    }

    #[test]
    fn cycle_is_rejected() {
        let mut cpm = Cpm::new();
        for name in ["a", "b", "c"] {
            cpm.add_task(name, 1.0).unwrap();
        }
        cpm.add_dependency("a", "b").unwrap();
        cpm.add_dependency("b", "c").unwrap();
        cpm.add_dependency("c", "a").unwrap();
        assert_eq!(cpm.earliest_finish(), Err(CpmError::Cycle));
    }

    #[test]
    fn unknown_tasks_are_rejected() {
        let mut cpm = Cpm::new();
        cpm.add_task("a", 1.0).unwrap();
        let err = cpm.add_dependency("a", "ghost").unwrap_err();
        assert_eq!(err, CpmError::UnknownTask("ghost".into()));
    }

    #[test]
    fn backward_pass_assigns_slack_to_the_light_branch() {
        let cpm = sample();
        let schedule = cpm.schedule().unwrap();
        let by_name: HashMap<&str, &TaskTimes> = schedule
            .iter()
            .map(|row| (row.name.as_str(), row))
            .collect();
        assert!((by_name["short"].total_float - 4.0).abs() < 1e-9);
        assert!((by_name["short"].late_finish - 9.0).abs() < 1e-9);
        assert!(by_name["long"].total_float.abs() < 1e-9);
        assert!((by_name["a"].late_finish - 2.0).abs() < 1e-9);
        assert!(by_name["z"].total_float.abs() < 1e-9);
        assert_eq!(cpm.critical_tasks().unwrap(), vec!["a", "long", "z"]);
    }

    #[test]
    fn chain_schedule_is_fully_critical() {
        let mut cpm = Cpm::new();
        cpm.add_task("only", 5.0).unwrap();
        cpm.add_task("then", 3.0).unwrap();
        cpm.add_dependency("only", "then").unwrap();
        for row in cpm.schedule().unwrap() {
            assert!(row.total_float.abs() < 1e-9, "{}", row.name);
        }
        assert!((cpm.makespan().unwrap() - 8.0).abs() < 1e-9);
    }

    #[test]
    fn tied_branches_produce_two_critical_paths() {
        let mut cpm = Cpm::new();
        cpm.add_task("a", 2.0).unwrap();
        cpm.add_task("p", 3.0).unwrap();
        cpm.add_task("q", 3.0).unwrap();
        cpm.add_task("z", 1.0).unwrap();
        cpm.add_dependency("a", "p").unwrap();
        cpm.add_dependency("a", "q").unwrap();
        cpm.add_dependency("p", "z").unwrap();
        cpm.add_dependency("q", "z").unwrap();

        assert!((cpm.makespan().unwrap() - 6.0).abs() < 1e-9);
        assert_eq!(
            cpm.critical_paths().unwrap(),
            vec![vec!["a", "p", "z"], vec!["a", "q", "z"]]
        );
        assert_eq!(cpm.critical_tasks().unwrap(), vec!["a", "p", "q", "z"]);
    }

    #[test]
    fn slack_branches_are_not_critical_paths() {
        let cpm = sample();
        assert_eq!(cpm.critical_paths().unwrap(), vec![vec!["a", "long", "z"]]);
    }

    #[test]
    fn mermaid_marks_binding_critical_edges_as_heavy() {
        let cpm = sample();
        let expected = "graph LR\n    a[\"a (2)\"] --> short[\"short (3)\"]\n    a[\"a (2)\"] ==> long[\"long (7)\"]\n    short[\"short (3)\"] --> z[\"z (1)\"]\n    long[\"long (7)\"] ==> z[\"z (1)\"]\n";
        assert_eq!(cpm.to_mermaid().unwrap(), expected);
    }

    #[test]
    fn analysis_is_repeatable() {
        let cpm = sample();
        let first = cpm.critical_path().unwrap();
        let second = cpm.critical_path().unwrap();
        assert_eq!(first, second);
    }
}
