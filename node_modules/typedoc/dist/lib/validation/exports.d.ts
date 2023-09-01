import type { ProjectReflection } from "../models";
import type { Logger } from "../utils";
export declare function validateExports(project: ProjectReflection, logger: Logger, intentionallyNotExported: readonly string[]): void;
