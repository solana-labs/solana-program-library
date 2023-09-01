import { ThemeData } from './themes.test';
import { Resolver } from './resolver';
export declare class ThemeTest {
    private static _readFile;
    private static _normalizeNewLines;
    private readonly EXPECTED_FILE_PATH;
    private readonly tests;
    readonly expected: string;
    readonly testName: string;
    actual: string | null;
    constructor(THEMES_TEST_PATH: string, testFile: string, themeDatas: ThemeData[], resolver: Resolver);
    evaluate(): Promise<any>;
    writeExpected(): void;
}
