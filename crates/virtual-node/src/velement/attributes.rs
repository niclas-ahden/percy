use super::AttributeValue;

/// An element's HTML attributes.
///
/// Backed by a `Vec` rather than a `HashMap`: elements almost always have just a handful
/// of attributes (often one or two), for which a small vector is cheaper to build and to
/// compare than hashing `String` keys. Building and comparing attribute collections is a
/// hot path in both virtual-DOM construction and diffing, so this matters at scale.
///
/// Equality is order-insensitive (set semantics over key/value pairs), matching the
/// `HashMap` this replaced, so diffing behaviour is unchanged. Keys are unique: `insert`
/// replaces any existing value for the same key.
#[derive(Clone, Debug, Default)]
pub struct Attributes {
    entries: Vec<(String, AttributeValue)>,
}

impl Attributes {
    /// Create an empty attribute collection.
    pub fn new() -> Self {
        Attributes {
            entries: Vec::new(),
        }
    }

    /// Insert an attribute, replacing (and returning) any existing value for the same key.
    /// Mirrors `HashMap::insert`.
    pub fn insert(&mut self, key: String, value: AttributeValue) -> Option<AttributeValue> {
        for entry in self.entries.iter_mut() {
            if entry.0 == key {
                return Some(std::mem::replace(&mut entry.1, value));
            }
        }
        self.entries.push((key, value));
        None
    }

    /// Get the value associated with `key`, if present.
    pub fn get<Q: AsRef<str>>(&self, key: Q) -> Option<&AttributeValue> {
        let key = key.as_ref();
        for (k, v) in &self.entries {
            if k.as_str() == key {
                return Some(v);
            }
        }
        None
    }

    /// Iterate over `(name, value)` pairs, matching `HashMap::iter`.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &AttributeValue)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }

    /// Number of attributes.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether there are no attributes.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl PartialEq for Attributes {
    fn eq(&self, other: &Self) -> bool {
        self.entries.len() == other.entries.len()
            && self
                .entries
                .iter()
                .all(|(k, v)| other.get(k.as_str()) == Some(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> AttributeValue {
        AttributeValue::String(v.to_string())
    }

    #[test]
    fn insert_get() {
        let mut a = Attributes::new();
        assert_eq!(a.insert("class".into(), s("row")), None);
        assert_eq!(a.get("class"), Some(&s("row")));
        assert_eq!(a.get("missing"), None);
        assert_eq!(a.len(), 1);
    }

    #[test]
    fn insert_replaces_existing_key() {
        let mut a = Attributes::new();
        a.insert("class".into(), s("row"));
        // Replacing returns the previous value and does not grow the collection.
        assert_eq!(a.insert("class".into(), s("row active")), Some(s("row")));
        assert_eq!(a.get("class"), Some(&s("row active")));
        assert_eq!(a.len(), 1);
    }

    #[test]
    fn equality_is_order_insensitive() {
        let mut a = Attributes::new();
        a.insert("class".into(), s("row"));
        a.insert("id".into(), s("1"));

        let mut b = Attributes::new();
        b.insert("id".into(), s("1"));
        b.insert("class".into(), s("row"));

        assert_eq!(a, b);
    }

    #[test]
    fn inequality_on_value_and_length() {
        let mut a = Attributes::new();
        a.insert("class".into(), s("row"));

        let mut different_value = Attributes::new();
        different_value.insert("class".into(), s("col"));
        assert_ne!(a, different_value);

        let mut extra_key = Attributes::new();
        extra_key.insert("class".into(), s("row"));
        extra_key.insert("id".into(), s("1"));
        assert_ne!(a, extra_key);
    }
}
