use super::{Port, RRC};
use std::mem;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};
use std::{cmp::Ordering, hash::Hash, rc::Rc};

/// An assignment guard which has pointers to the various ports from which it reads.
#[derive(Debug, Clone)]
pub enum Guard {
    /// Represents `c1 || c2`.
    Or(Box<Guard>, Box<Guard>),
    /// Represents `c1 && c2`.
    And(Box<Guard>, Box<Guard>),
    /// Represents `!c1`
    Not(Box<Guard>),
    /// The constant true
    True,
    /// Represents `p1 == p2`.
    Eq(RRC<Port>, RRC<Port>),
    /// Represents `p1 != p2`.
    Neq(RRC<Port>, RRC<Port>),
    /// Represents `p1 > p2`.
    Gt(RRC<Port>, RRC<Port>),
    /// Represents `p1 < p2`.
    Lt(RRC<Port>, RRC<Port>),
    /// Represents `p1 >= p2`.
    Geq(RRC<Port>, RRC<Port>),
    /// Represents `p1 <= p2`.
    Leq(RRC<Port>, RRC<Port>),
    /// Uses the value on a port as the condition. Same as `p1 == true`
    Port(RRC<Port>),
}

impl Default for Guard {
    fn default() -> Self {
        Guard::True
    }
}

impl Hash for Guard {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Guard::Or(l, r) | Guard::And(l, r) => {
                l.hash(state);
                r.hash(state)
            }
            Guard::Eq(l, r)
            | Guard::Neq(l, r)
            | Guard::Gt(l, r)
            | Guard::Lt(l, r)
            | Guard::Geq(l, r)
            | Guard::Leq(l, r) => {
                l.borrow().name.hash(state);
                l.borrow().get_parent_name().hash(state);
                r.borrow().name.hash(state);
                r.borrow().get_parent_name().hash(state);
            }
            Guard::Not(inner) => inner.hash(state),
            Guard::Port(p) => {
                p.borrow().name.hash(state);
                p.borrow().get_parent_name().hash(state);
            }
            Guard::True => {}
        }
    }
}

/// Helper functions for the guard.
impl Guard {
    /// Mutates a guard by calling `f` on every leaf in the
    /// guard tree and replacing the leaf with the guard that `f`
    /// returns.
    pub fn for_each<F>(&mut self, f: &F)
    where
        F: Fn(RRC<Port>) -> Option<Guard>,
    {
        match self {
            Guard::And(l, r) | Guard::Or(l, r) => {
                l.for_each(f);
                r.for_each(f);
            }
            Guard::Not(inner) => {
                inner.for_each(f);
            }
            Guard::Eq(l, r)
            | Guard::Neq(l, r)
            | Guard::Gt(l, r)
            | Guard::Lt(l, r)
            | Guard::Geq(l, r)
            | Guard::Leq(l, r) => {
                match f(Rc::clone(l)) {
                    Some(Guard::Port(p)) => *l = p,
                    Some(_) => unreachable!(
                        "Cannot replace port inside comparison operator"
                    ),
                    None => {}
                }
                match f(Rc::clone(r)) {
                    Some(Guard::Port(p)) => *r = p,
                    Some(_) => unreachable!(
                        "Cannot replace port inside comparison operator"
                    ),
                    None => {}
                }
            }
            Guard::Port(port) => {
                let guard = f(Rc::clone(port))
                    .unwrap_or_else(|| Guard::port(Rc::clone(port)));
                *self = guard;
            }
            Guard::True => {}
        }
    }

    /// Returns all the ports used by this guard.
    pub fn all_ports(&self) -> Vec<RRC<Port>> {
        match self {
            Guard::Port(a) => vec![Rc::clone(a)],
            Guard::And(l, r) | Guard::Or(l, r) => {
                let mut atoms = l.all_ports();
                atoms.append(&mut r.all_ports());
                atoms
            }
            Guard::Eq(l, r)
            | Guard::Neq(l, r)
            | Guard::Gt(l, r)
            | Guard::Lt(l, r)
            | Guard::Leq(l, r)
            | Guard::Geq(l, r) => {
                vec![Rc::clone(l), Rc::clone(r)]
            }
            Guard::Not(g) => g.all_ports(),
            Guard::True => vec![],
        }
    }

    /// Returns true if this is a `Guard::True`.
    pub fn is_true(&self) -> bool {
        matches!(self, Guard::True)
    }

