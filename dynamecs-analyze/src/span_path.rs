use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpanPath {
    span_names: Vec<String>,
}

impl SpanPath {
    pub const fn new(span_names: Vec<String>) -> Self {
        Self { span_names }
    }

    pub fn span_name(&self) -> Option<&str> {
        self.span_names.last().map(String::as_str)
    }

    pub fn span_names(&self) -> &[String] {
        self.span_names.as_ref()
    }

    /// The number of span names that make up this span path.
    pub fn depth(&self) -> usize {
        self.span_names.len()
    }

    pub fn parent(&self) -> Option<SpanPath> {
        let n = self.span_names().len();
        (n > 0).then(|| SpanPath::new(self.span_names[0..(n - 1)].to_vec()))
    }

    pub fn is_parent_of(&self, other: &SpanPath) -> bool {
        let n = self
            .span_names()
            .iter()
            .zip(other.span_names())
            .take_while(|(self_name, other_name)| self_name == other_name)
            .count();
        n == self.span_names().len() && n + 1 == other.span_names().len()
    }

    /// Determines if this path is an ancestor of another path.
    ///
    /// A path is an ancestor of itself.
    pub fn is_ancestor_of(&self, other: &SpanPath) -> bool {
        let n = self
            .span_names()
            .iter()
            .zip(other.span_names())
            .take_while(|(self_name, other_name)| self_name == other_name)
            .count();
        n == self.span_names().len()
    }

    /// Determines the common ancestor of this path and another path.
    ///
    /// A path is an ancestor of itself.
    pub fn common_ancestor(&self, other: &SpanPath) -> SpanPath {
        let common_span_names = self
            .span_names()
            .iter()
            .zip(other.span_names())
            .map_while(|(self_name, other_name)| (self_name == other_name).then(|| self_name))
            .cloned()
            .collect();
        SpanPath::new(common_span_names)
    }

    pub fn push_span_name(&mut self, span_name: String) {
        self.span_names.push(span_name);
    }
}

impl Display for SpanPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some((first, rest)) = self.span_names().split_first() {
            write!(f, "{first}")?;
            for name in rest {
                write!(f, ">{}", name)?;
            }
        }
        Ok(())
    }
}
