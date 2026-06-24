//! Tests for the `set_element_effect` special attribute: a node-lifetime effect whose
//! `setup` runs when the element is created (fresh or hydrated) and whose `teardown` runs
//! with the setup's state when the element is removed or the effect's key changes.
//!
//! To run all tests in this file:
//!
//! wasm-pack test --chrome --headless crates/percy-dom --test element_effect

extern crate wasm_bindgen_test;
extern crate web_sys;

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen_test::*;
use web_sys::Element;

use crate::testing_utilities::create_mount;
use percy_dom::prelude::*;
use percy_dom::PercyDom;

mod testing_utilities;

wasm_bindgen_test_configure!(run_in_browser);

/// State that an effect's setup returns and its teardown later receives. Holds a label so a
/// test can prove the *typed* state produced by setup is the exact value handed to teardown.
struct EffectState {
    label: String,
}

/// Build a `<div>` carrying an element effect that logs `setup:<label>` when it runs and
/// `teardown:<state label>` when torn down. `key` is the effect's dependency key.
fn effect_div(
    log: Rc<RefCell<Vec<String>>>,
    key: &'static str,
    label: &'static str,
) -> VirtualNode {
    let log_setup = log.clone();
    let log_teardown = log;

    let mut node = VirtualNode::element("div");
    node.as_velement_mut()
        .unwrap()
        .special_attributes
        .set_element_effect(
            key,
            move |_elem: Element| {
                log_setup.borrow_mut().push(format!("setup:{}", label));
                EffectState {
                    label: label.to_string(),
                }
            },
            move |state: EffectState| {
                log_teardown
                    .borrow_mut()
                    .push(format!("teardown:{}", state.label));
            },
        );
    node
}

/// The full lifecycle on a single mounted element:
/// - setup runs once on create,
/// - a same-key re-render does not re-run the effect,
/// - a key change runs teardown (with the original state) then setup again,
/// - removing the element runs teardown with the current state.
///
/// wasm-pack test --chrome --headless crates/percy-dom --test element_effect -- effect_lifecycle
#[wasm_bindgen_test]
fn effect_lifecycle() {
    let log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

    let mount = create_mount();
    let mut pdom = PercyDom::new_append_to_mount(effect_div(log.clone(), "k1", "a"), &mount);

    // Setup ran exactly once on create.
    assert_eq!(*log.borrow(), vec!["setup:a"]);

    // Same key: the effect is left untouched, even though a fresh vnode (with fresh closures)
    // was diffed over it.
    pdom.update(effect_div(log.clone(), "k1", "b"));
    assert_eq!(*log.borrow(), vec!["setup:a"]);

    // Key changed: teardown runs with the *original* setup's state ("a"), then setup runs
    // again. This is the dependency-scoped re-run, atomic on the same element.
    pdom.update(effect_div(log.clone(), "k2", "c"));
    assert_eq!(*log.borrow(), vec!["setup:a", "teardown:a", "setup:c"]);

    // Element removed (replaced by a different tag): teardown runs with the current state.
    pdom.update(VirtualNode::element("span"));
    assert_eq!(
        *log.borrow(),
        vec!["setup:a", "teardown:a", "setup:c", "teardown:c"]
    );
}

/// An effect on a child element is torn down when an ancestor is removed.
///
/// wasm-pack test --chrome --headless crates/percy-dom --test element_effect -- teardown_runs_for_descendants
#[wasm_bindgen_test]
fn teardown_runs_for_descendants() {
    let log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

    let mut parent = VirtualNode::element("div");
    parent
        .as_velement_mut()
        .unwrap()
        .children
        .push(effect_div(log.clone(), "k1", "child"));

    let mount = create_mount();
    let mut pdom = PercyDom::new_append_to_mount(parent, &mount);
    assert_eq!(*log.borrow(), vec!["setup:child"]);

    // Removing the parent must tear down the child's effect.
    pdom.update(VirtualNode::element("span"));
    assert_eq!(*log.borrow(), vec!["setup:child", "teardown:child"]);
}

/// Setup runs when a server-rendered element is hydrated (the only time that node becomes a
/// real element), mirroring how hydration wires up events and on_create hooks.
///
/// wasm-pack test --chrome --headless crates/percy-dom --test element_effect -- setup_runs_on_hydrate
#[wasm_bindgen_test]
fn setup_runs_on_hydrate() {
    let document = web_sys::window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    let wrapper: Element = document.create_element("div").unwrap();
    wrapper.set_inner_html(r#"<div id="effect-hydrate-root"></div>"#);
    body.append_child(&wrapper).unwrap();

    let log: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

    let mut vdom = html! { <div id="effect-hydrate-root"></div> };
    let log_setup = log.clone();
    let log_teardown = log.clone();
    vdom.as_velement_mut()
        .unwrap()
        .special_attributes
        .set_element_effect(
            "k1",
            move |elem: Element| {
                log_setup.borrow_mut().push(format!("setup:{}", elem.id()));
                EffectState {
                    label: elem.id(),
                }
            },
            move |state: EffectState| {
                log_teardown
                    .borrow_mut()
                    .push(format!("teardown:{}", state.label));
            },
        );

    let mount_elem = document.get_element_by_id("effect-hydrate-root").unwrap();
    let _pdom = PercyDom::new_hydrate_mount(vdom, mount_elem);

    assert_eq!(
        *log.borrow(),
        vec!["setup:effect-hydrate-root".to_string()],
        "effect setup should fire once on the hydrated element"
    );

    body.remove_child(&wrapper).ok();
}
