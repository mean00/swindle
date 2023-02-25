#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]

mod util;
mod linear_ring;

extern crate alloc;
extern crate std;

use std::print;
//
//
#[derive(PartialEq)]
enum PARSER_AUTOMATOMON
{
    Init,
    Idle,
    Body,
    Escape,
    End1,
    End2,  
    Done,  
}
enum RESULT_AUTOMATON
{
    Continue,
    Ready,
    Error,
}

//
struct input_stream
{
    ring        : linear_ring::linear_ring<512>,
    automaton   : PARSER_AUTOMATOMON,
}
impl input_stream
{
    pub fn new() -> Self
    {
        input_stream
        {
            ring        : linear_ring::linear_ring::new(),
            automaton   : PARSER_AUTOMATOMON::Idle,
        }
    }
    pub fn parse( &mut self, data : &[u8]) -> (usize, RESULT_AUTOMATON)
    {
        let mut dex =0;
        let mut sz = data.len();
        let mut consumed = 0;
        while sz>0
        {
            let c : u8 = data[dex];
            consumed+=1;
            sz-=1;
            dex+=1;
            self.automaton  = match self.automaton 
            {
                PARSER_AUTOMATOMON::Init => 
                                                match c
                                                {
                                                    0x04 => PARSER_AUTOMATOMON::Idle,
                                                    _    => PARSER_AUTOMATOMON::Init,
                                                }
                                            ,
                PARSER_AUTOMATOMON::Idle => 
                                                match c
                                                {
                                                    0x24 /*'$'*/ => PARSER_AUTOMATOMON::Body, 
                                                    0x04 => PARSER_AUTOMATOMON::Idle,
                                                    _    => PARSER_AUTOMATOMON::Idle,
                                                }
                                            ,                  
                PARSER_AUTOMATOMON::Body => 
                                            match c
                                            {
                                                0x23 /*'#'*/  => PARSER_AUTOMATOMON::End1, 
                                                0x7d /*'}'*/  => PARSER_AUTOMATOMON::Escape, 
                                                0x04 => PARSER_AUTOMATOMON::Idle,
                                                _    => {
                                                            
                                                            PARSER_AUTOMATOMON::Body
                                                        },
                                            }
                                        ,                                                                                              
                PARSER_AUTOMATOMON::Escape => 
                                        {
                                            PARSER_AUTOMATOMON::Body
                                        },
                PARSER_AUTOMATOMON::End1 => 
                                        {
                                            PARSER_AUTOMATOMON::End2
                                        },                                        
                PARSER_AUTOMATOMON::End2 => 
                                        {
                                            PARSER_AUTOMATOMON::Done
                                        },  
                PARSER_AUTOMATOMON::Done => {panic!("automaton"); PARSER_AUTOMATOMON::Done},
            };
            if self.automaton ==  PARSER_AUTOMATOMON::Done
            {
                print!("!!!!!!!!!!!!!!\n");
                self.automaton = PARSER_AUTOMATOMON::Idle;
                return (consumed, RESULT_AUTOMATON::Ready);
            }

        }
        return (consumed, RESULT_AUTOMATON::Continue);
    }

}
//
#[no_mangle]

static mut autoauto : Option<input_stream> = None;

#[no_mangle]
extern "C" fn rngdbstub_init()
{
    unsafe {
    if autoauto.is_some()
    {
        panic!("notnull");
    }
    autoauto = Some(input_stream::new());
    }
}
#[no_mangle]
extern "C" fn rngdbstub_shutdown() 
{
    unsafe {
        if autoauto.is_none()
        {
            panic!("notsome");
        }
        autoauto = None;
    }
}

#[no_mangle]
extern "C" fn rngdbstub_run(l : usize, d : *const cty::c_uchar ) 
{
    unsafe {
    let data_as_slice : &[u8] = core::slice::from_raw_parts(d,l);    
    match autoauto
    {
        Some(ref mut x) => x.parse(data_as_slice),
        None => panic!("noauto"),
    };
    }
}

// EOF
