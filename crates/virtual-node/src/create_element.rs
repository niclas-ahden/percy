use js_sys::Reflect;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{Document, Element, Node, Text};

use crate::event::{set_events_id, VirtualEventElement, VirtualEvents};
use crate::{AttributeValue, VirtualEventNode, VirtualNode, VElement};

mod add_events;

// Used to indicate that a DOM node was created from a virtual-node.
#[doc(hidden)]
pub const VIRTUAL_NODE_MARKER_PROPERTY: &'static str = "__v__";

impl VElement {
    /// Build a DOM element by recursively creating DOM nodes for this element and it's
    /// children, it's children's children, etc.
    pub(crate) fn create_element_node(
        &self,
        events: &mut VirtualEvents,
    ) -> (Element, VirtualEventNode) {
        let document = web_sys::window().unwrap().document().unwrap();

        let element = if html_validation::is_svg_namespace(&self.tag) {
            document
                .create_element_ns(Some("http://www.w3.org/2000/svg"), &self.tag)
                .unwrap()
        } else {
            document.create_element(&self.tag).unwrap()
        };
        set_virtual_node_marker(&element);

        self.attrs.iter().for_each(|(name, value)| {
            match value {
                AttributeValue::String(s) => {
                    element.set_attribute(name, s).unwrap();
                }
                AttributeValue::Bool(b) => {
                    if *b {
                        element.set_attribute(name, "").unwrap();
                    }
                }
            };
        });

        let mut event_elem = events.create_element_node();
        self.add_events(
            &element,
            events,
            event_elem.as_element().unwrap().events_id(),
        );

        self.append_children_to_dom(
            &element,
            &document,
            event_elem.as_element_mut().unwrap(),
            events,
        );

        self.special_attributes
            .maybe_call_on_create_element(&element);

        if let Some(inner_html) = &self.special_attributes.dangerous_inner_html {
            element.set_inner_html(inner_html);
        }

        (element, event_elem)
    }
}

impl VElement {
    fn append_children_to_dom(
        &self,
        element: &Element,
        document: &Document,
        event_node: &mut VirtualEventElement,
        events: &mut VirtualEvents,
    ) {
        let mut previous_node_was_text = false;

        self.children.iter().for_each(|child| {
            let child_events_node = match child {
                VirtualNode::Text(text_node) => {
                    let current_node = element.as_ref() as &web_sys::Node;

                    // We ensure that the text siblings are patched by preventing the browser from merging
                    // neighboring text nodes. Originally inspired by some of React's work from 2016.
                    //  -> https://reactjs.org/blog/2016/04/07/react-v15.html#major-changes
                    //  -> https://github.com/facebook/react/pull/5753
                    //
                    // `ptns` = Percy text node separator
                    if previous_node_was_text {
                        let separator = document.create_comment("ptns");
                        set_virtual_node_marker(&separator);
                        current_node
                            .append_child(separator.as_ref() as &web_sys::Node)
                            .unwrap();
                    }

                    current_node
                        .append_child(&text_node.create_text_node())
                        .unwrap();

                    previous_node_was_text = true;

                    events.create_text_node()
                }
                VirtualNode::Element(element_node) => {
                    previous_node_was_text = false;

                    let (child, child_events) = element_node.create_element_node(events);
                    let child_elem: Element = child;

                    element.append_child(&child_elem).unwrap();

                    child_events
                }
            };

            let child_events_node = Rc::new(RefCell::new(child_events_node));
            event_node.append_child(child_events_node.clone());
        });
    }
}

