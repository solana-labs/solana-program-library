import { Registry } from '../main';
import { IRawTheme } from '../theme';
export interface ThemeData {
    themeName: string;
    theme: IRawTheme;
    registry: Registry;
}
