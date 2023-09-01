import type { Logger } from "../../utils";
export declare const gitIsInstalled: boolean;
/**
 * Stores data of a repository.
 */
export declare class Repository {
    /**
     * The path of this repository on disk.
     */
    path: string;
    /**
     * All files tracked by the repository.
     */
    files: Set<string>;
    urlTemplate: string;
    gitRevision: string;
    /**
     * Create a new Repository instance.
     *
     * @param path  The root path of the repository.
     */
    constructor(path: string, gitRevision: string, urlTemplate: string);
    /**
     * Get the URL of the given file on GitHub or Bitbucket.
     *
     * @param fileName  The file whose URL should be determined.
     * @returns A URL pointing to the web preview of the given file or undefined.
     */
    getURL(fileName: string, line: number): string | undefined;
    /**
     * Try to create a new repository instance.
     *
     * Checks whether the given path is the root of a valid repository and if so
     * creates a new instance of {@link Repository}.
     *
     * @param path  The potential repository root.
     * @returns A new instance of {@link Repository} or undefined.
     */
    static tryCreateRepository(path: string, sourceLinkTemplate: string, gitRevision: string, gitRemote: string, logger: Logger): Repository | undefined;
}
export declare function guessSourceUrlTemplate(remotes: string[]): string | undefined;
