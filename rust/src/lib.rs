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
    fn analysis_is_repeatable() {
        let cpm = sample();
        let first = cpm.critical_path().unwrap();
        let second = cpm.critical_path().unwrap();
        assert_eq!(first, second);
    }
}
