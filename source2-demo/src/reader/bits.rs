#[doc(hidden)]
pub trait BitsReader {
    fn refill(&mut self);
    fn read_bits(&mut self, amount: u32) -> u32;
    fn read_bits_unchecked(&mut self, amount: u32) -> u32;
    fn read_bytes(&mut self, amount: u32) -> Vec<u8>;
    fn read_bool(&mut self) -> bool;
    fn read_f32(&mut self) -> f32;
    fn read_var_u32(&mut self) -> u32;
    fn read_var_u64(&mut self) -> u64;
    fn read_var_i32(&mut self) -> i32;
    fn read_ubit_var(&mut self) -> u32;
    fn read_ubit_var_fp(&mut self) -> i32;
    fn read_ubit_var_fp_unchecked(&mut self) -> i32;
    fn read_normal(&mut self) -> f32;
    fn read_normal_vec3(&mut self) -> [f32; 3];
    fn read_u64_le(&mut self) -> u64;
    fn read_cstring(&mut self) -> String;
    fn read_coordinate(&mut self) -> f32;
    fn read_angle(&mut self, n: u32) -> f32;
    fn read_bits_as_bytes(&mut self, n: u32) -> Vec<u8>;
    fn remaining_bytes(&self) -> usize;
    fn seek(&mut self, offset: usize);
}
