import ts from "typescript";
import type { Logger } from "./loggers";
export declare function findTsConfigFile(path: string): string | undefined;
export declare function readTsConfig(path: string, logger: Logger): ts.ParsedCommandLine | undefined;
