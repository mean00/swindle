
// https://sourceware.org/gdb/onlinedocs/gdb/Packets.html
// https://sourceware.org/gdb/onlinedocs/gdb/General-Query-Packets.html

use crate::encoder::encoder;
use super::{CommandTree,exec_one};

use crate::bmp::{bmp_attach,bmp_flash_erase,bmp_flash_write, bmp_flash_complete};
use crate::commands::CallbackType;

crate::setup_log!(false);
use crate::{bmplog,bmpwarning};

const vflash_command_tree: [CommandTree;3] = 
[
    CommandTree{ command: "vFlashErase",args: 0, require_connected: true,  cb: CallbackType::text( _vFlashErase) },  // flash erase
    CommandTree{ command: "vFlashWrite",args: 0, require_connected: true,  cb: CallbackType::raw(  _vFlashWrite) },  // flash write
    CommandTree{ command: "vFlashDone", args: 0, require_connected: true,  cb: CallbackType::text( _vFlashDone)  },  // flash erase
];


const DISABLE_FLASH: bool = false;


//
//
//

pub fn _flashv(command : &str, args : &[u8]) -> bool
{
    exec_one(&vflash_command_tree,command,args)
}

//
//vFlashErase:08000000,00005000
fn _vFlashErase(_command : &str, args :&[&str]) -> bool
{   
    if DISABLE_FLASH
    {
        encoder::reply_ok();
        return true;
    }

    let xin = &args[0];
    bmplog!("Input : {}\n",xin );
    match crate::parsing_util::take_adress_length(xin)
    {
        None =>   encoder::reply_e01(),
        Some( (adr,len) ) => 
            {   
                bmplog!("Erase : Adr 0x{:x} len {}\n",adr,len);
                encoder::reply_bool(bmp_flash_erase(adr,len));
            },
    };
    true
}
//
//vFlashWrite:08000000,data
fn _vFlashWrite(_command : &str, args : &[u8]) -> bool
{    
    if DISABLE_FLASH
    {
        encoder::reply_ok();
        return true;
    }

    let block: &[u8] = args;
    let len = block.len();

    if len<9
    {
        bmplog!("flashWrite: invalid arg1 \n");
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
        bmplog!("flashWrite: invalid arg2");
        encoder::reply_e01();
        return true; 
    }

    let adr  =crate::parsing_util::u8s_string_to_u32( &block[..prefix] );
    let data: &[u8] = &block[(prefix+1)..];
    bmplog!("adr:0x{:x} en:{}\n",adr,data.len());
    bmplog!("write : Adr 0x{:x} len {}\n",adr ,len );

    encoder::reply_bool( bmp_flash_write(adr, data) );
    true
}
//vFlashDone
fn _vFlashDone(_command : &str, _args :&[&str]) -> bool
{    
    if DISABLE_FLASH
    {
        encoder::reply_ok();
        return true;
    }

    encoder::reply_bool( bmp_flash_complete() );
    true
}


// EOF
