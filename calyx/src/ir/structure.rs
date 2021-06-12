//! Representation for structure (wires and cells) in a Calyx program.
use super::{Attributes, GetAttributes, Guard, Id, RRC, WRC};
use smallvec::SmallVec;
use std::rc::Rc;

/// Direction of a port on a cell.
#[derive(Debug, Clone, PartialEq)]
pub enum Direction {
    /// Input port.
    Input,
    /// Output port.
    Output,
    /// Input-Output "port". Should only be used by holes.
    Inout,
}

impl Direction {
    /// Return the direction opposite to the current direction
    pub fn reverse(&self) -> Self {
        match self {
            Direction::Input => Direction::Output,
            Direction::Output => Direction::Input,
            Direction::Inout => Direction::Inout,
        }
    }
}

impl<T> From<T> for Direction
where
    T: AsRef<str> + std::fmt::Display,
{
    fn from(s: T) -> Self {
        match s.as_ref() {
            "inout" => Direction::Inout,
            "in" => Direction::Input,
            "out" => Direction::Output,
            _ => panic!("Unknown port direction {}", s),
        }
    }
}

/// Ports can come from Cells or Groups
#[derive(Debug, Clone)]
pub enum PortParent {
    Cell(WRC<Cell>),
    Group(WRC<Group>),
}

/// Represents a port on a cell.
#[derive(Debug, Clone)]
pub struct Port {
    /// Name of the port
    pub name: Id,
    /// Width of the port
    pub width: u64,
    /// Direction of the port
    pub direction: Direction,
    /// Weak pointer to this port's parent
    pub parent: PortParent,
    /// Attributes associated with this port.
    pub attributes: Attributes,
}

impl Port {
    /// Checks if this port is a hole
    pub fn is_hole(&self) -> bool {
        matches!(&self.parent, PortParent::Group(_))
    }

    /// Checks if this port is a constant of value: `val`.
    pub fn is_constant(&self, val: u64, width: u64) -> bool {
        if let PortParent::Cell(cell) = &self.parent {
            match cell.upgrade().borrow().prototype {
                CellType::Constant { val: v, width: w } => {
                    v == val && width == w
                }
                _ => false,
            }
        } else {
            false
        }
    }

    /// Gets name of parent object.
    pub fn get_parent_name(&self) -> Id {
        match &self.parent {
            PortParent::Cell(cell) => cell.upgrade().borrow().name.clone(),
            PortParent::Group(group) => group.upgrade().borrow().name.clone(),
        }
    }

    /// Get the canonical representation for this Port.
    pub fn canonical(&self) -> (Id, Id) {
        (self.get_parent_name(), self.name.clone())
    }
}

impl PartialEq for Port {
    fn eq(&self, other: &Self) -> bool {
        self.get_parent_name() == other.get_parent_name()
            && self.name == other.name
    }
}

impl Eq for Port {}

/// Alias for bindings
pub type Binding = SmallVec<[(Id, u64); 5]>;

/// The type for a Cell
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum CellType {
    /// Cell constructed using a primitive definition
    Primitive {
        /// Name of the primitive cell used to instantiate this cell.
        name: Id,
        /// Bindings for the parameters. Uses Vec to retain the input order.
        param_binding: Binding,
    },
    /// Cell constructed using a Calyx component
    Component {
        /// Name of the component used to instantiate this cell.
        name: Id,
    },
    /// This cell represents the current component
    ThisComponent,
    /// Cell representing a Constant
    Constant {
        /// Value of this constant
        val: u64,
        /// Width of this constant
        width: u64,
    },
}

/// Represents an instantiated cell.
#[derive(Debug)]
pub struct Cell {
    /// Name of this cell.
    pub(super) name: Id,
    /// Ports on this cell
    pub ports: SmallVec<[RRC<Port>; 10]>,
    /// Underlying type for this cell
    pub prototype: CellType,
    /// Attributes for this group.
    pub(super) attributes: Attributes,
}

impl GetAttributes for Cell {
    fn get_attributes(&self) -> Option<&Attributes> {
        Some(&self.attributes)
    }

    fn get_mut_attributes(&mut self) -> Option<&mut Attributes> {
        Some(&mut self.attributes)
    }
}

