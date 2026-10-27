package io.github.edxeme.loadpath;

/** A task with a non-negative duration. */
public record Task(String name, double duration) {
    public Task {
        if (duration < 0) {
            throw new IllegalArgumentException("duration must be >= 0: " + name);
        }
    }
}
