package io.github.edxeme.loadpath;

import org.junit.jupiter.api.Test;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

class CpmTest {

    @Test
    void textbookCaseMatchesClosedForm() {
        Cpm cpm = new Cpm()
                .addTask(new Task("start", 0))
                .addTask(new Task("excavate", 4))
                .addTask(new Task("foundation", 6))
                .addTask(new Task("walls", 8))
                .addTask(new Task("roof", 3))
                .addTask(new Task("finish", 5))
                .addDependency(new Edge("start", "excavate"))
                .addDependency(new Edge("excavate", "foundation"))
                .addDependency(new Edge("foundation", "walls"))
                .addDependency(new Edge("walls", "roof"))
                .addDependency(new Edge("roof", "finish"));

        assertEquals(26.0, cpm.makespan(), 1e-9);
        assertEquals(List.of("start", "excavate", "foundation", "walls", "roof", "finish"), cpm.criticalPath());
    }

    @Test
    void parallelBranchesPickTheLongerOne() {
        Cpm cpm = new Cpm()
                .addTask(new Task("a", 2))
                .addTask(new Task("short", 3))
                .addTask(new Task("long", 7))
                .addTask(new Task("z", 1))
                .addDependency(new Edge("a", "short"))
                .addDependency(new Edge("a", "long"))
                .addDependency(new Edge("short", "z"))
                .addDependency(new Edge("long", "z"));

        assertEquals(10.0, cpm.makespan(), 1e-9);
        assertEquals(List.of("a", "long", "z"), cpm.criticalPath());
    }

    @Test
    void diamondSlackIsNotCritical() {
        Cpm cpm = new Cpm()
                .addTask(new Task("s", 1))
                .addTask(new Task("heavy", 9))
                .addTask(new Task("light", 2))
                .addDependency(new Edge("s", "heavy"))
                .addDependency(new Edge("s", "light"));

        Map<String, Double> finish = cpm.earliestFinish();
        assertEquals(10.0, finish.get("heavy"), 1e-9);
        assertEquals(3.0, finish.get("light"), 1e-9);
        assertEquals(List.of("s", "heavy"), cpm.criticalPath());
    }

    @Test
    void zeroDurationTasksAreAllowed() {
        Cpm cpm = new Cpm()
                .addTask(new Task("milestone", 0))
                .addTask(new Task("work", 4))
                .addDependency(new Edge("milestone", "work"));

        assertEquals(4.0, cpm.makespan(), 1e-9);
    }

    @Test
    void cycleIsRejected() {
        Cpm cpm = new Cpm()
                .addTask(new Task("a", 1))
                .addTask(new Task("b", 2))
                .addTask(new Task("c", 3))
                .addDependency(new Edge("a", "b"))
                .addDependency(new Edge("b", "c"))
                .addDependency(new Edge("c", "a"));

        assertThrows(DependencyCycleException.class, cpm::earliestFinish);
    }

    @Test
    void sharedFixturesStayConsistent() throws Exception {
        Path fixtures = Path.of("..", "fixtures", "cases");
        try (var files = Files.list(fixtures)) {
            for (Path file : files.filter(p -> p.toString().endsWith(".tsv")).toList()) {
                Cpm cpm = new Cpm();
                String expectedPath = null;
                double expectedMakespan = 0.0;
                for (String line : Files.readAllLines(file)) {
                    if (line.isBlank() || line.startsWith("#")) {
                        continue;
                    }
                    String[] parts = line.split("\t");
                    switch (parts[0]) {
                        case "task" -> cpm.addTask(new Task(parts[1], Double.parseDouble(parts[2])));
                        case "edge" -> cpm.addDependency(new Edge(parts[1], parts[2]));
                        case "expect_path" -> expectedPath = parts[1];
                        case "expect_makespan" -> expectedMakespan = Double.parseDouble(parts[1]);
                        default -> throw new IllegalStateException("bad fixture line: " + line);
                    }
                }
                assertEquals(expectedMakespan, cpm.makespan(), 1e-9, file.toString());
                assertEquals(expectedPath, String.join(",", cpm.criticalPath()), file.toString());
            }
        }
    }
}
