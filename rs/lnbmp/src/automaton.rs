/*
    Small automaton to parse gdb mii string protocol

 */

#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]

use crate::util::ascii_to_hex;
use crate::util::log;
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
    Reset, 
}
/**
 * This returns the state of the internal automaton
 * Continue : Need more data
 * Read     : Decoded packet available
 * Reset    : A reset has been received
 */
#[derive(PartialEq)]
pub enum RESULT_AUTOMATON
{
    Continue,
    Ready,
    Reset,
}
const INPUT_BUFFER_SIZE : usize = 256;

const CHAR_RESET_04 : u8 = 0x04;
const CHAR_START : u8 = b'$';
const CHAR_END : u8 = b'#';
const CHAR_ESCAPE : u8 = b'}';
//
pub struct input_stream
{    
    automaton       : PARSER_AUTOMATOMON,
    input_buffer    : [u8;INPUT_BUFFER_SIZE],
    indx            : usize,
    checksum        : usize,
    checksum_received : [u8;2],
}
impl input_stream
{
    pub fn new() -> Self
    {
        input_stream
        {
            
            automaton       : PARSER_AUTOMATOMON::Idle,
            input_buffer    : [0;INPUT_BUFFER_SIZE],
            indx            : 0,
            checksum        : 0, // only 1 byte is actually used
            checksum_received : [0,0], // only 1 byte is actually used
        }
    }
    /**
     * 
     */
    pub fn reset(&mut self)
    {
        self.automaton       = PARSER_AUTOMATOMON::Idle;
    }
    /**
     * 
     */
    pub fn get_result(&mut self) -> &[u8]
    {
        self.automaton = PARSER_AUTOMATOMON::Idle;        
        &self.input_buffer[0..self.indx]
    }
    /**
     * 
     */
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
                                                CHAR_RESET_04   => PARSER_AUTOMATOMON::Reset,
                                                _               => PARSER_AUTOMATOMON::Init,
                                            }
                                            ,
                PARSER_AUTOMATOMON::Idle => 
                                            match c
                                            {
                                                CHAR_START /*'$'*/  => {self.indx = 0;self.checksum=0;PARSER_AUTOMATOMON::Body}, 
                                                CHAR_RESET_04       => PARSER_AUTOMATOMON::Reset,
                                                _                   => PARSER_AUTOMATOMON::Idle,
                                            }
                                            ,
                PARSER_AUTOMATOMON::Body => 
                                            match c
                                            {
                                                CHAR_END /*'#'*/        => PARSER_AUTOMATOMON::End1, 
                                                CHAR_ESCAPE /*'}'*/     => PARSER_AUTOMATOMON::Escape, 
                                                CHAR_RESET_04           => PARSER_AUTOMATOMON::Reset,
                                                _                       => {
                                                                        self.checksum+=c as usize;
                                                                        if c==b'\t'
                                                                        {
                                                                            self.input_buffer[self.indx]=b' ';
                                                                        }else
                                                                        {
                                                                            self.input_buffer[self.indx]=c;
                                                                        }
                                                                        self.indx+=1;
                                                                        PARSER_AUTOMATOMON::Body
                                                                    },
                                            }, 
                PARSER_AUTOMATOMON::Escape => 
                                        {
                                            self.checksum+= CHAR_ESCAPE as usize;
                                            self.checksum+=c as usize;
                                            self.input_buffer[self.indx]=c^20;
                                            self.indx+=1;
                                            PARSER_AUTOMATOMON::Body
                                        },
                PARSER_AUTOMATOMON::End1 => 
                                        {
                                            self.checksum_received[0]=c;
                                            PARSER_AUTOMATOMON::End2
                                        },
                PARSER_AUTOMATOMON::End2 => 
                                        {
                                            self.checksum_received[1]=c;
                                            // Verify checksum 
                                            let chk=ascii_to_hex(self.checksum_received[0],self.checksum_received[1]);
                                            if chk == (self.checksum & 0xff) as u8
                                            {
                                                PARSER_AUTOMATOMON::Done
                                            }else
                                            {
                                                log("Wrong checksum\n");
                                                PARSER_AUTOMATOMON::Idle
                                            }
                                        },
                PARSER_AUTOMATOMON::Reset =>   panic!("automatonReset"), 
                PARSER_AUTOMATOMON::Done => panic!("automatonDone"), //; PARSER_AUTOMATOMON::Done},
            };
            match self.automaton
            {
                PARSER_AUTOMATOMON::Done  =>   return (consumed, RESULT_AUTOMATON::Ready),
                PARSER_AUTOMATOMON::Reset =>   {self.automaton=PARSER_AUTOMATOMON::Idle;return (consumed, RESULT_AUTOMATON::Reset);},
                _                         => (),
            }
        }
        return (consumed, RESULT_AUTOMATON::Continue);
    }

}
//

// EOF
