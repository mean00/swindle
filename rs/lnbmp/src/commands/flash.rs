
// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use alloc::vec;
use alloc::vec::Vec;
use crate::util::glog;
use crate::util::hex_to_u8s;

use crate::encoder::encoder;
use super::{CommandTree,exec_one};

use crate::bmp::{bmp_attach,bmp_flash_erase,bmp_flash_write, bmp_flash_complete};
use crate::commands::CallbackType;

const vflash_command_tree: [CommandTree;3] = 
[
    CommandTree{ command: "vFlashErase",args: 0, require_connected: true,  cb: CallbackType::text( _vFlashErase) },  // flash erase
    CommandTree{ command: "vFlashWrite",args: 0, require_connected: true,  cb: CallbackType::raw(  _vFlashWrite) },  // flash write
    CommandTree{ command: "vFlashDone",args: 0, require_connected: true,  cb: CallbackType::text( _vFlashDone)  },  // flash erase
];


const DISABLE_FLASH: bool = false;

//
//
//

pub fn _flashv(command : &str, args : &[u8]) -> bool
{
    return exec_one(&vflash_command_tree,command,args);
}

//
//vFlashErase:08000000,00005000
fn _vFlashErase(command : &str, args : &Vec<&str>) -> bool
{   
    if(DISABLE_FLASH) 
    {
        encoder::reply_ok();
        return true;
    }

    let xin = &args[0];
    match crate::util::take_adress_length(&xin[1..])
    {
        None =>   encoder::reply_e01(),
        Some( (adr,len) ) => 
            {   
                encoder::reply_bool(bmp_flash_erase(adr,len));
            },
    };
    return true;
}
//
//vFlashWrite:08000000,data
fn _vFlashWrite(command : &str, args : &[u8]) -> bool
{    
    if(DISABLE_FLASH) 
    {
        encoder::reply_ok();
        return true;
    }

    let block: &[u8] = args;
    let len = block.len();

    if len<9
    {
        crate::util::glog("flashWrite: invalid arg1");
        encoder::reply_e01();
        return true; 
    }

    // lookup for the ":"
    let mut prefix : usize =0;
    let mut index  : usize = 0;
    while prefix==0 && index <  16
    {
        if block[index]==b':'
        {
            prefix=index;
        }
        index+=1;
    }
    if prefix==0
    {
        crate::util::glog("flashWrite: invalid arg2");
        encoder::reply_e01();
        return true; 
    }

    let adr  =crate::util::u8s_to_u32( &block[..prefix] );
    let data: &[u8] = &block[(prefix+1)..];
    //crate::util::glog1("adr:",adr);
    //crate::util::glog1("len:",data.len());
    encoder::reply_bool( bmp_flash_write(adr, data) );
    true
}
//vFlashDone
fn _vFlashDone(command : &str, args : &Vec<&str>) -> bool
{    
    if(DISABLE_FLASH) 
    {
        encoder::reply_ok();
        return true;
    }

    encoder::reply_bool( bmp_flash_complete() );
    true
}


// EOF