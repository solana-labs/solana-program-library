import { ConverterComponent } from "../components";
/**
 * A plugin that detects interface implementations of functions and
 * properties on classes and links them.
 */
export declare class ImplementsPlugin extends ConverterComponent {
    private resolved;
    private postponed;
    /**
     * Create a new ImplementsPlugin instance.
     */
    initialize(): void;
    /**
     * Mark all members of the given class to be the implementation of the matching interface member.
     */
    private analyzeImplements;
    private analyzeInheritance;
    private onResolveEnd;
    private resolve;
    private tryResolve;
    private doResolve;
    private getExtensionInfo;
    private onSignature;
    /**
     * Responsible for setting the {@link DeclarationReflection.inheritedFrom},
     * {@link DeclarationReflection.overwrites}, and {@link DeclarationReflection.implementationOf}
     * properties on the provided reflection temporarily, these links will be replaced
     * during the resolve step with links which actually point to the right place.
     */
    private onDeclaration;
}
