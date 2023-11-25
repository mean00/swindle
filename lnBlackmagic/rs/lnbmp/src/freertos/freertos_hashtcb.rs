/**
 *  Hold a hashMap so that a given tcb address always gives the same task id
 *   whatever freertos compilation options are.
 * 
 */

use hashbrown::HashMap;
crate::setup_log!(false);
use crate::{bmplog, bmpwarning};

pub struct hashed_tcb {
    hash : HashMap<u32,u32> ,
    index: u32,
}
/**
 * 
 */
impl hashed_tcb {
    /**
     * 
     */
    pub fn new()->Self
    {
        hashed_tcb {
            hash : HashMap::new(),
            index : 1,
        }
    }
    /**
     * 
     */
    pub fn clear (&mut self) {
            self.hash.drain();
            self.index=1;

    }
    /**
     * 
     */
    pub fn get(&mut self, tcb : u32) -> u32 {
        let id: u32 =  match self.hash.get(&tcb)
        {
            Some(x) => *x,
            None =>  { self.hash.insert(tcb,self.index); self.index +=1;self.index-1},
        };
        id        
    }
}

//--

static mut tcb_hashmap: Option<hashed_tcb>=  None;
/**
 * 
 */
pub fn get_hashtcb() ->  &'static mut hashed_tcb
{
    unsafe {
        if tcb_hashmap.is_none() {
            tcb_hashmap = Some(  hashed_tcb::new());
        }
        match &mut tcb_hashmap {
            Some(ref mut x) =>  x ,
            None =>  panic!("hashap"),
        }
    }
}