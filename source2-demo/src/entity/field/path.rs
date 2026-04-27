/// Internal path representation for a networked entity property.
///
/// A `FieldPath` identifies a concrete property within an entity serializer tree.
/// It is passed to property-change observers and can be converted back into a
/// human-readable property name via [`Class::field_name_for_path`](crate::Class::field_name_for_path).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct FieldPath {
    pub(crate) path: [u16; 7],
    pub(crate) last: usize,
}

impl Default for FieldPath {
    #[inline]
    fn default() -> Self {
        FieldPath {
            path: [u16::MAX, 0, 0, 0, 0, 0, 0],
            last: 0,
        }
    }
}

impl FieldPath {
    #[inline]
    pub(crate) fn push(&mut self, val: u16) {
        self.last += 1;
        self.path[self.last] = val;
    }

    #[inline]
    pub(crate) fn pop(&mut self, n: usize) {
        self.last -= n;
    }

    #[inline]
    pub(crate) fn inc(&mut self, n: usize, val: u16) {
        self.path[n] = self.path[n].wrapping_add(val)
    }

    #[inline]
    pub(crate) fn sub(&mut self, n: usize, val: u16) {
        self.path[n] = self.path[n].wrapping_sub(val)
    }

    #[inline]
    pub(crate) fn inc_curr(&mut self, val: u16) {
        self.path[self.last] = self.path[self.last].wrapping_add(val)
    }
}
