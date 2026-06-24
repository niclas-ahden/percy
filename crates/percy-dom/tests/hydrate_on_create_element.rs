//! Tests that the on_create_element special attribute fires during hydration.
//!
//! Hydration is the only time a server-rendered node is turned into a real element,
//! so a node carrying an on_create_element hook must run it then. This mirrors the
//! fresh-create path, where the hook runs after children are in place.
//!
//! To run all tests in this file:
//!
//! wasm-pack test --firefox --headless crates/percy-dom --test hydrate_on_create_element

extern crate wasm_bindgen_test;
extern crate web_sys;

use console_error_panic_hook;
use percy_dom::prelude::*;
use percy_dom::PercyDom;
use std::cell::RefCell;
use std::rc::Rc;
use web_sys::Element;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

/// Hydrating a mount whose VDOM root carries an on_create_element hook should run that
/// hook once, with the real mounted element.
///
/// wasm-pack test --firefox --headless crates/percy-dom --test hydrate_on_create_element -- on_create_fires_for_root_on_hydrate
#[wasm_bindgen_test]
fn on_create_fires_for_root_on_hydrate() {
    console_error_panic_hook::set_once();

    let document = web_sys::window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    let wrapper: Element = document.create_element("div").unwrap();
    wrapper.set_inner_html(r#"<div id="hydrate-oncreate-root"></div>"#);
    body.append_child(&wrapper).unwrap();

    let calls: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let calls_for_hook = calls.clone();

    let mut vdom: VirtualNode = html! { <div id="hydrate-oncreate-root"></div> };
    vdom.as_velement_mut()
        .unwrap()
        .special_attributes
        .set_on_create_element("root", move |elem: web_sys::Element| {
            calls_for_hook.borrow_mut().push(elem.id());
        });

    let mount_elem = document.get_element_by_id("hydrate-oncreate-root").unwrap();
    let _pdom = PercyDom::new_hydrate_mount(vdom, mount_elem);

    assert_eq!(
        *calls.borrow(),
        vec!["hydrate-oncreate-root".to_string()],
        "on_create_element should fire exactly once for the hydrated root"
    );

    body.remove_child(&wrapper).ok();
}

/// Hydrating a tree where both the root and a nested child carry on_create_element hooks
/// should run both, child before parent (bottom-up), matching the fresh-create ordering.
///
/// wasm-pack test --firefox --headless crates/percy-dom --test hydrate_on_create_element -- on_create_fires_bottom_up_on_hydrate
#[wasm_bindgen_test]
fn on_create_fires_bottom_up_on_hydrate() {
    console_error_panic_hook::set_once();

    let document = web_sys::window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    let wrapper: Element = document.create_element("div").unwrap();
    wrapper.set_inner_html(
        r#"<div id="hydrate-oncreate-parent"><span id="hydrate-oncreate-child"></span></div>"#,
    );
    body.append_child(&wrapper).unwrap();

    let order: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

    let order_parent = order.clone();
    let order_child = order.clone();

    let mut child: VirtualNode = html! { <span id="hydrate-oncreate-child"></span> };
    child
        .as_velement_mut()
        .unwrap()
        .special_attributes
        .set_on_create_element("child", move |_elem: web_sys::Element| {
            order_child.borrow_mut().push("child".to_string());
        });

    let mut parent: VirtualNode = html! { <div id="hydrate-oncreate-parent"></div> };
    {
        let parent_elem = parent.as_velement_mut().unwrap();
        parent_elem
            .special_attributes
            .set_on_create_element("parent", move |_elem: web_sys::Element| {
                order_parent.borrow_mut().push("parent".to_string());
            });
        parent_elem.children.push(child);
    }

    let mount_elem = document
        .get_element_by_id("hydrate-oncreate-parent")
        .unwrap();
    let _pdom = PercyDom::new_hydrate_mount(parent, mount_elem);

    assert_eq!(
        *order.borrow(),
        vec!["child".to_string(), "parent".to_string()],
        "on_create_element should fire bottom-up during hydration (child before parent)"
    );

    body.remove_child(&wrapper).ok();
}
