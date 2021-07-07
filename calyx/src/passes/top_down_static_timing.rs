use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    IRPrinter, LibrarySignatures,
};
use crate::{build_assignments, guard, structure};
use itertools::Itertools;
use std::collections::HashMap;

/// A range of FSM states.
type Range = (u64, u64);

/// A schedule keeps track of two things:
/// 1. `enables`: Specifies which groups are active during a range of
///     FSM states.
/// 2. `transitions`: Transitions for the FSM registers. A static FSM normally
///    transitions from `state` to `state + 1`. However, special transitions
///    are needed for loops, conditionals, and reseting the FSM.
#[derive(Default)]
struct Schedule {
    enables: HashMap<Range, Vec<ir::Assignment>>,
    transitions: Vec<(u64, u64, ir::Guard)>,
}

impl Schedule {
    #[allow(dead_code)]
    fn display(&self) {
        self.enables
            .iter()
            .sorted_by(|(k1, _), (k2, _)| k1.cmp(k2))
            .for_each(|(state, assigns)| {
                eprintln!("======== ({}, {}) =========", state.0, state.1);
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

fn calculate_states(
    con: &ir::Control,
    // The current state
    cur_state: u64,
    // Additional guard for this condition.
    pre_guard: &ir::Guard,
    // Current schedule.
    schedule: &mut Schedule,
    // Component builder
    builder: &mut ir::Builder,
) -> u64 {
    match con {
        ir::Control::Enable(e) => {
            enable_calculate_states(e, cur_state, pre_guard, schedule, builder)
        }
        ir::Control::Seq(s) => {
            seq_calculate_states(s, cur_state, pre_guard, schedule, builder)
        }
        _ => panic!("Not yet implemented!"),
    }
}

fn seq_calculate_states(
    con: &ir::Seq,
    cur_state: u64,
    pre_guard: &ir::Guard,
    schedule: &mut Schedule,
    builder: &mut ir::Builder,
) -> u64 {
    let mut cur = cur_state;
    for stmt in &con.stmts {
        cur = calculate_states(stmt, cur, pre_guard, schedule, builder)
    }
    cur
}

/// Compiled to:
/// ```
/// group[go] = (fsm >= cur_start & fsm < cur_state + static) & pre_guard ? 1'd1;
/// ```
fn enable_calculate_states(
    con: &ir::Enable,
    // The current state
    cur_state: u64,
    // Additional guard for this condition.
    pre_guard: &ir::Guard,
    // Current schedule.
    schedule: &mut Schedule,
    // Component builder
    builder: &mut ir::Builder,
) -> u64 {
    let time = con
        .attributes
        .get("static")
        .expect("`static` annotation missing");
    let range = (cur_state, cur_state + time);
    let group = &con.group;
    structure!(builder;
        let signal_on = constant(1, 1);
    );
    let mut assigns = build_assignments!(builder;
        group["go"] = pre_guard ? signal_on["out"];
    );
    schedule
        .enables
        .entry(range)
        .or_default()
        .append(&mut assigns);
    cur_state + time
}

#[derive(Default)]
pub struct TopDownStaticTiming;

impl Named for TopDownStaticTiming {
    fn name() -> &'static str {
        "top-down-st"
    }

    fn description() -> &'static str {
        "Top-down latency-sensitive compilation for removing control constructs"
    }
}

impl Visitor for TopDownStaticTiming {
    fn start(
        &mut self,
        comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        // Do not try to compile an enable or empty control
        if matches!(
            *comp.control.borrow(),
            ir::Control::Enable(..) | ir::Control::Empty(..)
        ) {
            return Ok(Action::Stop);
        }

        Ok(Action::Continue)
    }

    fn start_seq(
        &mut self,
        s: &mut ir::Seq,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
    ) -> VisResult {
        if s.attributes.has("static") {
            let mut schedule = Schedule::default();
            let mut builder = ir::Builder::new(comp, sigs);
            seq_calculate_states(
                &*s,
                0,
                &ir::Guard::True,
                &mut schedule,
                &mut builder,
            );
            schedule.display();
        }
        Ok(Action::Continue)
    }
}
