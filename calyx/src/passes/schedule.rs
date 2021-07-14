use super::math_utilities::get_bit_width_from;
use crate::ir::{self, RRC};
use crate::{build_assignments, guard, structure};
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

    /// Realize the current Schedule as a [ir::Group].
    pub fn realize_schedule(self, builder: &mut ir::Builder) -> RRC<ir::Group> {
        self.validate();
        let final_state = self.last_state();
        let fsm_size = get_bit_width_from(
            final_state + 1, /* represent 0..final_state */
        );
        structure!(builder;
            let fsm = prim std_reg(fsm_size);
            let signal_on = constant(1, 1);
            let last_state = constant(final_state, fsm_size);
            let first_state = constant(0, fsm_size);
        );

        // The compilation group
        let group = builder.add_group("tdcc");

        // Enable assignments
        group.borrow_mut().assignments.extend(
            self.enables
                .into_iter()
                .sorted_by(|(k1, _), (k2, _)| k1.cmp(k2))
                .flat_map(|(state, mut assigns)| {
                    let state_const = builder.add_constant(state, fsm_size);
                    let state_guard =
                        guard!(fsm["out"]).eq(guard!(state_const["out"]));
                    assigns.iter_mut().for_each(|asgn| {
                        asgn.guard.update(|g| g.and(state_guard.clone()))
                    });
                    assigns
                }),
        );

        // Transition assignments
        group.borrow_mut().assignments.extend(
            self.transitions.into_iter().flat_map(|(s, e, guard)| {
                structure!(builder;
                    let end_const = constant(e, fsm_size);
                    let start_const = constant(s, fsm_size);
                );
                let ec_borrow = end_const.borrow();
                let trans_guard =
                    guard!(fsm["out"]).eq(guard!(start_const["out"])) & guard;

                vec![
                    builder.build_assignment(
                        fsm.borrow().get("in"),
                        ec_borrow.get("out"),
                        trans_guard.clone(),
                    ),
                    builder.build_assignment(
                        fsm.borrow().get("write_en"),
                        signal_on.borrow().get("out"),
                        trans_guard,
                    ),
                ]
            }),
        );

        // Done condition for group
        let last_guard = guard!(fsm["out"]).eq(guard!(last_state["out"]));
        let done_assign = builder.build_assignment(
            group.borrow().get("done"),
            signal_on.borrow().get("out"),
            last_guard.clone(),
        );
        group.borrow_mut().assignments.push(done_assign);

        // Cleanup: Add a transition from last state to the first state.
        let mut reset_fsm = build_assignments!(builder;
            fsm["in"] = last_guard ? first_state["out"];
            fsm["write_en"] = last_guard ? signal_on["out"];
        );
        builder
            .component
            .continuous_assignments
            .append(&mut reset_fsm);

        group
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
