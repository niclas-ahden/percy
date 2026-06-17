//! Tests that ensure hydration followed by updating works correctly.
//!
//! This covers the real-world SSR flow: server renders HTML → Percy hydrates it →
//! client updates with a new VDOM (possibly with more/fewer children).
//!
//! To run all tests in this file:
//!
//! wasm-pack test --firefox --headless crates/percy-dom --test hydrate_update

extern crate wasm_bindgen_test;
extern crate web_sys;

use console_error_panic_hook;
use percy_dom::prelude::*;
use percy_dom::PercyDom;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::Element;

wasm_bindgen_test_configure!(run_in_browser);

/// After hydrating a node with 1 child, updating to 3 same-tag children should
/// produce exactly 3 children — not duplicates.
///
/// This is the real-world scenario: server renders 1 <img>, client fetches more
/// images and updates to 3 <img> elements for a slideshow.
///
/// wasm-pack test --firefox --headless crates/percy-dom --test hydrate_update -- hydrate_then_grow_same_tag_children
#[wasm_bindgen_test]
fn hydrate_then_grow_same_tag_children() {
    console_error_panic_hook::set_once();

    let document = web_sys::window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    // Step 1: Simulate server-rendered HTML by creating a real DOM element
    let mount: Element = document.create_element("div").unwrap();
    mount.set_attribute("id", "hydrate-grow-test").unwrap();

    let img = document.create_element("img").unwrap();
    img.set_attribute("class", "a").unwrap();
    img.set_attribute("src", "1.jpg").unwrap();
    mount.append_child(&img).unwrap();

    body.append_child(&mount).unwrap();

    // Step 2: Hydrate with the same VDOM structure (1 child)
    let initial_vdom = html! {
        <div id="hydrate-grow-test">
            <img class="a" src="1.jpg" />
        </div>
    };

    let mount_elem = document.get_element_by_id("hydrate-grow-test").unwrap();
    let mut pdom = PercyDom::new_hydrate_mount(initial_vdom, mount_elem);

    // Step 3: Update to 3 same-tag children
    let updated_vdom = html! {
        <div id="hydrate-grow-test">
            <img class="b" src="1.jpg" />
            <img class="c" src="2.jpg" />
            <img class="d" src="3.jpg" />
        </div>
    };

    pdom.update(updated_vdom);

    // Step 4: Verify — should have exactly 3 <img> children
    let root: Element = pdom.root_node().unchecked_into();
    let children = root.children();
    assert_eq!(
        children.length(),
        3,
        "Expected 3 <img> children after hydrate + update, got {}. innerHTML: {}",
        children.length(),
        root.inner_html()
    );

    // Verify the attributes are correct
    let child0: Element = children.item(0).unwrap();
    let child1: Element = children.item(1).unwrap();
    let child2: Element = children.item(2).unwrap();

    assert_eq!(child0.get_attribute("class").unwrap(), "b");
    assert_eq!(child0.get_attribute("src").unwrap(), "1.jpg");
    assert_eq!(child1.get_attribute("class").unwrap(), "c");
    assert_eq!(child1.get_attribute("src").unwrap(), "2.jpg");
    assert_eq!(child2.get_attribute("class").unwrap(), "d");
    assert_eq!(child2.get_attribute("src").unwrap(), "3.jpg");

    // Cleanup
    body.remove_child(&root).ok();
}

