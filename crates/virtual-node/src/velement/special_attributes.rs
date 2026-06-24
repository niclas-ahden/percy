use std::any::Any;
use std::borrow::Cow;
use std::cell::RefCell;
use std::ops::DerefMut;

/// A specially supported attributes.
#[derive(Default, PartialEq)]
pub struct SpecialAttributes {
    /// A a function that gets called when the virtual node is first turned into a real node.
    ///
    /// See [`SpecialAttributes.set_on_create_element`] for more documentation.
    on_create_element: Option<KeyAndElementFn>,
    /// A a function that gets called when the virtual node is first turned into a real node.
    ///
    /// See [`SpecialAttributes.set_on_remove_element`] for more documentation.
    on_remove_element: Option<KeyAndElementFn>,
    /// A node-lifetime effect: a `setup` that runs when the element is created (including on
    /// hydration) and produces a piece of state, and a `teardown` that runs with that state
    /// when the element is removed. See [`SpecialAttributes.set_element_effect`].
    ///
    /// Boxed so the common case (no effect) costs one null pointer per element instead of a
    /// full inline `ElementEffect`. Effects are rare but every `VElement` carries this field.
    /// The allocation only happens for the nodes that set one.
    element_effect: Option<Box<ElementEffect>>,
    /// Allows setting the innerHTML of an element.
    ///
    /// # Danger
    ///
    /// Be sure to escape all untrusted input to avoid cross site scripting attacks.
    pub dangerous_inner_html: Option<String>,
}

impl SpecialAttributes {
    /// The key for the on create element function
    pub fn on_create_element_key(&self) -> Option<&Cow<'static, str>> {
        self.on_create_element.as_ref().map(|k| &k.key)
    }

    /// Set the [`SpecialAttributes.on_create_element`] function.
    ///
    /// # Key
    ///
    /// The key is used when one virtual-node is being patched over another.
    ///
    /// If the new node's key is different from the old node's key, the on create element function
    /// gets called.
    ///
    /// If the keys are the same, the function does not get called.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use virtual_node::VirtualNode;
    /// use wasm_bindgen::JsValue;
    ///
    /// let mut node = VirtualNode::element("div");
    ///
    /// // A key can be any `Into<Cow<'static, str>>`.
    /// let key = "some-key";
    ///
    /// let on_create_elem = move |elem: web_sys::Element| {
    ///     assert_eq!(elem.id(), "");
    /// };
    ///
    /// node
    ///     .as_velement_mut()
    ///     .unwrap()
    ///     .special_attributes
    ///     .set_on_create_element(key, on_create_elem);
    /// ```
    pub fn set_on_create_element<Key, Func>(&mut self, key: Key, func: Func)
    where
        Key: Into<Cow<'static, str>>,
        Func: FnMut(web_sys::Element) + 'static,
    {
        self.on_create_element = Some(KeyAndElementFn {
            key: key.into(),
            func: RefCell::new(ElementFunc::OneArg(Box::new(func))),
        });
    }

    // Used by the html-macro
    #[doc(hidden)]
    pub fn set_on_create_element_no_args<Key, Func>(&mut self, key: Key, func: Func)
    where
        Key: Into<Cow<'static, str>>,
        Func: FnMut() + 'static,
    {
        self.on_create_element = Some(KeyAndElementFn {
            key: key.into(),
            func: RefCell::new(ElementFunc::NoArgs(Box::new(func))),
        });
    }

    /// If an `on_create_element` function was set, call it.
    pub fn maybe_call_on_create_element(&self, element: &web_sys::Element) {
        if let Some(on_create_elem) = &self.on_create_element {
            on_create_elem.call(element.clone());
        }

        let _ = element;
    }
}

impl SpecialAttributes {
    /// The key for the on remove element function
    pub fn on_remove_element_key(&self) -> Option<&Cow<'static, str>> {
        self.on_remove_element.as_ref().map(|k| &k.key)
    }

    /// Set the [`SpecialAttributes.on_remove_element`] function.
    ///
    /// # Key
    ///
    /// The key is used when one virtual-node is being patched over another.
    ///
    /// If the old node's key is different from the new node's key, the on remove element function
    /// gets called for the old element.
    ///
    /// If the keys are the same, the function does not get called.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use virtual_node::VirtualNode;
    /// use wasm_bindgen::JsValue;
    ///
    /// let mut node = VirtualNode::element("div");
    ///
    /// // A key can be any `Into<Cow<'static, str>>`.
    /// let key = "some-key";
    ///
    /// let on_remove_elem = move |elem: web_sys::Element| {
    ///     assert_eq!(elem.id(), "");
    /// };
    ///
    /// node
    ///     .as_velement_mut()
    ///     .unwrap()
    ///     .special_attributes
    ///     .set_on_remove_element(key, on_remove_elem);
    /// ```
    pub fn set_on_remove_element<Key, Func>(&mut self, key: Key, func: Func)
    where
        Key: Into<Cow<'static, str>>,
        Func: FnMut(web_sys::Element) + 'static,
    {
        self.on_remove_element = Some(KeyAndElementFn {
            key: key.into(),
            func: RefCell::new(ElementFunc::OneArg(Box::new(func))),
        });
    }

    // Used by the html-macro
    #[doc(hidden)]
    pub fn set_on_remove_element_no_args<Key, Func>(&mut self, key: Key, func: Func)
    where
        Key: Into<Cow<'static, str>>,
        Func: FnMut() + 'static,
    {
        self.on_remove_element = Some(KeyAndElementFn {
            key: key.into(),
            func: RefCell::new(ElementFunc::NoArgs(Box::new(func))),
        });
    }

    /// If an `on_remove_element` function was set, call it.
    pub fn maybe_call_on_remove_element(&self, element: &web_sys::Element) {
        if let Some(on_remove_elem) = &self.on_remove_element {
            on_remove_elem.call(element.clone());
        }

        let _ = element;
    }
}

