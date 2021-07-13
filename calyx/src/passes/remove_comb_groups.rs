use itertools::Itertools;

use crate::errors::Error;
use crate::ir::{
    self,
    traversal::{Action, Named, VisResult, Visitor},
    LibrarySignatures, RRC,
};
use crate::{guard, structure};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Default)]
/// Transforms combinational groups, which have a constant done condition,
/// into proper groups by registering the values read from the ports of cells
/// used within the combinational group.
///
/// # Example
/// ```
/// group comb_cond<"static"=0> {
///     lt.right = 32'd10;
///     lt.left = 32'd1;
///     eq.right = r.out;
///     eq.left = x.out;
///     comb_cond[done] = 1'd1;
/// }
/// control {
///     if lt.out with comb_cond {
///         ...
///     }
///     while eq.out with comb_cond {
///         ...
///     }
/// }
/// ```
/// into:
/// ```
/// group comb_cond<"static"=1> {
///     lt.right = 32'd10;
///     lt.left = 32'd1;
///     eq.right = r.out;
///     eq.left = x.out;
///     lt_reg.in = lt.out
///     lt_reg.write_en = 1'd1;
///     eq_reg.in = eq.out;
///     eq_reg.write_en = 1'd1;
///     comb_cond[done] = lt_reg.done & eq_reg.done ? 1'd1;
/// }
/// control {
///     if lt_reg.out with comb_cond {
///         ...
///     }
///     while eq_reg.out with comb_cond {
///         ...
///     }
/// }
/// ```
pub struct RemoveCombGroups {
    /// Mapping from name of a group to the ports in the group that were
    /// read from.
    used_ports: HashMap<ir::Id, Vec<RRC<ir::Port>>>,
}

impl Named for RemoveCombGroups {
    fn name() -> &'static str {
        "remove-comb-groups"
    }

    fn description() -> &'static str {
        "Transforms all groups with a constant done condition"
    }
}

impl Visitor for RemoveCombGroups {
    fn start_if(
        &mut self,
        s: &mut ir::If,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        let cond_group = s.cond.borrow();
        self.used_ports
            .entry(cond_group.name().clone())
            .or_default()
            .push(Rc::clone(&s.port));

        Ok(Action::Continue)
    }

    fn start_while(
        &mut self,
        s: &mut ir::While,
        _comp: &mut ir::Component,
        _sigs: &LibrarySignatures,
    ) -> VisResult {
        let cond_group = s.cond.borrow();
        self.used_ports
            .entry(cond_group.name().clone())
            .or_default()
            .push(Rc::clone(&s.port));

        Ok(Action::Continue)
    }

    fn finish(
        &mut self,
        comp: &mut ir::Component,
        sigs: &LibrarySignatures,
    ) -> VisResult {
        // Detach groups from the component
        let groups = comp.groups.drain().collect_vec();
        let mut builder = ir::Builder::new(comp, sigs);
        for group_ref in &groups {
            let group = group_ref.borrow();

            // Is this group combinational
            let done_assign = group
                .assignments
                .iter()
                .find(|assign| {
                    let dst = assign.dst.borrow();
                    dst.is_hole() && *group.name() == dst.get_parent_name()
                })
                .map(|asgn| {
                    asgn.guard.is_true() && asgn.src.borrow().is_constant(1, 1)
                });
            let is_comb = group
                .attributes
                .get("static")
                .map(|v| *v == 0)
                .unwrap_or(false)
                || done_assign.unwrap_or(false);

            if !is_comb {
                continue;
            }

            // Register the ports read by the combinational group's usages.
            let used_ports =
                self.used_ports.remove(group.name()).ok_or_else(|| {
                    Error::MalformedStructure(format!(
                        "Values from combinational group {} never used",
                        group.name()
                    ))
                })?;

            let mut save_regs = Vec::with_capacity(used_ports.len());
            for port in used_ports {
                // Register to save port value
                structure!(builder;
                    let comb_reg = prim std_reg(port.borrow().width);
                    let signal_on = constant(1, 1);
                );
                let write = builder.build_assignment(
                    comb_reg.borrow().get("in"),
                    port,
                    ir::Guard::True,
                );
                let en = builder.build_assignment(
                    comb_reg.borrow().get("write_en"),
                    signal_on.borrow().get("out"),
                    ir::Guard::True,
                );
                group_ref.borrow_mut().assignments.push(write);
                group_ref.borrow_mut().assignments.push(en);
                save_regs.push(comb_reg);
            }

            // Update the done condition
            for mut assign in group_ref.borrow_mut().assignments.iter_mut() {
                let dst = assign.dst.borrow();
                if dst.is_hole() && dst.name == "done" {
                    // The source should be the constant 1 since this is a combinational group.
                    debug_assert!(assign.src.borrow().is_constant(1, 1));
                    assign.guard = Box::new(
                        save_regs
                            .drain(..)
                            .map(|reg| guard!(reg["done"]))
                            .fold(ir::Guard::True, ir::Guard::and),
                    );
                }
            }
        }
        comp.groups = groups.into();

        Ok(Action::Continue)
    }
}
