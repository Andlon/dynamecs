use std::fmt::{Debug, Display, Formatter};
use itertools::izip;
use crate::SpanPath;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanTree<Payload> {
    // Stored in depth-first order
    tree_depth_first: Vec<SpanPath>,
    payloads: Vec<Payload>,
    // TODO: Precompute children indices so that we can just skip directly to
    // relevant indices
}

#[derive(Debug, Clone)]
pub struct SpanTreeError {
    message: String,
}

impl Display for SpanTreeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SpanTreeError {}

impl SpanTreeError {
    fn message(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

impl<Payload> SpanTree<Payload> {
    pub fn root(&self) -> SpanTreeNode<Payload> {
        // TODO: Ensure that the first entry is always the root in the tree
        debug_assert_eq!(self.tree_depth_first.len(), self.payloads.len());
        assert!(self.tree_depth_first.len() > 0);
        SpanTreeNode {
            tree_depth_first: &self.tree_depth_first,
            payloads: &self.payloads,
            index: 0,
        }
    }

    pub fn try_from_depth_first_ordering(paths: Vec<SpanPath>, payloads: Vec<Payload>) -> Result<Self, SpanTreeError> {
        let (root, others) = paths.split_first()
            // TODO: Should we support empty trees?
            .ok_or_else(|| SpanTreeError::message("there must be at least one path in the tree"))?;
        let mut stack = Vec::new();
        for name in root.span_names() {
            stack.push(name.as_str());
        }

        for path in others {
            let num_common_names = izip!(&stack, path.span_names())
                .take_while(|(&stack_name, path_name)| stack_name == path_name.as_str())
                .count();

            if num_common_names < root.depth() {
                return Err(SpanTreeError::message("first path is not an ancestor of all other nodes"));
            }

            stack.truncate(num_common_names);

            if path.depth() > num_common_names + 1 {
                return Err(SpanTreeError::message("intermediate nodes missing"));
            } else if path.depth() == num_common_names + 1 {
                stack.push(path.span_name().unwrap());
            } else if path.depth() == num_common_names {
                return Err(SpanTreeError::message("duplicate paths detected"));
            } else if path.depth() < num_common_names {
                unreachable!()
            }
        }

        assert_eq!(paths.len(), payloads.len());
        Ok(Self {
            tree_depth_first: paths,
            payloads
        })
    }

    pub fn from_paths_and_payloads(paths: Vec<SpanPath>, payloads: Vec<Payload>) -> Self {
        assert_eq!(paths.len(), payloads.len());
        let mut path_payload_pairs: Vec<_> = paths.into_iter()
            .zip(payloads)
            .collect();
        path_payload_pairs.sort_by(|pair1, pair2| pair1.0.span_names().cmp(pair2.0.span_names()));
        let (tree_depth_first, payloads) = path_payload_pairs
            .into_iter()
            .unzip();
        Self {
            tree_depth_first,
            payloads
        }
    }

    /// Return an identical tree in which the payload associated with each node
    /// is transformed by the provided transformation function.
    pub fn transform_payloads<Payload2>(&mut self,
                                        transform: impl FnMut(SpanTreeNode<Payload>) -> Payload2)
        -> SpanTree<Payload2> {
        let new_payloads: Vec<_> = (0 .. self.tree_depth_first.len())
            .map(|i| SpanTreeNode {
                tree_depth_first: &self.tree_depth_first,
                payloads: &self.payloads,
                index: i,
            }).map(transform)
            .collect();

        SpanTree::from_paths_and_payloads(self.tree_depth_first.clone(), new_payloads)
    }
}

pub struct SpanTreeNode<'a, Payload> {
    tree_depth_first: &'a [SpanPath],
    payloads: &'a [Payload],
    index: usize,
}

impl<'a, Payload> Debug for SpanTreeNode<'a, Payload>
where
    Payload: Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let Self { tree_depth_first, payloads, index } = self;
        f.debug_struct("SpanTreeNode")
            .field("path", &tree_depth_first[*index])
            .field("payload", &payloads[*index])
            .finish()
    }
}

impl<'a, Payload> SpanTreeNode<'a, Payload> {
    pub fn payload(&self) -> &Payload {
        &self.payloads[self.index]
    }

    pub fn path(&self) -> SpanPath {
        self.tree_depth_first[self.index].clone()
    }

    pub fn count_children(&self) -> usize {
        self.visit_children().count()
    }

    pub fn root(&self) -> SpanTreeNode<'a, Payload> {
        SpanTreeNode {
            index: 0,
            .. *self
        }
    }

    pub fn parent(&self) -> Option<SpanTreeNode<'a, Payload>> {
        self.path()
            .parent()
            .and_then(|parent_path| {
                self.tree_depth_first[.. self.index].binary_search_by_key(
                    &parent_path.span_names(),
                    |path| path.span_names()).ok()
                    .map(|index| {
                        SpanTreeNode {
                            tree_depth_first: self.tree_depth_first,
                            payloads: self.payloads,
                            index,
                        }
                    })
            })
    }

    pub fn visit_children(&self) -> impl Iterator<Item=SpanTreeNode<'a, Payload>> {
        // This is just for type inference, to make sure that we get the 'a lifetime
        // and not something tied to 'self
        let tree_depth_first: &'a [SpanPath] = self.tree_depth_first;
        let payloads: &'a [Payload] = self.payloads;

        // TODO: Fix this. It's a temporary workaround for the fact that we cannot move
        // in the same SpanPath to two different closures, since it's not Copy.
        // Might want to split SpanPath into SpanPathBuf and SpanPath or something like that
        let self_path1 = self.path();
        let self_path2 = self_path1.clone();

        // TODO: Use exponential search to avoid accidental complexity explosion for
        // very large trees? (It seems unlikely that anyone will have a tree large enough
        // to make a significant difference though)
        self.tree_depth_first.iter()
            .enumerate()
            // Start at the first potential child
            .skip(self.index + 1)
            .take_while(move |(_, maybe_child)| self_path1.is_ancestor_of(maybe_child))
            .filter(move |(_, descendant)| self_path2.is_parent_of(descendant))
            .map(move |(child_index, _)| SpanTreeNode {
                tree_depth_first,
                payloads,
                index: child_index,
            })
    }
}
