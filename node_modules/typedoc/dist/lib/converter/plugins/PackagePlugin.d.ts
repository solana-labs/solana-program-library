import { ConverterComponent } from "../components";
import { EntryPointStrategy } from "../../utils";
/**
 * A handler that tries to find the package.json and readme.md files of the
 * current project.
 */
export declare class PackagePlugin extends ConverterComponent {
    readme: string;
    entryPointStrategy: EntryPointStrategy;
    entryPoints: string[];
    includeVersion: boolean;
    /**
     * The file name of the found readme.md file.
     */
    private readmeFile?;
    /**
     * Contents of the readme.md file discovered, if any
     */
    private readmeContents?;
    /**
     * Contents of package.json for the active project
     */
    private packageJson?;
    initialize(): void;
    private onRevive;
    private onBegin;
    private onBeginResolve;
    private addEntries;
}
