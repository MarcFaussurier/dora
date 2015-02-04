use data_type::DataType;

#[derive(PartialEq,Eq,Debug)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub block: Box<Expr>
}

#[derive(PartialEq,Eq,Debug)]
pub struct Param {
    pub name: String,
    pub data_type: DataType
}

#[derive(PartialEq,Eq,Debug)]
pub enum UnOp {
    Plus, Neg, Not
}

#[derive(PartialEq,Eq,Debug)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, NEq, LThan, LEq, GThan, GEq
}

#[derive(PartialEq,Eq,Debug)]
pub enum Expr {
    ExprUn(UnOp,Box<Expr>),
    ExprBin(BinOp,Box<Expr>,Box<Expr>),
    ExprLitInt(i64),
    ExprLitStr(String),
    ExprIdent(String),
    ExprAssign(Box<Expr>,Box<Expr>),
    ExprCall(String,Vec<Box<Expr>>)
}
