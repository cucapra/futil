use std::{collections::HashMap, rc::Rc};

use crate::ir::{self, RRC};

/// Contains a mapping from name of groups to the ports read by the control
/// program.
pub struct ControlPorts {
    used_ports: HashMap<ir::Id, Vec<RRC<ir::Port>>>,
}

impl ControlPorts {
    pub fn get(&self, group: &ir::Id) -> Option<&Vec<RRC<ir::Port>>> {
        self.used_ports.get(&group)
    }

    pub fn remove(&mut self, group: &ir::Id) -> Option<Vec<RRC<ir::Port>>> {
        self.used_ports.remove(&group)
    }
}

fn construct(
    con: &ir::Control,
    used_ports: &mut HashMap<ir::Id, Vec<RRC<ir::Port>>>,
) {
    match con {
        ir::Control::Enable(_)
        | ir::Control::Invoke(_)
        | ir::Control::Empty(_) => {}
        ir::Control::If(ir::If {
            cond,
            port,
            tbranch,
            fbranch,
            ..
        }) => {
            used_ports
                .entry(cond.borrow().name().clone())
                .or_default()
                .push(Rc::clone(port));

            construct(&tbranch, used_ports);
            construct(&fbranch, used_ports);
        }
        ir::Control::While(ir::While {
            cond, port, body, ..
        }) => {
            used_ports
                .entry(cond.borrow().name().clone())
                .or_default()
                .push(Rc::clone(port));
            construct(&body, used_ports);
        }
        ir::Control::Seq(ir::Seq { stmts, .. })
        | ir::Control::Par(ir::Par { stmts, .. }) => {
            stmts.iter().for_each(|con| construct(con, used_ports));
        }
    }
}

impl From<&ir::Control> for ControlPorts {
    fn from(con: &ir::Control) -> Self {
        let mut used_ports = HashMap::default();
        construct(con, &mut used_ports);
        ControlPorts { used_ports }
    }
}