impl VElement {
    /// Hydrate an existing DOM element into a Percy-managed tree:
    /// - Marks nodes (`__v__`)
    /// - Assigns events ids (`__events_id__`) and registers handlers
    /// - Builds the VirtualEvents tree without re-appending children
    /// - Handles adjacent VText by splitting an existing Text node (UTF-16 aware)
    pub fn hydrate_element_node_from_dom_element(
        &self,
        events: &mut VirtualEvents,
        element: Element,
    ) -> (Element, VirtualEventNode) {
        let document: Document = element.owner_document().unwrap_or_else(|| {
            web_sys::window().unwrap().document().unwrap()
        });

        // Mark root element once so the patcher recognizes it.
        set_virtual_node_marker(&element.clone().into());

        // Create the VirtualEvents node for this element and assign events id to the real element.
        let mut event_elem_node = events.create_element_node();
        let eid = event_elem_node.as_element().unwrap().events_id();
        set_events_id(&element.clone().into(), events, eid);

        // Collect non-delegated handlers to attach at the end (batching helps perf a bit).
        let mut pending_non_delegated: Vec<(Element, crate::event::EventName, crate::event::EventHandler, crate::event::ElementEventsId)> = Vec::new();

        self.add_events_batched(&element, events, eid, &mut pending_non_delegated);

        // Walk & wire children using sibling traversal (minimizes JS/WASM calls).
        self.hydrate_children_from_dom_fast(
            &document,
            &element,
            event_elem_node.as_element_mut().unwrap(),
            events,
            &mut pending_non_delegated,
        );

        // Attach non-delegated listeners in a tight loop at the end.
        for (el, name, handler, id) in pending_non_delegated {
            crate::event::insert_non_delegated_event(&el, &name, &handler, id, events);
        }

        (element, event_elem_node)
    }

    /// Same semantics as `add_events`, but delay non-delegated attachments.
    fn add_events_batched(
        &self,
        element: &Element,
        events: &VirtualEvents,
        events_id: crate::event::ElementEventsId,
        pending_non_delegated: &mut Vec<(Element, crate::event::EventName, crate::event::EventHandler, crate::event::ElementEventsId)>,
    ) {
        if self.events.has_events() {
            for (onevent, callback) in self.events.events() {
                if onevent.is_delegated() {
                    events.insert_event(events_id, onevent.clone(), callback.clone(), None);
                } else {
                    // Defer attaching; store el + event info.
                    pending_non_delegated.push((
                        element.clone(),
                        onevent.clone(),
                        callback.clone(),
                        events_id,
                    ));
                }
            }
        }
    }