impl SpecialAttributes {
    /// Whether no special attribute is set (no create/remove hook, no element effect, no
    /// `dangerous_inner_html`).
    ///
    /// Most elements carry none of these, so `diff` checks this once per node and skips the
    /// four per-kind diffs below when both the old and new node are empty. The common case
    /// becomes a single branch instead of four reads and matches.
    pub fn is_empty(&self) -> bool {
        self.on_create_element.is_none()
            && self.on_remove_element.is_none()
            && self.element_effect.is_none()
            && self.dangerous_inner_html.is_none()
    }

    /// The key (dependencies) for the element effect, if one is set.
    pub fn element_effect_key(&self) -> Option<&Cow<'static, str>> {
        self.element_effect.as_ref().map(|e| &e.key)
    }

    /// Set a node-lifetime effect, modeled on a `useEffect` with dependencies.
    ///
    /// `setup` runs when the element is turned into a real DOM node (on fresh create and on
    /// hydration) and returns a piece of `State`. The renderer stores that state for the
    /// lifetime of the element. `teardown` runs with the stored state when the element is
    /// removed from the DOM, including when an ancestor is removed. An element uses this to
    /// own an imperative browser resource such as an `IntersectionObserver` whose lifetime
    /// matches the element's, with no global bookkeeping.
    ///
    /// # Key (dependencies)
    ///
    /// The key behaves like a `useEffect` dependency list. When a node is patched over a
    /// previous node, an equal key leaves the effect and its state untouched. A different
    /// key runs `teardown` with the old state, then runs `setup` again and stores the new
    /// state. That is one re-run on the same element, so a resource can be rebuilt against
    /// new inputs without recreating the element.
    ///
    /// An `IntersectionObserver` only fires on a crossing. For infinite scroll the key is
    /// bumped when the data changes so the effect re-runs and re-delivers the current state.
    pub fn set_element_effect<Key, State, Setup, Teardown>(
        &mut self,
        key: Key,
        setup: Setup,
        teardown: Teardown,
    ) where
        Key: Into<Cow<'static, str>>,
        State: 'static,
        Setup: Fn(web_sys::Element) -> State + 'static,
        Teardown: Fn(State) + 'static,
    {
        let setup: Box<dyn Fn(web_sys::Element) -> Box<dyn Any>> =
            Box::new(move |element| Box::new(setup(element)) as Box<dyn Any>);
        let teardown: Box<dyn Fn(Box<dyn Any>)> = Box::new(move |state| {
            // State always comes from this effect's own `setup`, so the downcast succeeds.
            // If it ever did not, drop the box rather than panic during a patch.
            if let Ok(state) = state.downcast::<State>() {
                teardown(*state);
            }
        });
        self.element_effect = Some(Box::new(ElementEffect {
            key: key.into(),
            setup,
            teardown,
        }));
    }

    /// Run the effect's `setup` against `element`, returning the state it produced (which the
    /// renderer stores). Returns `None` if no effect is set.
    pub fn call_effect_setup(&self, element: &web_sys::Element) -> Option<Box<dyn Any>> {
        self.element_effect
            .as_ref()
            .map(|effect| (effect.setup)(element.clone()))
    }

    /// Run the effect's `teardown` with the previously stored `state`. No-op if no effect is set.
    pub fn call_effect_teardown(&self, state: Box<dyn Any>) {
        if let Some(effect) = &self.element_effect {
            (effect.teardown)(state);
        }
    }
}

struct ElementEffect {
    key: Cow<'static, str>,
    setup: Box<dyn Fn(web_sys::Element) -> Box<dyn Any>>,
    teardown: Box<dyn Fn(Box<dyn Any>)>,
}

impl PartialEq for ElementEffect {
    fn eq(&self, rhs: &Self) -> bool {
        self.key == rhs.key
    }
}

struct KeyAndElementFn {
    key: Cow<'static, str>,
    func: RefCell<ElementFunc>,
}

enum ElementFunc {
    NoArgs(Box<dyn FnMut()>),
    OneArg(Box<dyn FnMut(web_sys::Element)>),
}

impl KeyAndElementFn {
    fn call(&self, element: web_sys::Element) {
        match self.func.borrow_mut().deref_mut() {
            ElementFunc::NoArgs(func) => (func)(),
            ElementFunc::OneArg(func) => (func)(element),
        };
    }
}

impl PartialEq for KeyAndElementFn {
    fn eq(&self, rhs: &Self) -> bool {
        self.key == rhs.key
    }
}
