use num_bigint::BigInt;
use pyo3::prelude::*;

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

struct PartsIterator<'t> {
    variable: &'t str,
    start: usize,
}

impl<'t> Iterator for PartsIterator<'t> {
    type Item = (&'t str, (usize, usize));

    fn next(&mut self) -> Option<Self::Item> {
        if self.variable.is_empty() {
            return None;
        }

        match self.variable.find('.') {
            Some(index) => {
                let part = &self.variable[..index];
                let at = (self.start, index);
                self.start += index + 1;
                self.variable = &self.variable[index + 1..];
                Some((part, at))
            }
            None => {
                let part = self.variable;
                self.variable = "";
                Some((part, (self.start, part.len())))
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Text {
    pub at: (usize, usize),
}

impl Text {
    pub fn new(at: (usize, usize)) -> Self {
        Self { at }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Variable {
    pub at: (usize, usize),
}

impl<'t> Variable {
    pub fn new(at: (usize, usize)) -> Self {
        Self { at }
    }

    pub fn parts(
        &self,
        template: TemplateString<'t>,
    ) -> impl Iterator<Item = (&'t str, (usize, usize))> {
        let start = self.at.0;
        let variable = template.content(self.at);
        PartsIterator { variable, start }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ArgumentType {
    Variable(Variable),
    Text(Text),
    TranslatedText(Text),
    Int(BigInt),
    Float(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Argument {
    pub at: (usize, usize),
    pub argument_type: ArgumentType,
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

#[cfg(test)]
pub trait PyEq {
    fn py_eq(&self, other: &Self, py: Python<'_>) -> bool;
}

#[cfg(test)]
impl<T> PyEq for Vec<T>
where
    T: PyEq,
{
    fn py_eq(&self, other: &Self, py: Python<'_>) -> bool {
        if self.len() != other.len() {
            return false;
        }
        for (l, r) in self.iter().zip(other.iter()) {
            if !l.py_eq(r, py) {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
impl<K, V> PyEq for Vec<(K, V)>
where
    K: Eq,
    V: PyEq,
{
    fn py_eq(&self, other: &Self, py: Python<'_>) -> bool {
        if self.len() != other.len() {
            return false;
        }
        for ((l1, l2), (r1, r2)) in self.iter().zip(other.iter()) {
            if l1 != r1 || !l2.py_eq(r2, py) {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::parse::TagElement;

    #[test]
    fn test_vec_ne() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            assert!(!vec![TagElement::Int(1.into())].py_eq(&vec![TagElement::Float(1.0)], py))
        })
    }

    #[test]
    fn test_vec_ne_different_lengths() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            assert!(!vec![TagElement::Int(1.into()), TagElement::Float(2.3)]
                .py_eq(&vec![TagElement::Int(1.into())], py));
            assert!(!vec![TagElement::Int(1.into())]
                .py_eq(&vec![TagElement::Int(1.into()), TagElement::Float(2.3)], py));
        })
    }

    #[test]
    fn test_vec_tuple_ne() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            assert!(!vec![
                ("foo", TagElement::Int(1.into())),
                ("bar", TagElement::Int(2.into()))
            ]
            .py_eq(
                &vec![
                    ("foo", TagElement::Int(1.into())),
                    ("bar", TagElement::Float(1.0))
                ],
                py
            ))
        })
    }

    #[test]
    fn test_vec_tuple_ne_different_lengths() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            assert!(!vec![
                ("foo", TagElement::Int(1.into())),
                ("bar", TagElement::Float(2.3))
            ]
            .py_eq(&vec![("foo", TagElement::Int(1.into()))], py));
            assert!(!vec![("foo", TagElement::Int(1.into()))].py_eq(
                &vec![
                    ("foo", TagElement::Int(1.into())),
                    ("bar", TagElement::Float(2.3))
                ],
                py
            ));
        })
    }
}