/// After hydrating a node with 1 child, updating to 2 same-tag children should
/// produce exactly 2 children.
///
/// wasm-pack test --firefox --headless crates/percy-dom --test hydrate_update -- hydrate_then_grow_one_to_two
#[wasm_bindgen_test]
fn hydrate_then_grow_one_to_two() {
    console_error_panic_hook::set_once();

    let document = web_sys::window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    // Simulate server-rendered HTML
    let mount: Element = document.create_element("div").unwrap();
    mount.set_attribute("id", "hydrate-grow-1to2").unwrap();

    let img = document.create_element("img").unwrap();
    img.set_attribute("class", "a").unwrap();
    img.set_attribute("src", "1.jpg").unwrap();
    mount.append_child(&img).unwrap();

    body.append_child(&mount).unwrap();

    // Hydrate
    let initial_vdom = html! {
        <div id="hydrate-grow-1to2">
            <img class="a" src="1.jpg" />
        </div>
    };

    let mount_elem = document.get_element_by_id("hydrate-grow-1to2").unwrap();
    let mut pdom = PercyDom::new_hydrate_mount(initial_vdom, mount_elem);

    // Update to 2 children
    let updated_vdom = html! {
        <div id="hydrate-grow-1to2">
            <img class="b" src="1.jpg" />
            <img class="c" src="2.jpg" />
        </div>
    };

    pdom.update(updated_vdom);

    // Verify
    let root: Element = pdom.root_node().unchecked_into();
    let children = root.children();
    assert_eq!(
        children.length(),
        2,
        "Expected 2 <img> children after hydrate + update, got {}. innerHTML: {}",
        children.length(),
        root.inner_html()
    );

    body.remove_child(&root).ok();
}

/// Hydrate with 3 children, then update to 1 child. Should have exactly 1 child.
///
/// wasm-pack test --firefox --headless crates/percy-dom --test hydrate_update -- hydrate_then_shrink
#[wasm_bindgen_test]
fn hydrate_then_shrink() {
    console_error_panic_hook::set_once();

    let document = web_sys::window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    // Simulate server-rendered HTML with 3 images
    let mount: Element = document.create_element("div").unwrap();
    mount.set_attribute("id", "hydrate-shrink").unwrap();

    for i in 0..3 {
        let img = document.create_element("img").unwrap();
        img.set_attribute("src", &format!("{}.jpg", i + 1)).unwrap();
        mount.append_child(&img).unwrap();
    }

    body.append_child(&mount).unwrap();

    // Hydrate with 3 children
    let initial_vdom = html! {
        <div id="hydrate-shrink">
            <img src="1.jpg" />
            <img src="2.jpg" />
            <img src="3.jpg" />
        </div>
    };

    let mount_elem = document.get_element_by_id("hydrate-shrink").unwrap();
    let mut pdom = PercyDom::new_hydrate_mount(initial_vdom, mount_elem);

    // Update to 1 child
    let updated_vdom = html! {
        <div id="hydrate-shrink">
            <img src="1.jpg" />
        </div>
    };

    pdom.update(updated_vdom);

    let root: Element = pdom.root_node().unchecked_into();
    let children = root.children();
    assert_eq!(
        children.length(),
        1,
        "Expected 1 <img> child after hydrate + shrink, got {}. innerHTML: {}",
        children.length(),
        root.inner_html()
    );

    body.remove_child(&root).ok();
}

