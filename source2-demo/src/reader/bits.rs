pub trait BitsReader {
    /// Refills the internal lookahead buffer.
    /// Must be called before `read_bits_unchecked` to ensure sufficient bits
    /// are available in the lookahead buffer.
    fn refill(&mut self);

    fn read_bits(&mut self, amount: u32) -> u32;

    /// Reads bits without refilling the buffer (unchecked).
    ///
    /// The caller must ensure `refill()` was called and sufficient bits remain.
    ///
    /// # Arguments
    /// * `amount` - Number of bits to read (must be ≤ 32)
    fn read_bits_unchecked(&mut self, amount: u32) -> u32;

    fn read_bytes(&mut self, amount: u32) -> Vec<u8>;

    fn read_bool(&mut self) -> bool;

    fn read_f32(&mut self) -> f32;

    fn read_var_u32(&mut self) -> u32;

    fn read_var_u64(&mut self) -> u64;

    fn read_var_i32(&mut self) -> i32;

    fn read_ubit_var(&mut self) -> u32;

    fn read_ubit_var_fp(&mut self) -> i32;

    /// Reads a variable-length unsigned integer (field path encoding) without refilling.
    ///
    /// The caller must ensure sufficient bits are available in the lookahead buffer.
    fn read_ubit_var_fp_unchecked(&mut self) -> i32;

    fn read_normal(&mut self) -> f32;

    /// Reads a compressed 3D normal vector.
    ///
    /// The vector components are encoded with selective precision,
    /// and the third component is derived from the unit length constraint.
    fn read_normal_vec3(&mut self) -> [f32; 3];

    /// Reads a little-endian 64-bit unsigned integer.
    fn read_u64_le(&mut self) -> u64;

    /// Reads a null-terminated C-style string.
    fn read_cstring(&mut self) -> String;

    fn read_coordinate(&mut self) -> f32;

    fn read_angle(&mut self, n: u32) -> f32;

    fn read_bits_as_bytes(&mut self, n: u32) -> Vec<u8>;

    /// Returns the number of bytes remaining in the buffer.
    fn remaining_bytes(&self) -> usize;

    /// Seeks to a specific byte offset from the start.
    ///
    /// # Arguments
    /// * `offset` - Byte offset from the start of the data
    fn seek(&mut self, offset: usize);
}

