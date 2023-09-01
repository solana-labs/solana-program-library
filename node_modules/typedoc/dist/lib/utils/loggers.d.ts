import ts from "typescript";
import type { MinimalSourceFile } from "./minimalSourceFile";
/**
 * List of known log levels. Used to specify the urgency of a log message.
 */
export declare enum LogLevel {
    Verbose = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
    None = 4
}
/**
 * A logger that will not produce any output.
 *
 * This logger also serves as the base class of other loggers as it implements
 * all the required utility functions.
 */
export declare class Logger {
    /**
     * How many error messages have been logged?
     */
    errorCount: number;
    /**
     * How many warning messages have been logged?
     */
    warningCount: number;
    private seenErrors;
    private seenWarnings;
    /**
     * The minimum logging level to print.
     */
    level: LogLevel;
    /**
     * Has an error been raised through the log method?
     */
    hasErrors(): boolean;
    /**
     * Has a warning been raised through the log method?
     */
    hasWarnings(): boolean;
    /**
     * Reset the error counter.
     */
    resetErrors(): void;
    /**
     * Reset the warning counter.
     */
    resetWarnings(): void;
    /**
     * Log the given verbose message.
     *
     * @param text  The message that should be logged.
     * @param args  The arguments that should be printed into the given message.
     */
    verbose(text: string): void;
    /** Log the given info message. */
    info(text: string): void;
    /**
     * Log the given warning.
     *
     * @param text  The warning that should be logged.
     * @param args  The arguments that should be printed into the given warning.
     */
    warn(text: string, node?: ts.Node): void;
    warn(text: string, pos: number, file: MinimalSourceFile): void;
    /**
     * Log the given error.
     *
     * @param text  The error that should be logged.
     * @param args  The arguments that should be printed into the given error.
     */
    error(text: string, node?: ts.Node): void;
    error(text: string, pos: number, file: MinimalSourceFile): void;
    /** @internal */
    deprecated(text: string, addStack?: boolean): void;
    /**
     * Print a log message.
     *
     * @param _message The message itself.
     * @param level The urgency of the log message.
     */
    log(_message: string, level: LogLevel): void;
    /**
     * Print the given TypeScript log messages.
     *
     * @param diagnostics  The TypeScript messages that should be logged.
     */
    diagnostics(diagnostics: ReadonlyArray<ts.Diagnostic>): void;
    /**
     * Print the given TypeScript log message.
     *
     * @param diagnostic  The TypeScript message that should be logged.
     */
    diagnostic(diagnostic: ts.Diagnostic): void;
    protected addContext(message: string, _level: LogLevel, ..._args: [ts.Node?] | [number, MinimalSourceFile]): string;
}
/**
 * A logger that outputs all messages to the console.
 */
export declare class ConsoleLogger extends Logger {
    /**
     * Create a new ConsoleLogger instance.
     */
    constructor();
    /**
     * Print a log message.
     *
     * @param message  The message itself.
     * @param level  The urgency of the log message.
     */
    log(message: string, level: Exclude<LogLevel, LogLevel.None>): void;
    protected addContext(message: string, level: Exclude<LogLevel, LogLevel.None>, ...args: [ts.Node?] | [number, MinimalSourceFile]): string;
}
