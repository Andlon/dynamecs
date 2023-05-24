use std::cell::RefCell;
use std::fmt::{Debug, Formatter};
use std::rc::Rc;
use crate::SpanPath;

pub struct SpanTreeNode2<Payload> {
    payload: Payload,
    children: Vec<Rc<RefCell<SpanTreeNode2<Payload>>>>,
    parent: Option<Rc<RefCell<SpanTreeNode2<Payload>>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanTree<Payload> {
    // Stored in depth-first order
    tree_depth_first: Vec<SpanPath>,
    payloads: Vec<Payload>,
    // TODO: Precompute children indices so that we can just skip directly to
    // relevant indices
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum InvalidTreeLayout {
    Empty,
    NotDepthFirst,
    NotTree,
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

    pub fn try_from_depth_first(paths: Vec<SpanPath>, payloads: Vec<Payload>) -> Result<Self, InvalidTreeLayout> {
        assert_eq!(paths.len(), payloads.len());

        let (root, other_paths) = paths.split_first().ok_or_else(|| InvalidTreeLayout::Empty)?;
        for path in other_paths {
            if !root.is_ancestor_of(path) {
                return Err(InvalidTreeLayout::NotTree);
            }
        }





        // for pair in paths.windows(2) {
        //     let [path1, path2]: &[SpanPath; 2] = pair.try_into().unwrap();
        //     if path1.
        //         return Err(InvalidTreeLayout::NotDepthFirst)
        //     }
        // }
        Ok(Self { tree_depth_first: paths, payloads })
    }

    pub fn from_paths_and_payloads(paths: Vec<SpanPath>, payloads: Vec<Payload>) -> Self {
        // TODO: Verify that we have a tree, not a forest!!!
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
            .take_while(move |(_, maybe_child)| self_path1.is_parent_of(maybe_child))
            .filter(move |(_, descendant)| self_path2.is_parent_of(descendant))
            .map(move |(child_index, _)| SpanTreeNode {
                tree_depth_first,
                payloads,
                index: child_index,
            })
    }
}
