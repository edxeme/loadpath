package io.github.edxeme.loadpath;

/** Precedence edge: {@code before} must finish before {@code after} starts. */
public record Edge(String before, String after) {}
