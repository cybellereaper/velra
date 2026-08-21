#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub const fn merge(self, other: Self) -> Self {
        Self::new(self.start, other.end)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Use {
        path: Vec<String>,
    },
    Var {
        name: String,
        type_name: Option<String>,
        value: Expr,
    },
    Assign {
        target: Expr,
        value: Expr,
    },
    Return(Option<Expr>),
    While {
        condition: Expr,
        body: Block,
    },
    Break,
    Continue,
    For {
        name: String,
        iterable: Expr,
        body: Block,
    },
    Function(FunctionDecl),
    Data(DataDecl),
    Expr(Expr),
    Pub(Box<Stmt>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub body: FunctionBody,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FunctionBody {
    Expr(Expr),
    Block(Block),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub type_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub computed: Vec<(String, Expr)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Ident(String),
    List(Vec<Expr>),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Member {
        object: Box<Expr>,
        name: String,
        safe: bool,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    Propagate {
        expr: Box<Expr>,
    },
    Lambda {
        params: Vec<Param>,
        body: Box<Expr>,
    },
    If {
        condition: Box<Expr>,
        then_branch: Block,
        else_branch: Option<ElseBranch>,
    },
    When {
        subject: Option<Box<Expr>>,
        cases: Vec<WhenCase>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ElseBranch {
    Block(Block),
    If(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhenCase {
    pub patterns: Vec<Expr>,
    pub guard: Option<Expr>,
    pub body: WhenBody,
    pub is_else: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WhenBody {
    Expr(Expr),
    Block(Block),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Elvis,
    Or,
    And,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}
