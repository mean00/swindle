/*
    Small automaton to parse gdb mii string protocol

 */


#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]

use crate::util::ascii_to_hex;
use crate::util::glog;
use crate::packet_symbols::{ CHAR_RESET_04,CHAR_START, CHAR_END,CHAR_ESCAPE};
//
//
#[derive(PartialEq)]
enum PARSER_AUTOMATON
{
    Init,
    Idle,
    Body,
    Escape,
    End1,
    End2,  
    Done, 
    Reset, 
    Error,
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
    Error,
}

//
pub struct gdb_stream <const INPUT_BUFFER_SIZE: usize>
{    
    automaton       : PARSER_AUTOMATON,
    input_buffer    : [u8;INPUT_BUFFER_SIZE],
    indx            : usize,
    checksum        : usize,
    checksum_received : [u8;2],
}
impl <const INPUT_BUFFER_SIZE: usize>gdb_stream <INPUT_BUFFER_SIZE>
{
    pub fn new() -> Self
    {
        gdb_stream
        {
            
            automaton       : PARSER_AUTOMATON::Idle,
            input_buffer    : [0;INPUT_BUFFER_SIZE],
            indx            : 0,
            checksum        : 0, // only 1 byte is actually used
            checksum_received : [0,0], // 2 Hex digits
        }
    }
    /**
     * 
     */
    pub fn reset(&mut self)
    {
        self.automaton       = PARSER_AUTOMATON::Idle;
    }
    /**
     * 
     */
    pub fn get_result(&mut self) -> &[u8]
    {
        self.automaton = PARSER_AUTOMATON::Idle;        
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

        // auto clear errors
        if  self.automaton  ==    PARSER_AUTOMATON::Error
        {
            self.automaton = PARSER_AUTOMATON::Idle;
        }     

        while sz>0
        {
            
            let c : u8 = data[dex];
            consumed+=1;
            sz-=1;
            dex+=1;
            self.automaton  = match self.automaton 
            {
                PARSER_AUTOMATON::Init => 
                                            match c
                                            {
                                                CHAR_RESET_04   => PARSER_AUTOMATON::Reset,
                                                _               => PARSER_AUTOMATON::Init,
                                            }
                                            ,
                PARSER_AUTOMATON::Idle => 
                                            match c
                                            {
                                                CHAR_START /*'$'*/  => {self.indx = 0;self.checksum=0;PARSER_AUTOMATON::Body}, 
                                                CHAR_RESET_04       => PARSER_AUTOMATON::Reset,
                                                _                   => PARSER_AUTOMATON::Idle,
                                            }
                                            ,
                PARSER_AUTOMATON::Body => 
                                            match c
                                            {
                                                CHAR_END /*'#'*/        => PARSER_AUTOMATON::End1, 
                                                CHAR_ESCAPE /*'}'*/     => PARSER_AUTOMATON::Escape, 
                                                CHAR_RESET_04           => PARSER_AUTOMATON::Reset,
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
                                                                        PARSER_AUTOMATON::Body
                                                                    },
                                            }, 
                PARSER_AUTOMATON::Escape => 
                                        {
                                            self.checksum+= CHAR_ESCAPE as usize;
                                            self.checksum+=c as usize;
                                            self.input_buffer[self.indx]=c^20;
                                            self.indx+=1;
                                            PARSER_AUTOMATON::Body
                                        },
                PARSER_AUTOMATON::End1 => 
                                        {
                                            self.checksum_received[0]=c;
                                            PARSER_AUTOMATON::End2
                                        },
                PARSER_AUTOMATON::End2 => 
                                        {
                                            self.checksum_received[1]=c;
                                            // Verify checksum 
                                            let chk=ascii_to_hex(self.checksum_received[0],self.checksum_received[1]);
                                            if chk == (self.checksum & 0xff) as u8
                                            {
                                                PARSER_AUTOMATON::Done
                                            }else
                                            {
                                                glog("Wrong checksum\n");
                                                PARSER_AUTOMATON::Error
                                            }
                                        },
                PARSER_AUTOMATON::Reset =>   panic!("automatonReset"), 
                PARSER_AUTOMATON::Done => panic!("automatonDone"), //; PARSER_AUTOMATON::Done},
                PARSER_AUTOMATON::Error => panic!("automatonError"), //; PARSER_AUTOMATON::Done},
            };
            // should exit the loop even if we have data left ?
            match self.automaton
            {
                PARSER_AUTOMATON::Error => {self.automaton=PARSER_AUTOMATON::Idle; return (consumed, RESULT_AUTOMATON::Error);},
                PARSER_AUTOMATON::Done  =>   return (consumed, RESULT_AUTOMATON::Ready),
                PARSER_AUTOMATON::Reset =>   {self.automaton=PARSER_AUTOMATON::Idle;return (consumed, RESULT_AUTOMATON::Reset);},
                _                         => (),
            }
        }
        return (consumed, RESULT_AUTOMATON::Continue);
    }

}
//

// EOF