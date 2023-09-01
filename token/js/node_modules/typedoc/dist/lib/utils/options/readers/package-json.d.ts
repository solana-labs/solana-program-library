import type { OptionsReader } from "..";
import type { Logger } from "../../loggers";
import type { Options } from "../options";
export declare class PackageJsonReader implements OptionsReader {
    order: number;
    supportsPackages: boolean;
    name: string;
    read(container: Options, logger: Logger, cwd: string): void;
}
