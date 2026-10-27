package io.github.edxeme.loadpath;

/** Thrown when the task graph contains a dependency cycle. */
public class DependencyCycleException extends RuntimeException {
    public DependencyCycleException(String message) {
        super(message);
    }
}
