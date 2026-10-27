//! Validates the Rust port against the shared TSV vectors.

use loadpath::Cpm;
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("fixtures")
        .join("cases")
}

#[test]
fn shared_fixtures_stay_consistent() {
    let mut entries: Vec<_> = std::fs::read_dir(fixtures_dir())
        .expect("fixtures directory")
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "tsv"))
        .collect();
    entries.sort();
    assert!(!entries.is_empty(), "no fixtures found");

    for file in entries {
        let text = std::fs::read_to_string(&file).unwrap();
        let mut cpm = Cpm::new();
        let mut expected_path = String::new();
        let mut expected_makespan = 0.0;

        for line in text
            .lines()
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
        {
            let parts: Vec<&str> = line.split('\t').collect();
            match parts[0] {
                "task" => {
                    cpm.add_task(parts[1], parts[2].parse().unwrap()).unwrap();
                }
                "edge" => {
                    cpm.add_dependency(parts[1], parts[2]).unwrap();
                }
                "expect_path" => expected_path = parts[1].to_string(),
                "expect_makespan" => expected_makespan = parts[1].parse().unwrap(),
                other => panic!("bad fixture line in {:?}: {other}", file),
            }
        }

        assert!(
            (cpm.makespan().unwrap() - expected_makespan).abs() < 1e-9,
            "{file:?}"
        );
        assert_eq!(
            cpm.critical_path().unwrap().join(","),
            expected_path,
            "{file:?}"
        );
    }
}
