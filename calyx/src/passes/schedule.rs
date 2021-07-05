use crate::ir;
use ir::IRPrinter;
use itertools::Itertools;
use petgraph::{algo::connected_components, graph::DiGraph};
use std::collections::HashMap;

/// Represents the execution schedule of a control program.
#[derive(Default)]
pub struct Schedule {
    /// Assigments that should be enabled in a given state.
    pub enables: HashMap<u64, Vec<ir::Assignment>>,
    /// Transition from one state to another when the guard is true.
    pub transitions: Vec<(u64, u64, ir::Guard)>,
}

impl Schedule {
    /// Validate that all states are reachable in the transition graph.
    pub fn validate(&self) {
        let graph = DiGraph::<(), u32>::from_edges(
            &self
                .transitions
                .iter()
                .map(|(s, e, _)| (*s as u32, *e as u32))
                .collect::<Vec<_>>(),
        );

        debug_assert!(
            connected_components(&graph) == 1,
            "State transition graph has unreachable states (graph has more than one connected component).");
    }

    /// Return the max state in the transition graph
    pub fn last_state(&self) -> u64 {
        self.transitions
            .iter()
            .max_by_key(|(_, s, _)| s)
            .expect("Schedule::transition is empty!")
            .1
    }

    /// Print out the current schedule
    #[allow(dead_code)]
    pub fn display(&self) {
        self.enables
            .iter()
            .sorted_by(|(k1, _), (k2, _)| k1.cmp(k2))
            .for_each(|(state, assigns)| {
                eprintln!("======== {} =========", state);
                assigns.iter().for_each(|assign| {
                    IRPrinter::write_assignment(
                        assign,
                        0,
                        &mut std::io::stderr(),
                    )
                    .expect("Printing failed!");
                    eprintln!();
                })
            });
        eprintln!("------------");
        self.transitions
            .iter()
            .sorted_by(|(k1, _, _), (k2, _, _)| k1.cmp(k2))
            .for_each(|(i, f, g)| {
                eprintln!("({}, {}): {}", i, f, IRPrinter::guard_str(&g));
            })
    }
}