    /// Hydrate children by walking the DOM via first_child/next_sibling, aligning to VDOM.
    /// Splits text nodes when consecutive VText appear.
    fn hydrate_children_from_dom_fast(
        &self,
        document: &Document,
        element: &Element,
        event_node: &mut VirtualEventElement,
        events: &mut VirtualEvents,
        pending_non_delegated: &mut Vec<(Element, crate::event::EventName, crate::event::EventHandler, crate::event::ElementEventsId)>,
    ) {
        // Convert real children to a Vec<Node> using sibling traversal once (reduce boundary calls).
        let mut real_children: Vec<Node> = Vec::new();
        let mut cursor = element.first_child();
        while let Some(n) = cursor {
            // Skip comment nodes (we don't require SSR ptns comments)
            if n.node_type() != Node::COMMENT_NODE {
                real_children.push(n.clone());
            }
            cursor = n.next_sibling();
        }

        // Two cursors, one for VDOM children, one for real DOM children.
        let mut di = 0usize;

        // Helper: split a Text node at a UTF-16 offset and return (left, right).
        fn split_text_at_utf16(text: &Text, utf16_count: usize) -> Option<(Text, Text)> {
            let data = text.data();
            // DOM's length is in UTF-16 code units; we need a char boundary in UTF-16 space.
            let total = data.encode_utf16().count();
            if utf16_count >= total { return None; }
            // The DOM API split_text takes code unit index
            let right = text.split_text(utf16_count as u32).ok()?;
            let left = text.clone();
            Some((left, right))
        }

        // Iterate VDOM children
        for (vi, vchild) in self.children.iter().enumerate() {
            // Advance real DOM cursor to next supported node type if needed
            while di < real_children.len()
                && real_children[di].node_type() == Node::COMMENT_NODE
            {
                di += 1;
            }
            if di >= real_children.len() {
                // If we run out of real nodes: only VText is recoverable (create empty text)
                if let VirtualNode::Text(_vt) = vchild {
                    let t = document.create_text_node("");
                    set_virtual_node_marker(&(t.as_ref() as &Node).into());
                    element
                        .append_child(t.as_ref() as &Node)
                        .expect("append empty text during hydrate");

                    let child_events_node = events.create_text_node();
                    event_node.append_child(Rc::new(RefCell::new(child_events_node)));
                    // di now points before the appended node; we won't need it again anyway.
                    continue;
                } else {
                    // Element missing: fail fast (or you can choose to create it).
                    debug_assert!(false, "Percy hydrate: missing DOM element for VElement");
                    return;
                }
            }

            let target = &real_children[di];

            match vchild {
                VirtualNode::Text(vt) => {
                    // If target is element, try to look ahead for a text node; if none, create empty text before target.
                    if target.node_type() != Node::TEXT_NODE {
                        // Insert an empty text node before the current target to maintain order
                        let t = document.create_text_node("");
                        set_virtual_node_marker(&(t.as_ref() as &Node).into());
                        element
                            .insert_before(t.as_ref() as &Node, Some(target))
                            .expect("insert empty text during hydrate");

                        let child_events_node = events.create_text_node();
                        event_node.append_child(Rc::new(RefCell::new(child_events_node)));
                        // Don't advance di; we'll process the same target again for the next VChild
                        continue;
                    }

                    // We have a real text node. Make sure it's marked and aligned to this VText.
                    let t: Text = target.clone().unchecked_into();
                    set_virtual_node_marker(&(t.as_ref() as &Node).into());

                    // If the next VChild is also Text, split once so each VText maps to its own real node.
                    let next_is_text = self
                        .children
                        .get(vi + 1)
                        .map(|n| matches!(n, VirtualNode::Text(_)))
                        .unwrap_or(false);

                    if next_is_text {
                        // Split at boundary = utf16 length of current vt
                        let split_at = vt.text.encode_utf16().count();

                        if split_at > 0 {
                            if let Some((_left, right)) = split_text_at_utf16(&t, split_at) {
                                // Insert the newly created right sibling into our real_children Vec after 'di'
                                let right_node: Node = right.unchecked_into::<Node>();
                                real_children.insert(di + 1, right_node);
                            }
                        } else {
                            // If this VText is empty, we keep the node as-is; the next iteration can create/split as needed.
                        }
                    }

                    // Events tree node for text
                    let child_events_node = events.create_text_node();
                    event_node.append_child(Rc::new(RefCell::new(child_events_node)));

                    di += 1;
                }
                VirtualNode::Element(velem) => {
                    if target.node_type() != Node::ELEMENT_NODE {
                        // Scan ahead for an element; cheaper than per-iteration DOM calls because we use our Vec snapshot
                        let mut found = None;
                        for j in di + 1..real_children.len() {
                            if real_children[j].node_type() == Node::ELEMENT_NODE {
                                found = Some(j);
                                break;
                            }
                        }
                        if let Some(j) = found {
                            di = j;
                        } else {
                            debug_assert!(false, "Percy hydrate: could not find element for VElement");
                            return;
                        }
                    }

                    let real_elem: Element = real_children[di].clone().unchecked_into();
                    set_virtual_node_marker(&real_elem.clone().into());

                    // Recurse to hydrate subtree
                    let (_again, child_events_node) =
                        velem.hydrate_element_node_from_dom_element(events, real_elem);
                    event_node.append_child(Rc::new(RefCell::new(child_events_node)));

                    di += 1;
                }
            }
        }
    }
}

/// Set a property on a node that can be used to know if a node was created by Percy.
pub(crate) fn set_virtual_node_marker(node: &JsValue) {
    let unused_data = 123;

    Reflect::set(
        &node.into(),
        &VIRTUAL_NODE_MARKER_PROPERTY.into(),
        &unused_data.into(),
    )
    .unwrap();
}
