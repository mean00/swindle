/*


*/
use crate::util::unsafe_slice_alloc;

pub struct linear_ring<const ring_size: usize> {
    data: &'static mut [u8],
    head: usize,
    tail: usize,
}
//
//
impl<const ring_size: usize> linear_ring<ring_size> {
    //
    pub fn new() -> Self {
        linear_ring {
            data: unsafe_slice_alloc::<u8>(ring_size),
            head: 0,
            tail: 0,
        }
    }
    fn modd(l: usize) -> usize {
        if l >= ring_size {
            return l - ring_size;
        }
        l
    }
    //
    pub fn get_write_slice(&mut self) -> &mut [u8] {
        if self.tail >= self.head {
            return &mut self.data[self.tail..];
        }
        return &mut self.data[self.head..self.tail];
    }
    pub fn set_write_count(&mut self, sz: usize) {
        self.tail = Self::modd(self.tail + sz);
    }
    //
    pub fn get_read_slice(&mut self) -> &[u8] {
        if self.tail >= self.head {
            return &mut self.data[self.head..self.tail];
        }
        return &mut self.data[self.head..];
    }
    //
    pub fn read_consume(&mut self, sz: usize) {
        self.tail = Self::modd(self.head + sz);
    }
    //
    pub fn flush(&mut self) {
        self.head = 0;
        self.tail = 0;
    }
}
