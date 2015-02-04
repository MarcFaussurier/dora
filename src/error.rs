use std::fmt;

use lexer::position::Position;

#[derive(PartialEq,Eq,Debug)]
pub enum ErrorCode {
    UnclosedComment, UnknownChar, UnclosedString, NumberOverflow, UnknownFactor,
    UnexpectedToken, NoTopLevelElement, ExpectedType, ExpectedIdentifier
}

pub struct ParseError {
    pub filename: String,
    pub position: Position,
    pub message: String,
    pub code: ErrorCode
}

impl ParseError {
    pub fn println(&self) {
        println!("{}", self);
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "error in {} at line {}: {}", self.filename, self.position, self.message)
    }
}

impl fmt::Debug for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "error in {} at line {}: {}", self.filename, self.position, self.message)
    }
}
