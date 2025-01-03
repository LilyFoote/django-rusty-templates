use pyo3::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub struct TemplateString<'t>(pub &'t str);

impl<'t> TemplateString<'t> {
    pub fn content(&self, at: (usize, usize)) -> &'t str {
        let (start, len) = at;
        &self.0[start..start + len]
    }
}

impl<'t> From<&'t str> for TemplateString<'t> {
    fn from(value: &'t str) -> Self {
        TemplateString(value)
    }
}

pub trait CloneRef {
    fn clone_ref(&self, py: Python<'_>) -> Self;
}

impl<T> CloneRef for Vec<T>
where
    T: CloneRef,
{
    fn clone_ref(&self, py: Python<'_>) -> Self {
        self.iter().map(|element| element.clone_ref(py)).collect()
    }
}

impl<K, V> CloneRef for HashMap<K, V>
where
    K: Clone + Eq + std::hash::Hash,
    V: CloneRef,
{
    fn clone_ref(&self, py: Python<'_>) -> Self {
        self.iter()
            .map(|(k, v)| (k.clone(), v.clone_ref(py)))
            .collect()
    }
}

impl<K, V> CloneRef for Vec<(K, V)>
where
    K: Clone,
    V: CloneRef,
{
    fn clone_ref(&self, py: Python<'_>) -> Self {
        self.iter()
            .map(|(k, v)| (k.clone(), v.clone_ref(py)))
            .collect()
    }
}
