
/**
 */

pub struct popping_buffer<'a>
{
   data : &'a [u8],
}

impl <'a>popping_buffer <'a>
{
    pub fn new( inc : &'a [u8]) -> Self
    {
        popping_buffer
        {
            data : inc,
        }
    }
    pub fn pop( &mut self, nb : usize) -> &'a [u8]
    {
        let r= &self.data[0..nb];
        self.data=&self.data[nb..];
        r
    }
    pub fn leftover( &mut self) -> &'a [u8]
    {
        let r= &self.data[0..];
        self.data=&self.data[0..0];
        r
    }
}
// EOF

