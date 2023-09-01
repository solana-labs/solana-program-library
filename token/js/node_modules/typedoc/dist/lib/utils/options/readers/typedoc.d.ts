import type { OptionsReader } from "..";
import type { Logger } from "../../loggers";
import type { Options } from "../options";
/**
 * Obtains option values from typedoc.json
 * or typedoc.js (discouraged since ~0.14, don't fully deprecate until API has stabilized)
 */
export declare class TypeDocReader implements OptionsReader {
    /**
     * Should run before the tsconfig reader so that it can specify a tsconfig file to read.
     */
    order: number;
    name: string;
    supportsPackages: boolean;
    /**
     * Read user configuration from a typedoc.json or typedoc.js configuration file.
     */
    read(container: Options, logger: Logger, cwd: string): void;
    /**
     * Read the given options file + any extended files.
     * @param file
     * @param container
     * @param logger
     */
    private readFile;
    /**
     * Search for the configuration file given path
     *
     * @param  path Path to the typedoc.(js|json) file. If path is a directory
     *   typedoc file will be attempted to be found at the root of this path
     * @param logger
     * @return the typedoc.(js|json) file path or undefined
     */
    private findTypedocFile;
}
