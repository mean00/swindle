
use crate::util::glog;
use alloc::vec::Vec;



pub fn exec(tokns : Vec<&str>)
{
    for i in 0..tokns.len()
    {
        glog(tokns[i]);
    }
    
}
