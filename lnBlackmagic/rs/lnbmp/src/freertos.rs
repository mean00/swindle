
pub mod freertos_trait;
use freertos_trait::{freertos_info, freertos_handler};


pub fn spawn_freertos_handler() -> Option<&'static dyn freertos_handler>
{
    None
}

 //