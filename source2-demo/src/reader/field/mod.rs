mod huff;
mod op;

use huff::*;
use op::*;

use crate::field::{FieldPath, FieldState, Serializer};
use crate::reader::{BitsReader, SliceReader};

pub(crate) struct FieldReader {
    tree: HTree,
    paths_buf: [FieldPath; 8192],
}

impl Default for FieldReader {
    fn default() -> Self {
        FieldReader {
            tree: HTree::default(),
            paths_buf: [FieldPath::default(); 8192],
        }
    }
}

impl FieldReader {
    #[inline]
    pub(crate) fn read_fields(
        &mut self,
        reader: &mut SliceReader,
        serializer: &Serializer,
        state: &mut FieldState,
    ) -> usize {
        let mut node = &self.tree;
        let mut i = 0;
        let mut fp = FieldPath::default();
        reader.refill();
        loop {
            node = match reader.read_bool() {
                true => node.right(),
                false => node.left(),
            };
            if let &HTree::Leaf { value, .. } = node {
                let op = OPERATIONS[value as usize].0;
                if let FieldOp::FieldPathEncodeFinish = op {
                    break;
                }
                op.execute(reader, &mut fp);
                self.paths_buf[i] = fp;
                i += 1;
                node = &self.tree;
                reader.refill();
            }
        }

        self.paths_buf[..i]
            .iter_mut()
            .for_each(|fp| state.set(fp, serializer.get_decoder_for_field_path(fp).decode(reader)));

        i
    }

    #[inline]
    pub(crate) fn field_paths(&self, count: usize) -> &[FieldPath] {
        &self.paths_buf[..count]
    }
}
