//! Defines the default passes available to [PassManager].
use crate::passes::{
    ClkInsertion, CollapseControl, CompileControl, CompileEmpty, CompileInvoke,
    ComponentInterface, DeadCellRemoval, Externalize, GoInsertion,
    GuardCanonical, InferStaticTiming, Inliner, MergeAssign, MinimizeRegs,
    Papercut, ParToSeq, RegisterUnsharing, RemoveCombGroups, ResetInsertion,
    ResourceSharing, SimplifyGuards, StaticTiming, SynthesisPapercut,
    TopDownCompileControl, TopDownStaticTiming, WellFormed,
};
use crate::{
    errors::FutilResult,
    ir::traversal::{Named, Visitor},
    pass_manager::PassManager,
    register_alias, register_pass,
};

impl PassManager {
    pub fn default_passes() -> FutilResult<Self> {
        // Construct the pass manager and register all passes.
        let mut pm = PassManager::default();

        // Register passes.
        register_pass!(pm, WellFormed);
        register_pass!(pm, StaticTiming);
        register_pass!(pm, CompileControl);
        register_pass!(pm, CompileInvoke);
        register_pass!(pm, GoInsertion);
        register_pass!(pm, ComponentInterface);
        register_pass!(pm, Inliner);
        register_pass!(pm, Externalize);
        register_pass!(pm, CollapseControl);
        register_pass!(pm, CompileEmpty);
        register_pass!(pm, Papercut);
        register_pass!(pm, ClkInsertion);
        register_pass!(pm, ResetInsertion);
        register_pass!(pm, ResourceSharing);
        register_pass!(pm, DeadCellRemoval);
        register_pass!(pm, MinimizeRegs);
        register_pass!(pm, InferStaticTiming);
        register_pass!(pm, SimplifyGuards);
        register_pass!(pm, MergeAssign);
        register_pass!(pm, TopDownCompileControl);
        register_pass!(pm, SynthesisPapercut);
        register_pass!(pm, RegisterUnsharing);
        register_pass!(pm, GuardCanonical);
        register_pass!(pm, ParToSeq);
        register_pass!(pm, TopDownStaticTiming);
        register_pass!(pm, RemoveCombGroups);

        register_alias!(pm, "validate", [WellFormed, Papercut, GuardCanonical]);
        register_alias!(
            pm,
            "pre-opt",
            [
                GuardCanonical,
                InferStaticTiming,
                CollapseControl,
                ResourceSharing,
                MinimizeRegs,
                CompileInvoke,
            ]
        );
        register_alias!(
            pm,
            "compile",
            [CompileEmpty, StaticTiming, TopDownCompileControl]
        );
        register_alias!(pm, "post-opt", [DeadCellRemoval]);
        register_alias!(
            pm,
            "lower",
            [
                GoInsertion,
                ComponentInterface,
                Inliner,
                ClkInsertion,
                ResetInsertion,
                MergeAssign,
            ]
        );

        // Register aliases
        register_alias!(
            pm,
            "all",
            ["validate", "pre-opt", "compile", "post-opt", "lower",]
        );

        register_alias!(
            pm,
            "external",
            [
                "validate",
                "pre-opt",
                "compile",
                "post-opt",
                "lower",
                Externalize,
            ]
        );

        register_alias!(pm, "none", []);

        Ok(pm)
    }
}
