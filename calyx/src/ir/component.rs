use super::{
    Assignment, Attributes, Builder, Cell, CellType, Control, Direction, Group,
    Id, RRC,
};
use crate::utils;
use linked_hash_map::LinkedHashMap;
use std::cell::RefCell;
use std::rc::Rc;

/// The default name of the signature cell in a component.
/// In general, this should not be used by anything.
const THIS_ID: &str = "_this";

/// In memory representation of a Component.
#[derive(Debug)]
pub struct Component {
    /// Name of the component.
    pub name: Id,
    /// The input/output signature of this component.
    pub signature: RRC<Cell>,
    /// The cells instantiated for this component.
    cells: LinkedHashMap<Id, RRC<Cell>>,
    /// Groups of assignment wires.
    groups: LinkedHashMap<Id, RRC<Group>>,
    /// The set of "continuous assignments", i.e., assignments that are always
    /// active.
    pub continuous_assignments: Vec<Assignment>,
    /// The control program for this component.
    pub control: RRC<Control>,
    /// Attributes for this component
    pub attributes: Attributes,

    ///// Internal structures
    /// Namegenerator that contains the names currently defined in this
    /// component (cell and group names).
    namegen: utils::NameGenerator,
}

/// Builder methods for extracting and construction IR nodes.
/// The naming scheme for methods is consistent:
/// - find_<construct>: Returns a reference to the construct with the given
///   name.
impl Component {
    /// Construct a new Component with the given `name` and signature fields.
    pub fn new<S, N>(
        name: S,
        ports: Vec<(N, u64, Direction, Attributes)>,
    ) -> Self
    where
        S: AsRef<str>,
        N: AsRef<str>,
    {
        let this_sig = Builder::cell_from_signature(
            THIS_ID.into(),
            CellType::ThisComponent,
            ports
                .into_iter()
                // Reverse the port directions inside the component.
                .map(|(name, w, d, attrs)| {
                    (name.as_ref().into(), w, d.reverse(), attrs)
                })
                .collect(),
        );

        Component {
            name: name.as_ref().into(),
            signature: this_sig,
            cells: LinkedHashMap::new(),
            groups: LinkedHashMap::new(),
            continuous_assignments: vec![],
            control: Rc::new(RefCell::new(Control::empty())),
            namegen: utils::NameGenerator::default(),
            attributes: Attributes::default(),
        }
    }

    /// Return a reference to the group with `name` if present.
    pub fn find_group<S>(&self, name: &S) -> Option<RRC<Group>>
    where
        S: Clone + AsRef<str>,
    {
        self.groups.get(&name.as_ref().into()).map(|r| Rc::clone(r))
    }

    /// Return a reference to the cell with `name` if present.
    pub fn find_cell<S>(&self, name: &S) -> Option<RRC<Cell>>
    where
        S: Clone + AsRef<str>,
    {
        self.cells.get(&name.as_ref().into()).map(|r| Rc::clone(r))
    }

    /// Construct a non-conflicting name using the Component's namegenerator.
    pub fn generate_name<S>(&mut self, prefix: S) -> Id
    where
        S: Into<Id> + ToString,
    {
        self.namegen.gen_name(prefix)
    }

    /// Returns an iterator over the groups contained in this component.
    pub fn iter_groups(&self) -> impl Iterator<Item = &RRC<Group>> {
        self.groups.values()
    }

    /// Returns an iterator over the cells contained in this component.
    pub fn iter_cells(&self) -> impl Iterator<Item = &RRC<Cell>> {
        self.cells.values()
    }

    /// Adds the given group to the component.
    pub fn add_group(&mut self, group: RRC<Group>) {
        self.groups.insert(group.borrow().name().clone(), group);
    }

    /// Adds the given cell to the component.
    pub fn add_cell(&mut self, cell: RRC<Cell>) {
        self.cells.insert(cell.borrow().name().clone(), cell);
    }

    pub fn add_cells(&mut self, cells: Vec<RRC<Cell>>) {
        for cell in cells {
            self.add_cell(cell)
        }
    }

    pub fn retain_cells<F>(&mut self, f: F)
    where
        F: FnMut(&RRC<Cell>) -> bool,
    {
        for entry in self.cells.entries() {
            if !f(entry.get()) {
                entry.remove();
            }
        }
    }

    pub fn clear_groups(&mut self) {
        self.groups.clear();
    }

    pub fn drain_cells(&mut self) -> impl Iterator<Item = RRC<Cell>> {
        let drain = std::mem::take(&mut self.cells);

        drain.into_iter().map(|(_, cell)| cell)
    }
}