    /// Update the guard in place. Replaces this guard with `upd(self)`.
    /// Uses `std::mem::take` for the in-place update.
    #[inline(always)]
    pub fn update<F>(&mut self, upd: F)
    where
        F: FnOnce(Guard) -> Guard,
    {
        let old = mem::take(self);
        let new = upd(old);
        *self = new;
    }

    /// Return the string corresponding to the guard operation.
    pub fn op_str(&self) -> String {
        match self {
            Guard::And(..) => "&".to_string(),
            Guard::Or(..) => "|".to_string(),
            Guard::Eq(..) => "==".to_string(),
            Guard::Neq(..) => "!=".to_string(),
            Guard::Gt(..) => ">".to_string(),
            Guard::Lt(..) => "<".to_string(),
            Guard::Geq(..) => ">=".to_string(),
            Guard::Leq(..) => "<=".to_string(),
            Guard::Not(..) => "!".to_string(),
            Guard::Port(..) | Guard::True => {
                panic!("No operator string for Guard::Port")
            }
        }
    }

    pub fn port(p: RRC<Port>) -> Self {
        if p.borrow().is_constant(1, 1) {
            Guard::True
        } else {
            Guard::Port(p)
        }
    }

    pub fn and(self, rhs: Guard) -> Self {
        if rhs == Guard::True {
            self
        } else if self == Guard::True {
            rhs
        } else if self == rhs {
            self
        } else {
            Guard::And(Box::new(self), Box::new(rhs))
        }
    }

    pub fn or(self, rhs: Guard) -> Self {
        if rhs == Guard::True || self == Guard::True {
            Guard::True
        } else if self == rhs {
            self
        } else {
            Guard::Or(Box::new(self), Box::new(rhs))
        }
    }

    pub fn eq(self, other: Guard) -> Self {
        match (self, other) {
            (Guard::Port(l), Guard::Port(r)) => Guard::Eq(l, r),
            (l, r) => {
                unreachable!("Cannot build Guard::Eq using {:?} and {:?}", l, r)
            }
        }
    }

    pub fn neq(self, other: Guard) -> Self {
        match (self, other) {
            (Guard::Port(l), Guard::Port(r)) => Guard::Neq(l, r),
            (l, r) => {
                unreachable!(
                    "Cannot build Guard::Neq using {:?} and {:?}",
                    l, r
                )
            }
        }
    }

    pub fn le(self, other: Guard) -> Self {
        match (self, other) {
            (Guard::Port(l), Guard::Port(r)) => Guard::Leq(l, r),
            (l, r) => {
                unreachable!(
                    "Cannot build Guard::Leq using {:?} and {:?}",
                    l, r
                )
            }
        }
    }

    pub fn lt(self, other: Guard) -> Self {
        match (self, other) {
            (Guard::Port(l), Guard::Port(r)) => Guard::Lt(l, r),
            (l, r) => {
                unreachable!("Cannot build Guard::Lt using {:?} and {:?}", l, r)
            }
        }
    }

    pub fn ge(self, other: Guard) -> Self {
        match (self, other) {
            (Guard::Port(l), Guard::Port(r)) => Guard::Geq(l, r),
            (l, r) => {
                unreachable!(
                    "Cannot build Guard::Geq using {:?} and {:?}",
                    l, r
                )
            }
        }
    }

    pub fn gt(self, other: Guard) -> Self {
        match (self, other) {
            (Guard::Port(l), Guard::Port(r)) => Guard::Gt(l, r),
            (l, r) => {
                unreachable!("Cannot build Guard::Gt using {:?} and {:?}", l, r)
            }
        }
    }
}

/// Construct guards from ports
impl From<RRC<Port>> for Guard {
    fn from(port: RRC<Port>) -> Self {
        Guard::Port(Rc::clone(&port))
    }
}

impl PartialEq for Guard {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Guard::Or(la, ra), Guard::Or(lb, rb))
            | (Guard::And(la, ra), Guard::And(lb, rb)) => la == lb && ra == rb,
            (Guard::Eq(la, ra), Guard::Eq(lb, rb))
            | (Guard::Neq(la, ra), Guard::Neq(lb, rb))
            | (Guard::Gt(la, ra), Guard::Gt(lb, rb))
            | (Guard::Lt(la, ra), Guard::Lt(lb, rb))
            | (Guard::Geq(la, ra), Guard::Geq(lb, rb))
            | (Guard::Leq(la, ra), Guard::Leq(lb, rb)) => {
                (la.borrow().get_parent_name(), &la.borrow().name)
                    == (lb.borrow().get_parent_name(), &lb.borrow().name)
                    && (ra.borrow().get_parent_name(), &ra.borrow().name)
                        == (rb.borrow().get_parent_name(), &rb.borrow().name)
            }
            (Guard::Not(a), Guard::Not(b)) => a == b,
            (Guard::Port(a), Guard::Port(b)) => {
                (a.borrow().get_parent_name(), &a.borrow().name)
                    == (b.borrow().get_parent_name(), &b.borrow().name)
            }
            (Guard::True, Guard::True) => true,
            _ => false,
        }
    }
}