/// Same as hydrate_then_grow_same_tag_children but uses innerHTML to set up
/// the DOM, which is what actually happens in SSR: the browser parses an HTML
/// string, potentially creating whitespace text nodes between elements.
///
/// wasm-pack test --firefox --headless crates/percy-dom --test hydrate_update -- hydrate_from_innerhtml_then_grow
#[wasm_bindgen_test]
fn hydrate_from_innerhtml_then_grow() {
    console_error_panic_hook::set_once();

    let document = web_sys::window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    // Create a wrapper and use innerHTML to simulate browser-parsed SSR HTML.
    let wrapper: Element = document.create_element("div").unwrap();
    wrapper.set_inner_html(r#"<div id="hydrate-innerhtml-grow"><img class="a" src="1.jpg"></div>"#);
    body.append_child(&wrapper).unwrap();

    let mount_elem = document.get_element_by_id("hydrate-innerhtml-grow").unwrap();

    // Hydrate with matching VDOM
    let initial_vdom = html! {
        <div id="hydrate-innerhtml-grow">
            <img class="a" src="1.jpg" />
        </div>
    };

    let mut pdom = PercyDom::new_hydrate_mount(initial_vdom, mount_elem);

    // Update to 3 children
    let updated_vdom = html! {
        <div id="hydrate-innerhtml-grow">
            <img class="b" src="1.jpg" />
            <img class="c" src="2.jpg" />
            <img class="d" src="3.jpg" />
        </div>
    };

    pdom.update(updated_vdom);

    let root: Element = pdom.root_node().unchecked_into();
    let children = root.children();
    assert_eq!(
        children.length(),
        3,
        "Expected 3 <img> children after innerHTML hydrate + update, got {}. innerHTML: {}",
        children.length(),
        root.inner_html()
    );

    body.remove_child(&wrapper).ok();
}

/// Simulate the EXACT real-world scenario: a deeply nested structure where the
/// parent has whitespace text nodes between child elements (as generated by
/// SSR with indentation). Then hydrate and grow children.
///
/// wasm-pack test --firefox --headless crates/percy-dom --test hydrate_update -- hydrate_nested_with_whitespace_then_grow
#[wasm_bindgen_test]
fn hydrate_nested_with_whitespace_then_grow() {
    console_error_panic_hook::set_once();

    let document = web_sys::window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    // Simulate server-rendered HTML with whitespace/newlines between elements
    // (as a real HTML serializer would produce)
    let wrapper: Element = document.create_element("div").unwrap();
    wrapper.set_inner_html(
        r#"<div id="hydrate-nested-ws" class="Hero">
  <div class="Hero-media">
    <img class="Hero-image" src="1.jpg">
  </div>
</div>"#
    );
    body.append_child(&wrapper).unwrap();

    let mount_elem = document.get_element_by_id("hydrate-nested-ws").unwrap();

    // Hydrate with VDOM that matches the structure
    // Note: the html! macro does NOT produce whitespace text nodes between elements.
    // This mismatch between DOM (with whitespace text nodes) and VDOM (without) is
    // the key difference from the manually-created DOM tests.
    let initial_vdom = html! {
        <div id="hydrate-nested-ws" class="Hero">
            <div class="Hero-media">
                <img class="Hero-image" src="1.jpg" />
            </div>
        </div>
    };

    let mut pdom = PercyDom::new_hydrate_mount(initial_vdom, mount_elem);

    // Now update: the Hero-media div gets 3 images instead of 1
    let updated_vdom = html! {
        <div id="hydrate-nested-ws" class="Hero">
            <div class="Hero-media">
                <img class="Hero-image Hero-image--current" src="1.jpg" />
                <img class="Hero-image" src="2.jpg" />
                <img class="Hero-image" src="3.jpg" />
            </div>
        </div>
    };

    pdom.update(updated_vdom);

    let root: Element = pdom.root_node().unchecked_into();
    let media_div = root.query_selector(".Hero-media").unwrap().unwrap();
    let imgs = media_div.children();

    assert_eq!(
        imgs.length(),
        3,
        "Expected 3 <img> children in .Hero-media after hydrate + update, got {}. innerHTML: {}",
        imgs.length(),
        media_div.inner_html()
    );
}

/// Test with a Hero-component-style structure:
/// Server renders a single <img> inside nested divs. Client updates to multiple images.
///
/// wasm-pack test --firefox --headless crates/percy-dom --test hydrate_update -- hydrate_hero_slideshow_scenario
#[wasm_bindgen_test]
fn hydrate_hero_slideshow_scenario() {
    console_error_panic_hook::set_once();

    let document = web_sys::window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    // SSR output: the server sends something like this
    let wrapper: Element = document.create_element("div").unwrap();
    wrapper.set_inner_html(
        r#"<div id="app-hero-test"><div class="Hero"><div class="Hero-content"><h1>Title</h1></div><div class="Hero-media"><img class="Hero-image" src="img1.jpg"></div></div></div>"#
    );
    body.append_child(&wrapper).unwrap();

    let mount_elem = document.get_element_by_id("app-hero-test").unwrap();

    // VDOM from initial render (matches SSR)
    let initial_vdom = html! {
        <div id="app-hero-test">
            <div class="Hero">
                <div class="Hero-content">
                    <h1>Title</h1>
                </div>
                <div class="Hero-media">
                    <img class="Hero-image" src="img1.jpg" />
                </div>
            </div>
        </div>
    };

    let mut pdom = PercyDom::new_hydrate_mount(initial_vdom, mount_elem);

    // Client loads images, updates to 3 images
    let updated_vdom = html! {
        <div id="app-hero-test">
            <div class="Hero">
                <div class="Hero-content">
                    <h1>Title</h1>
                </div>
                <div class="Hero-media">
                    <img class="Hero-image Hero-image--current" src="img1.jpg" />
                    <img class="Hero-image" src="img2.jpg" />
                    <img class="Hero-image" src="img3.jpg" />
                </div>
            </div>
        </div>
    };

    pdom.update(updated_vdom);

    let root: Element = pdom.root_node().unchecked_into();
    let media_div = root.query_selector(".Hero-media").unwrap().unwrap();
    let imgs = media_div.children();

    assert_eq!(
        imgs.length(),
        3,
        "Expected 3 <img> children in Hero-media, got {}. innerHTML: {}",
        imgs.length(),
        media_div.inner_html()
    );

    // Verify no duplicate images anywhere
    let all_imgs = root.query_selector_all("img").unwrap();
    assert_eq!(
        all_imgs.length(),
        3,
        "Expected 3 total <img> elements in entire tree, got {}",
        all_imgs.length()
    );

    body.remove_child(&wrapper).ok();
}

/// Hydrate then do multiple sequential updates that change child count.
/// This simulates a slideshow: 1 → 3 → advance (attrs change) → advance again.
///
/// wasm-pack test --firefox --headless crates/percy-dom --test hydrate_update -- hydrate_then_multiple_updates
#[wasm_bindgen_test]
fn hydrate_then_multiple_updates() {
    console_error_panic_hook::set_once();

    let document = web_sys::window().unwrap().document().unwrap();
    let body = document.body().unwrap();

    // Server-rendered: 1 image
    let mount: Element = document.create_element("div").unwrap();
    mount.set_attribute("id", "hydrate-multi").unwrap();

    let img = document.create_element("img").unwrap();
    img.set_attribute("class", "current").unwrap();
    img.set_attribute("src", "1.jpg").unwrap();
    mount.append_child(&img).unwrap();

    body.append_child(&mount).unwrap();

    let initial_vdom = html! {
        <div id="hydrate-multi">
            <img class="current" src="1.jpg" />
        </div>
    };

    let mount_elem = document.get_element_by_id("hydrate-multi").unwrap();
    let mut pdom = PercyDom::new_hydrate_mount(initial_vdom, mount_elem);

    // Update 1: grow to 3 children (images loaded)
    pdom.update(html! {
        <div id="hydrate-multi">
            <img class="current" src="1.jpg" />
            <img class="next" src="2.jpg" />
            <img class="next" src="3.jpg" />
        </div>
    });

    let root: Element = pdom.root_node().unchecked_into();
    assert_eq!(root.children().length(), 3, "After grow: expected 3, got {}", root.children().length());

    // Update 2: slideshow advances (attrs change, same child count)
    pdom.update(html! {
        <div id="hydrate-multi">
            <img class="previous" src="1.jpg" />
            <img class="current" src="2.jpg" />
            <img class="next" src="3.jpg" />
        </div>
    });

    let root: Element = pdom.root_node().unchecked_into();
    assert_eq!(root.children().length(), 3, "After advance: expected 3, got {}", root.children().length());

    // Verify attributes updated correctly
    let child0: Element = root.children().item(0).unwrap();
    let child1: Element = root.children().item(1).unwrap();
    assert_eq!(child0.get_attribute("class").unwrap(), "previous");
    assert_eq!(child1.get_attribute("class").unwrap(), "current");

    body.remove_child(&root).ok();
}
