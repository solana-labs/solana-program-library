import { DeclarationReflection } from "../../models/reflections/index";
import { ConverterComponent } from "../components";
/**
 * Responsible for adding `implementedBy` / `implementedFrom`
 */
export declare class TypePlugin extends ConverterComponent {
    reflections: Set<DeclarationReflection>;
    /**
     * Create a new TypeHandler instance.
     */
    initialize(): void;
    private onRevive;
    private onResolve;
    private resolve;
    private postpone;
    private onResolveEnd;
    private finishResolve;
}
