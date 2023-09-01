/**
 * Helper class that determines the common base path of a set of files.
 *
 * In the first step all files must be passed to {@link add}. Afterwards {@link trim}
 * can be used to retrieve the shortest path relative to the determined base path.
 */
export declare class BasePath {
    /**
     * List of known base paths.
     */
    private basePaths;
    /**
     * Add the given file path to this set of base paths.
     *
     * @param fileName  The absolute filename that should be added to the base path.
     */
    add(fileName: string): void;
    /**
     * Trim the given filename by the determined base paths.
     *
     * @param fileName  The absolute filename that should be trimmed.
     * @returns The trimmed version of the filename.
     */
    trim(fileName: string): string;
    /**
     * Reset this instance, ignore all paths already passed to {@link add}.
     */
    reset(): void;
    /**
     * Normalize the given path.
     *
     * @param path  The path that should be normalized.
     * @returns Normalized version of the given path.
     */
    static normalize(path: string): string;
}
