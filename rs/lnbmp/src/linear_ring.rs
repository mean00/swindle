


use crate::util::unsafe_slice_alloc;

struct linear_ring
{
    data : &'static mut [u8],
    head : usize,
    tail : usize,
    size : usize,
}

impl linear_ring  
{
    pub fn new<T>(size : usize) -> Self
    {
        linear_ring
        {
            data : unsafe_slice_alloc::<u8>(size),
            head : 0,
            tail : 0,
            size ,
        }
    }
}