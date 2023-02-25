/*
    Small automaton to parse gdb mii string protocol

 */

#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_imports)]

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
        }
    }
    pub fn get_result(&mut self) -> &[u8]
    {
        self.automaton = PARSER_AUTOMATOMON::Idle;        
        &self.input_buffer[0..self.indx]
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
                                                CHAR_RESET_04   => PARSER_AUTOMATOMON::Idle,
                                                _               => PARSER_AUTOMATOMON::Init,
                                            }
                                            ,
                PARSER_AUTOMATOMON::Idle => 
                                            match c
                                            {
                                                CHAR_START /*'$'*/  => {self.indx = 0;PARSER_AUTOMATOMON::Body}, 
                                                CHAR_RESET_04       => PARSER_AUTOMATOMON::Idle,
                                                _                   => PARSER_AUTOMATOMON::Idle,
                                            }
                                            ,
                PARSER_AUTOMATOMON::Body => 
                                            match c
                                            {
                                                CHAR_END /*'#'*/        => PARSER_AUTOMATOMON::End1, 
                                                CHAR_ESCAPE /*'}'*/     => PARSER_AUTOMATOMON::Escape, 
                                                CHAR_RESET_04 => PARSER_AUTOMATOMON::Idle,
                                                _                       => {
                                                                        self.input_buffer[self.indx]=c;
                                                                        self.indx+=1;
                                                                        PARSER_AUTOMATOMON::Body
                                                                    },
                                            }, 
                PARSER_AUTOMATOMON::Escape => 
                                        {
                                            self.input_buffer[self.indx]=c^20;
                                            self.indx+=1;
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
                PARSER_AUTOMATOMON::Done => panic!("automaton"), //; PARSER_AUTOMATOMON::Done},
            };
            if self.automaton ==  PARSER_AUTOMATOMON::Done
            {
                return (consumed, RESULT_AUTOMATON::Ready);
            }
        }
        return (consumed, RESULT_AUTOMATON::Continue);
    }

}
//

// EOF
