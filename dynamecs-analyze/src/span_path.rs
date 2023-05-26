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

    pub fn parent(&self) -> Option<SpanPath> {
        let n = self.span_names().len();
        (n > 0).then(|| SpanPath::new(self.span_names[0 .. (n - 1)].to_vec()))
    }

    pub fn is_parent_of(&self, other: &SpanPath) -> bool {
        other.ancestor_names()
            .map(|ancestor_names| self.span_names() == ancestor_names)
            .unwrap_or(false)
    }

    pub fn is_ancestor_of(&self, other: &SpanPath) -> bool {
        other.ancestor_names()
            .map(|ancestor_names| {
                let n = self.span_names.len();
                n <= ancestor_names.len() && self.span_names() == &other.span_names()[..n]
            }).unwrap_or(false)
    }

    fn ancestor_names(&self) -> Option<&[String]> {
        match self.span_names.len() {
            0 => None,
            n => Some(&self.span_names[..(n - 1)])
        }
    }

    pub fn common_ancestor(&self, other: &SpanPath) -> Option<SpanPath> {
        self.ancestor_names()
            .zip(other.ancestor_names())
            .map(|(self_ancestor_names, other_ancestor_names)| {
                let n_common = self_ancestor_names.iter()
                    .zip(other_ancestor_names)
                    .take_while(|(span1, span2)| span1 == span2)
                    .count();
                SpanPath::new(self.span_names()[..n_common].to_vec())
            })
    }

    pub fn push_span_name(&mut self, span_name: String) {
        self.span_names.push(span_name);
    }
}

impl Display for SpanPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for name in self.span_names() {
            write!(f, "{} > ", name)?;
        }
        Ok(())
    }
}