impl Cell {
    /// Get a reference to the named port if it exists.
    pub fn find<S>(&self, name: S) -> Option<RRC<Port>>
    where
        S: std::fmt::Display + Clone + AsRef<str>,
    {
        self.ports
            .iter()
            .find(|&g| g.borrow().name == name)
            .map(|r| Rc::clone(r))
    }

    /// Get a reference to the named port and throw an error if it doesn't
    /// exist.
    pub fn get<S>(&self, name: S) -> RRC<Port>
    where
        S: std::fmt::Display + Clone + AsRef<str>,
    {
        self.find(&name).unwrap_or_else(|| {
            panic!(
                "Port `{}' not found on cell `{}'",
                name.to_string(),
                self.name.to_string()
            )
        })
    }

    /// Returns the name of the component that is this cells type.
    pub fn type_name(&self) -> Option<&Id> {
        match &self.prototype {
            CellType::Primitive { name, .. } | CellType::Component { name } => {
                Some(name)
            }
            CellType::ThisComponent => Some(&self.name),
            CellType::Constant { .. } => None,
        }
    }

    /// Get parameter binding from the prototype used to build this cell.
    pub fn get_paramter<S>(&self, param: S) -> Option<u64>
    where
        S: std::fmt::Display + Clone + AsRef<str>,
    {
        match &self.prototype {
            CellType::Primitive { param_binding, .. } => param_binding
                .iter()
                .find(|(key, _)| *key == param)
                .map(|(_, val)| *val),
            CellType::Component { .. } => None,
            CellType::ThisComponent => None,
            CellType::Constant { .. } => None,
        }
    }

    /// Return the canonical name for the cell generated to represent this
    /// (val, width) constant.
    pub(super) fn constant_name(val: u64, width: u64) -> Id {
        format!("_{}_{}", val, width).into()
    }

    /// Return the value associated with this attribute key.
    pub fn get_attribute<S>(&self, attr: S) -> Option<&u64>
    where
        S: AsRef<str>,
    {
        self.attributes.get(attr.as_ref())
    }

    /// Add a new attribute to the group.
    pub fn add_attribute<S>(&mut self, attr: S, value: u64)
    where
        S: Into<String>,
    {
        self.attributes.insert(attr.into(), value);
    }

    /// Grants immutable access to the name of this cell.
    pub fn name(&self) -> &Id {
        &self.name
    }
}

/// Represents a guarded assignment in the program
#[derive(Clone, Debug)]
pub struct Assignment {
    /// The destination for the assignment.
    pub dst: RRC<Port>,

    /// The source for the assignment.
    pub src: RRC<Port>,

    /// The guard for this assignment.
    pub guard: Box<Guard>,
}

/// A Group of assignments that perform a logical action.
#[derive(Debug)]
pub struct Group {
    /// Name of this group
    pub(super) name: Id,

    /// The assignments used in this group
    pub assignments: Vec<Assignment>,

    /// Holes for this group
    pub holes: SmallVec<[RRC<Port>; 3]>,

    /// Attributes for this group.
    pub attributes: Attributes,
}
impl Group {
    /// Get a reference to the named hole if it exists.
    pub fn find<S>(&self, name: &S) -> Option<RRC<Port>>
    where
        S: std::fmt::Display + Clone + AsRef<str>,
    {
        self.holes
            .iter()
            .find(|&g| g.borrow().name == name)
            .map(|r| Rc::clone(r))
    }

    /// Get a reference to the named hole or panic.
    pub fn get<S>(&self, name: S) -> RRC<Port>
    where
        S: std::fmt::Display + Clone + AsRef<str>,
    {
        self.find(&name).unwrap_or_else(|| {
            panic!(
                "Hole `{}' not found on group `{}'",
                name.to_string(),
                self.name.to_string()
            )
        })
    }

    pub fn name(&self) -> &Id {
        &self.name
    }
}

/// A trait representing something in the IR that has a name.
pub trait GetName {
    /// Return a reference to the object's name
    fn name(&self) -> &Id;
}

impl GetName for Cell {
    fn name(&self) -> &Id {
        self.name()
    }
}

impl GetName for Group {
    fn name(&self) -> &Id {
        self.name()
    }
}

/// A utility trait representing the ability to clone the name of an object.
/// Automatically definied for anything that implements GetName
pub trait CloneName {
    /// Returns a clone of the object's name
    fn clone_name(&self) -> Id;
}

impl<T: GetName> CloneName for T {
    fn clone_name(&self) -> Id {
        self.name().clone()
    }
}