impl Eq for Guard {}

/// Define order on guards
impl PartialOrd for Guard {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Define an ordering on the precedence of guards. Guards are
/// considered equal when they have the same precedence.
impl Ord for Guard {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Guard::Or(..), Guard::Or(..))
            | (Guard::And(..), Guard::And(..))
            | (Guard::Eq(..), Guard::Eq(..))
            | (Guard::Neq(..), Guard::Neq(..))
            | (Guard::Gt(..), Guard::Gt(..))
            | (Guard::Lt(..), Guard::Lt(..))
            | (Guard::Geq(..), Guard::Geq(..))
            | (Guard::Leq(..), Guard::Leq(..))
            | (Guard::Not(..), Guard::Not(..))
            | (Guard::Port(..), Guard::Port(..))
            | (Guard::True, Guard::True) => Ordering::Equal,
            (Guard::Or(..), _) => Ordering::Greater,
            (_, Guard::Or(..)) => Ordering::Less,
            (Guard::And(..), _) => Ordering::Greater,
            (_, Guard::And(..)) => Ordering::Less,
            (Guard::Leq(..), _) => Ordering::Greater,
            (_, Guard::Leq(..)) => Ordering::Less,
            (Guard::Geq(..), _) => Ordering::Greater,
            (_, Guard::Geq(..)) => Ordering::Less,
            (Guard::Lt(..), _) => Ordering::Greater,
            (_, Guard::Lt(..)) => Ordering::Less,
            (Guard::Gt(..), _) => Ordering::Greater,
            (_, Guard::Gt(..)) => Ordering::Less,
            (Guard::Eq(..), _) => Ordering::Greater,
            (_, Guard::Eq(..)) => Ordering::Less,
            (Guard::Neq(..), _) => Ordering::Greater,
            (_, Guard::Neq(..)) => Ordering::Less,
            (Guard::Not(..), _) => Ordering::Greater,
            (_, Guard::Not(..)) => Ordering::Less,
            (Guard::Port(..), _) => Ordering::Greater,
            (_, Guard::Port(..)) => Ordering::Less,
        }
    }
}

/////////////// Sugar for convience constructors /////////////

/// Construct a Guard::And:
/// ```
/// let and_guard = g1 & g2;
/// ```
impl BitAnd for Guard {
    type Output = Self;

    fn bitand(self, other: Self) -> Self::Output {
        self.and(other)
    }
}

/// Construct a Guard::Or:
/// ```
/// let or_guard = g1 | g2;
/// ```
impl BitOr for Guard {
    type Output = Self;

    fn bitor(self, other: Self) -> Self::Output {
        self.or(other)
    }
}

/// Construct a Guard::Or:
/// ```
/// let not_guard = !g1;
/// ```
impl Not for Guard {
    type Output = Self;

    fn not(self) -> Self {
        match self {
            Guard::Eq(lhs, rhs) => Guard::Neq(lhs, rhs),
            Guard::Neq(lhs, rhs) => Guard::Eq(lhs, rhs),
            Guard::Gt(lhs, rhs) => Guard::Leq(lhs, rhs),
            Guard::Lt(lhs, rhs) => Guard::Geq(lhs, rhs),
            Guard::Geq(lhs, rhs) => Guard::Lt(lhs, rhs),
            Guard::Leq(lhs, rhs) => Guard::Gt(lhs, rhs),
            Guard::Not(expr) => *expr,
            _ => Guard::Not(Box::new(self)),
        }
    }
}

/// Update a Guard with Or.
/// ```
/// g1 |= g2;
/// ```
impl BitOrAssign for Guard {
    fn bitor_assign(&mut self, other: Self) {
        self.update(|old| old | other)
    }
}

/// Update a Guard with Or.
/// ```
/// g1 &= g2;
/// ```
impl BitAndAssign for Guard {
    fn bitand_assign(&mut self, other: Self) {
        self.update(|old| old & other)
    }
}
