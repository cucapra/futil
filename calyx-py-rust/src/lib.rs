//! Python bindings for the IR in Rust.
//! Because this operates across language boundaries, it makes use of `unsafe`
//! code.
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use std::ffi::c_void;

use calyx::ir;

unsafe impl Send for ComponentWrapper {}

/// Opaque wrapper struct for `ir::Component`.
/// It stores a `void*` pointer which **is not guaranteed** to be valid.
/// Functions that unwrap this component into a `Box` must call `raw_transform`
/// to make sure that the function does not run the drop method and frees
/// the component.
#[pyclass]
#[derive(Clone)]
struct ComponentWrapper {
    /// Raw pointer to the underlying `ir::Component`. Not guaranteed to
    /// be valid.
    comp: *mut c_void,
}

/// Transform a boxed ir::Component into a ComponentWrapper.
/// Leaks the underlying ir::Component so that it can be used by the
/// python library.
fn into_wrapper(comp: Box<ir::Component>) -> ComponentWrapper {
    ComponentWrapper {
        comp: Box::into_raw(comp) as *mut c_void,
    }
}

/// Check if the wrapper contains a valid pointer to an ir::Component
/// and returns a boxed representation.
/// **Note**: Methods using this probably want to use `into_wrapper` to
/// create a valid wrapper at the end of the function call.
fn from_wrapper(wrap: ComponentWrapper) -> Box<ir::Component> {
    unsafe {
        assert!(
            !wrap.comp.is_null(),
            "Invalid component wrapper. The component seems to have been already dropped.");
        let comp_raw = wrap.comp as *mut ir::Component;
        Box::from_raw(comp_raw)
    }
}

/// Create a new component.
/// Returns an opaque ComponentWrapper that can be used in other functions.
///
/// `name`: The name of the component.
/// `port`: List of tuples with (name, size, direction)
#[pyfunction]
fn new_component(
    name: &str,
    ports: Vec<(&str, u64, &str)>,
) -> ComponentWrapper {
    let port_defs = ports
        .into_iter()
        .map(|(name, width, dir)| {
            (name, width, dir.into(), ir::Attributes::default())
        })
        .collect();
    let comp = Box::new(ir::Component::new(name, port_defs));
    ComponentWrapper {
        comp: Box::into_raw(comp) as *mut c_void,
    }
}

/// Get the name of this Component.
#[pyfunction]
fn get_component_name(c: ComponentWrapper) -> String {
    let comp = from_wrapper(c);
    let ret = comp.name.to_string();
    into_wrapper(comp);
    ret
}

/// Free the underlying ir::Component. All components must have this
/// method called on them to ensure that they don't leak.
#[pyfunction]
fn free_component(c: ComponentWrapper) -> () {
    from_wrapper(c);
    // The component is dropped automatically due to Box<T> destructor.
}

/// The python module exposing the relevant functions.
#[pymodule]
fn libcalyx(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(new_component, m)?)?;
    m.add_function(wrap_pyfunction!(get_component_name, m)?)?;
    m.add_function(wrap_pyfunction!(free_component, m)?)?;

    Ok(())
}
