package io.github.edxeme.loadpath;

/** Start/finish bounds and total float for one task. */
public record TaskTimes(
        String name,
        double earlyStart,
        double earlyFinish,
        double lateStart,
        double lateFinish,
        double totalFloat) {

    /** A task is critical when its total float is zero. */
    public boolean isCritical() {
        return totalFloat <= 1e-9;
    }
}
