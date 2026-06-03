use crate::reader::{BitsReader, SliceReader};
use crate::stream::field_path::{FieldOp, FIELD_OPS};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub(crate) struct FieldPathCodec {
    tree: Rc<FieldPathTree>,
}

impl Default for FieldPathCodec {
    fn default() -> Self {
        Self {
            tree: Rc::new(FieldPathTree::default()),
        }
    }
}

impl FieldPathCodec {
    #[inline]
    pub(crate) fn read_op(&self, reader: &mut SliceReader) -> FieldOp {
        reader.refill();
        let mut node = self.tree.as_ref();
        loop {
            node = if reader.read_bool() {
                node.right()
            } else {
                node.left()
            };
            if let FieldPathTree::Leaf { value, .. } = node {
                return FIELD_OPS[*value as usize].0;
            }
        }
    }
}

#[derive(Clone, Debug)]
enum FieldPathTree {
    Leaf {
        weight: u32,
        value: u32,
    },
    Node {
        weight: u32,
        value: u32,
        left: Box<FieldPathTree>,
        right: Box<FieldPathTree>,
    },
}

impl Default for FieldPathTree {
    fn default() -> Self {
        let mut trees = FIELD_OPS
            .iter()
            .map(|(_, weight)| weight)
            .enumerate()
            .map(|(v, &w)| FieldPathTree::Leaf {
                value: v as u32,
                weight: if w == 0 { 1 } else { w },
            })
            .collect::<BinaryHeap<FieldPathTree>>();
        let mut n = 40;
        while let (Some(a), Some(b)) = (trees.pop(), trees.pop()) {
            trees.push(FieldPathTree::Node {
                weight: a.weight() + b.weight(),
                value: n,
                left: a.into(),
                right: b.into(),
            });
            n += 1;
            if trees.len() == 1 {
                break;
            }
        }
        trees.pop().unwrap()
    }
}

impl FieldPathTree {
    fn weight(&self) -> u32 {
        match self {
            FieldPathTree::Leaf { weight, .. } | FieldPathTree::Node { weight, .. } => *weight,
        }
    }
    fn value(&self) -> u32 {
        match self {
            FieldPathTree::Leaf { value, .. } | FieldPathTree::Node { value, .. } => *value,
        }
    }
    fn left(&self) -> &FieldPathTree {
        match self {
            FieldPathTree::Node { left, .. } => left,
            FieldPathTree::Leaf { .. } => unreachable!(),
        }
    }
    fn right(&self) -> &FieldPathTree {
        match self {
            FieldPathTree::Node { right, .. } => right,
            FieldPathTree::Leaf { .. } => unreachable!(),
        }
    }
}

impl PartialEq for FieldPathTree {
    fn eq(&self, other: &Self) -> bool {
        self.weight() == other.weight() && self.value() == other.value()
    }
}
impl Eq for FieldPathTree {}
impl PartialOrd for FieldPathTree {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for FieldPathTree {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.weight().cmp(&other.weight()) {
            Ordering::Equal => self.value().cmp(&other.value()),
            ord => ord.reverse(),
        }
    }
}
