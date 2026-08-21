use crate::ast::{
    BinaryOp, Block, DataDecl, ElseBranch, Expr, FunctionBody, FunctionDecl, Program, Stmt,
    UnaryOp, WhenBody,
};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::rc::Rc;

pub type RuntimeResult<T> = Result<T, RuntimeError>;

type EnvRef = Rc<RefCell<Environment>>;
type NativeFn = fn(Vec<Value>) -> RuntimeResult<Value>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    pub message: String,
}

impl RuntimeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RuntimeError {}

#[derive(Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Rc<RefCell<Vec<Value>>>),
    Record(Rc<Record>),
    Function(Rc<UserFunction>),
    Native(NativeFunction),
    Constructor(Rc<DataType>),
}

impl Value {
    pub fn type_name(&self) -> &str {
        match self {
            Self::Null => "Null",
            Self::Bool(_) => "Bool",
            Self::Int(_) => "Int",
            Self::Float(_) => "Float",
            Self::String(_) => "String",
            Self::List(_) => "List",
            Self::Record(record) => &record.name,
            Self::Function(_) | Self::Native(_) => "Function",
            Self::Constructor(data) => &data.name,
        }
    }

    fn expect_bool(&self, context: &str) -> RuntimeResult<bool> {
        match self {
            Self::Bool(value) => Ok(*value),
            _ => Err(RuntimeError::new(format!(
                "{context} requires Bool, got {}",
                self.type_name()
            ))),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("null"),
            Self::Bool(value) => write!(f, "{value}"),
            Self::Int(value) => write!(f, "{value}"),
            Self::Float(value) => write!(f, "{value}"),
            Self::String(value) => f.write_str(value),
            Self::List(values) => {
                f.write_str("[")?;
                for (index, value) in values.borrow().iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{value}")?;
                }
                f.write_str("]")
            }
            Self::Record(record) => {
                write!(f, "{}(", record.name)?;
                for (index, (name, value)) in record.fields.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{name}: {value}")?;
                }
                f.write_str(")")
            }
            Self::Function(function) => write!(f, "<function {}>", function.name),
            Self::Native(function) => write!(f, "<native {}>", function.name),
            Self::Constructor(data) => write!(f, "<data {}>", data.name),
        }
    }
}

#[derive(Clone)]
pub struct NativeFunction {
    name: &'static str,
    call: NativeFn,
}

#[derive(Clone)]
pub struct UserFunction {
    name: String,
    params: Vec<crate::ast::Param>,
    body: FunctionBody,
    closure: EnvRef,
}

#[derive(Clone)]
pub struct DataType {
    name: String,
    params: Vec<crate::ast::Param>,
    computed: Vec<(String, Expr)>,
    closure: EnvRef,
}

#[derive(Clone)]
pub struct Record {
    name: String,
    fields: BTreeMap<String, Value>,
}

#[derive(Clone)]
struct Binding {
    value: Value,
    mutable: bool,
}

#[derive(Clone, Default)]
struct Environment {
    values: HashMap<String, Binding>,
    parent: Option<EnvRef>,
}

impl Environment {
    fn child(parent: &EnvRef) -> EnvRef {
        Rc::new(RefCell::new(Self {
            values: HashMap::new(),
            parent: Some(Rc::clone(parent)),
        }))
    }
}

#[derive(Debug)]
enum Signal {
    Runtime(RuntimeError),
    Return(Value),
}

type EvalResult<T> = Result<T, Signal>;

impl From<RuntimeError> for Signal {
    fn from(value: RuntimeError) -> Self {
        Self::Runtime(value)
    }
}

pub struct Interpreter {
    globals: EnvRef,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    pub fn new() -> Self {
        let globals = Rc::new(RefCell::new(Environment::default()));
        install_builtins(&globals);
        Self { globals }
    }

    pub fn eval_program(&mut self, program: &Program) -> RuntimeResult<Value> {
        match self.eval_statements(&program.statements, Rc::clone(&self.globals)) {
            Ok(value) => Ok(value),
            Err(Signal::Runtime(error)) => Err(error),
            Err(Signal::Return(_)) => Err(RuntimeError::new("'return' used outside a function")),
        }
    }

    fn eval_statements(&mut self, statements: &[Stmt], env: EnvRef) -> EvalResult<Value> {
        let mut last = Value::Null;
        for statement in statements {
            last = self.eval_statement(statement, Rc::clone(&env))?;
        }
        Ok(last)
    }

    fn eval_statement(&mut self, statement: &Stmt, env: EnvRef) -> EvalResult<Value> {
        match statement {
            Stmt::Use { path } => Err(RuntimeError::new(format!(
                "module loading is not implemented yet: {}",
                path.join(".")
            ))
            .into()),
            Stmt::Var {
                name,
                type_name,
                value,
            } => {
                let value = self.eval_expr(value, Rc::clone(&env))?;
                if let Some(type_name) = type_name {
                    require_type(name, type_name, &value)?;
                }
                define(&env, name, value, true)?;
                Ok(Value::Null)
            }
            Stmt::Assign { target, value } => {
                let value = self.eval_expr(value, Rc::clone(&env))?;
                self.assign_target(target, value, env)?;
                Ok(Value::Null)
            }
            Stmt::Return(expr) => {
                let value = match expr {
                    Some(expr) => self.eval_expr(expr, env)?,
                    None => Value::Null,
                };
                Err(Signal::Return(value))
            }
            Stmt::For {
                name,
                iterable,
                body,
            } => {
                let iterable = self.eval_expr(iterable, Rc::clone(&env))?;
                let values = iterable_values(iterable)?;
                let mut last = Value::Null;
                for value in values {
                    let iteration_env = Environment::child(&env);
                    define(&iteration_env, name, value, false)?;
                    last = self.eval_block(body, iteration_env)?;
                }
                Ok(last)
            }
            Stmt::Function(decl) => {
                self.define_function(decl, env)?;
                Ok(Value::Null)
            }
            Stmt::Data(decl) => {
                self.define_data(decl, env)?;
                Ok(Value::Null)
            }
            Stmt::Expr(expr) => self.eval_expr(expr, env),
            Stmt::Pub(inner) => self.eval_statement(inner, env),
        }
    }

    fn define_function(&self, decl: &FunctionDecl, env: EnvRef) -> EvalResult<()> {
        let function = UserFunction {
            name: decl.name.clone(),
            params: decl.params.clone(),
            body: decl.body.clone(),
            closure: Rc::clone(&env),
        };
        define(&env, &decl.name, Value::Function(Rc::new(function)), false)?;
        Ok(())
    }

    fn define_data(&self, decl: &DataDecl, env: EnvRef) -> EvalResult<()> {
        let data = DataType {
            name: decl.name.clone(),
            params: decl.params.clone(),
            computed: decl.computed.clone(),
            closure: Rc::clone(&env),
        };
        define(&env, &decl.name, Value::Constructor(Rc::new(data)), false)?;
        Ok(())
    }

    fn assign_target(&mut self, target: &Expr, value: Value, env: EnvRef) -> EvalResult<()> {
        match target {
            Expr::Ident(name) => {
                if binding_exists(&env, name) {
                    assign(&env, name, value)?;
                } else {
                    define(&env, name, value, false)?;
                }
                Ok(())
            }
            Expr::Index { object, index } => {
                let object = self.eval_expr(object, Rc::clone(&env))?;
                let index = self.eval_expr(index, env)?;
                match (object, index) {
                    (Value::List(values), Value::Int(index)) => {
                        let index = normalize_index(index, values.borrow().len())?;
                        values.borrow_mut()[index] = value;
                        Ok(())
                    }
                    (object, index) => Err(RuntimeError::new(format!(
                        "cannot assign through {} index {}",
                        object.type_name(),
                        index.type_name()
                    ))
                    .into()),
                }
            }
            _ => Err(RuntimeError::new("invalid assignment target").into()),
        }
    }

    fn eval_block(&mut self, block: &Block, parent: EnvRef) -> EvalResult<Value> {
        let env = Environment::child(&parent);
        self.eval_statements(&block.statements, env)
    }

    fn eval_expr(&mut self, expr: &Expr, env: EnvRef) -> EvalResult<Value> {
        match expr {
            Expr::Null => Ok(Value::Null),
            Expr::Bool(value) => Ok(Value::Bool(*value)),
            Expr::Int(value) => Ok(Value::Int(*value)),
            Expr::Float(value) => Ok(Value::Float(*value)),
            Expr::String(value) => Ok(Value::String(value.clone())),
            Expr::Ident(name) => get(&env, name).ok_or_else(|| {
                Signal::Runtime(RuntimeError::new(format!("unknown name '{name}'")))
            }),
            Expr::List(items) => {
                let mut values = Vec::with_capacity(items.len());
                for item in items {
                    values.push(self.eval_expr(item, Rc::clone(&env))?);
                }
                Ok(Value::List(Rc::new(RefCell::new(values))))
            }
            Expr::Unary { op, expr } => {
                let value = self.eval_expr(expr, env)?;
                eval_unary(*op, value).map_err(Into::into)
            }
            Expr::Binary { left, op, right } => self.eval_binary(left, *op, right, env),
            Expr::Call { callee, args } => {
                let callee = self.eval_expr(callee, Rc::clone(&env))?;
                let mut values = Vec::with_capacity(args.len());
                for arg in args {
                    values.push(self.eval_expr(arg, Rc::clone(&env))?);
                }
                self.call(callee, values)
            }
            Expr::Member { object, name, safe } => {
                let object = self.eval_expr(object, env)?;
                if *safe && matches!(object, Value::Null) {
                    return Ok(Value::Null);
                }
                member(object, name).map_err(Into::into)
            }
            Expr::Index { object, index } => {
                let object = self.eval_expr(object, Rc::clone(&env))?;
                let index = self.eval_expr(index, env)?;
                index_value(object, index).map_err(Into::into)
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = self.eval_expr(condition, Rc::clone(&env))?;
                if condition.expect_bool("if condition")? {
                    self.eval_block(then_branch, env)
                } else {
                    match else_branch {
                        Some(ElseBranch::Block(block)) => self.eval_block(block, env),
                        Some(ElseBranch::If(expr)) => self.eval_expr(expr, env),
                        None => Ok(Value::Null),
                    }
                }
            }
            Expr::When { subject, cases } => {
                let subject = match subject {
                    Some(subject) => Some(self.eval_expr(subject, Rc::clone(&env))?),
                    None => None,
                };

                for case in cases {
                    let matched = if case.is_else {
                        true
                    } else if let Some(subject) = &subject {
                        let mut matched = false;
                        for pattern in &case.patterns {
                            let pattern = self.eval_expr(pattern, Rc::clone(&env))?;
                            if values_equal(subject, &pattern) {
                                matched = true;
                                break;
                            }
                        }
                        matched
                    } else {
                        let mut matched = false;
                        for pattern in &case.patterns {
                            let pattern = self.eval_expr(pattern, Rc::clone(&env))?;
                            if pattern.expect_bool("when condition")? {
                                matched = true;
                                break;
                            }
                        }
                        matched
                    };

                    if !matched {
                        continue;
                    }
                    if let Some(guard) = &case.guard {
                        let guard = self.eval_expr(guard, Rc::clone(&env))?;
                        if !guard.expect_bool("when guard")? {
                            continue;
                        }
                    }
                    return match &case.body {
                        WhenBody::Expr(expr) => self.eval_expr(expr, env),
                        WhenBody::Block(block) => self.eval_block(block, env),
                    };
                }
                Ok(Value::Null)
            }
        }
    }

    fn eval_binary(
        &mut self,
        left: &Expr,
        op: BinaryOp,
        right: &Expr,
        env: EnvRef,
    ) -> EvalResult<Value> {
        let left = self.eval_expr(left, Rc::clone(&env))?;
        match op {
            BinaryOp::Elvis => {
                if matches!(left, Value::Null) {
                    self.eval_expr(right, env)
                } else {
                    Ok(left)
                }
            }
            BinaryOp::And => {
                if !left.expect_bool("left operand of '&&'")? {
                    Ok(Value::Bool(false))
                } else {
                    let right = self.eval_expr(right, env)?;
                    Ok(Value::Bool(right.expect_bool("right operand of '&&'")?))
                }
            }
            BinaryOp::Or => {
                if left.expect_bool("left operand of '||'")? {
                    Ok(Value::Bool(true))
                } else {
                    let right = self.eval_expr(right, env)?;
                    Ok(Value::Bool(right.expect_bool("right operand of '||'")?))
                }
            }
            _ => {
                let right = self.eval_expr(right, env)?;
                eval_binary_values(left, op, right).map_err(Into::into)
            }
        }
    }

    fn call(&mut self, callee: Value, args: Vec<Value>) -> EvalResult<Value> {
        match callee {
            Value::Native(function) => (function.call)(args).map_err(Into::into),
            Value::Function(function) => {
                require_arity(&function.name, function.params.len(), args.len())?;
                let call_env = Environment::child(&function.closure);
                for (param, value) in function.params.iter().zip(args) {
                    if let Some(type_name) = &param.type_name {
                        require_type(&param.name, type_name, &value)?;
                    }
                    define(&call_env, &param.name, value, false)?;
                }
                let result = match &function.body {
                    FunctionBody::Expr(expr) => self.eval_expr(expr, call_env),
                    FunctionBody::Block(block) => self.eval_block(block, call_env),
                };
                match result {
                    Err(Signal::Return(value)) => Ok(value),
                    other => other,
                }
            }
            Value::Constructor(data) => {
                require_arity(&data.name, data.params.len(), args.len())?;
                let call_env = Environment::child(&data.closure);
                let mut fields = BTreeMap::new();
                for (param, value) in data.params.iter().zip(args) {
                    if let Some(type_name) = &param.type_name {
                        require_type(&param.name, type_name, &value)?;
                    }
                    define(&call_env, &param.name, value.clone(), false)?;
                    fields.insert(param.name.clone(), value);
                }
                for (name, expr) in &data.computed {
                    let value = self.eval_expr(expr, Rc::clone(&call_env))?;
                    fields.insert(name.clone(), value.clone());
                    define(&call_env, name, value, false)?;
                }
                Ok(Value::Record(Rc::new(Record {
                    name: data.name.clone(),
                    fields,
                })))
            }
            value => Err(RuntimeError::new(format!("{} is not callable", value.type_name())).into()),
        }
    }
}

fn define(env: &EnvRef, name: &str, value: Value, mutable: bool) -> RuntimeResult<()> {
    let mut env = env.borrow_mut();
    if env.values.contains_key(name) {
        return Err(RuntimeError::new(format!(
            "name '{name}' is already defined in this scope"
        )));
    }
    env.values
        .insert(name.to_owned(), Binding { value, mutable });
    Ok(())
}

fn get(env: &EnvRef, name: &str) -> Option<Value> {
    let (value, parent) = {
        let env = env.borrow();
        (
            env.values.get(name).map(|binding| binding.value.clone()),
            env.parent.clone(),
        )
    };
    value.or_else(|| parent.and_then(|parent| get(&parent, name)))
}

fn binding_exists(env: &EnvRef, name: &str) -> bool {
    let (exists, parent) = {
        let env = env.borrow();
        (env.values.contains_key(name), env.parent.clone())
    };
    exists || parent.is_some_and(|parent| binding_exists(&parent, name))
}

fn assign(env: &EnvRef, name: &str, value: Value) -> RuntimeResult<()> {
    let parent = {
        let mut env = env.borrow_mut();
        if let Some(binding) = env.values.get_mut(name) {
            if !binding.mutable {
                return Err(RuntimeError::new(format!(
                    "cannot assign to immutable binding '{name}'"
                )));
            }
            binding.value = value;
            return Ok(());
        }
        env.parent.clone()
    };

    match parent {
        Some(parent) => assign(&parent, name, value),
        None => Err(RuntimeError::new(format!("unknown name '{name}'"))),
    }
}

fn install_builtins(env: &EnvRef) {
    for (name, call) in [
        ("print", builtin_print as NativeFn),
        ("println", builtin_println as NativeFn),
        ("len", builtin_len as NativeFn),
        ("type", builtin_type as NativeFn),
        ("assert", builtin_assert as NativeFn),
        ("range", builtin_range as NativeFn),
        ("push", builtin_push as NativeFn),
        ("read", builtin_read as NativeFn),
        ("write", builtin_write as NativeFn),
    ] {
        env.borrow_mut().values.insert(
            name.into(),
            Binding {
                value: Value::Native(NativeFunction { name, call }),
                mutable: false,
            },
        );
    }
}

fn builtin_print(args: Vec<Value>) -> RuntimeResult<Value> {
    for value in args {
        print!("{value}");
    }
    Ok(Value::Null)
}

fn builtin_println(args: Vec<Value>) -> RuntimeResult<Value> {
    if args.is_empty() {
        println!();
    } else {
        for (index, value) in args.iter().enumerate() {
            if index > 0 {
                print!(" ");
            }
            print!("{value}");
        }
        println!();
    }
    Ok(Value::Null)
}

fn builtin_len(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("len", 1, args.len())?;
    let len = match &args[0] {
        Value::String(value) => value.chars().count(),
        Value::List(values) => values.borrow().len(),
        Value::Record(record) => record.fields.len(),
        value => {
            return Err(RuntimeError::new(format!(
                "len() does not accept {}",
                value.type_name()
            )))
        }
    };
    i64::try_from(len)
        .map(Value::Int)
        .map_err(|_| RuntimeError::new("length exceeds Int range"))
}

fn builtin_type(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("type", 1, args.len())?;
    Ok(Value::String(args[0].type_name().to_owned()))
}

fn builtin_assert(args: Vec<Value>) -> RuntimeResult<Value> {
    if !(1..=2).contains(&args.len()) {
        return Err(RuntimeError::new(format!(
            "assert() expects 1 or 2 arguments, got {}",
            args.len()
        )));
    }
    if args[0].expect_bool("assert()")? {
        return Ok(Value::Null);
    }
    let message = args
        .get(1)
        .map(ToString::to_string)
        .unwrap_or_else(|| "assertion failed".into());
    Err(RuntimeError::new(message))
}

fn builtin_range(args: Vec<Value>) -> RuntimeResult<Value> {
    if !(1..=2).contains(&args.len()) {
        return Err(RuntimeError::new(format!(
            "range() expects 1 or 2 arguments, got {}",
            args.len()
        )));
    }
    let (start, end) = match args.as_slice() {
        [Value::Int(end)] => (0, *end),
        [Value::Int(start), Value::Int(end)] => (*start, *end),
        _ => return Err(RuntimeError::new("range() arguments must be Int")),
    };
    let values = (start..end).map(Value::Int).collect();
    Ok(Value::List(Rc::new(RefCell::new(values))))
}

fn builtin_push(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("push", 2, args.len())?;
    let Value::List(values) = &args[0] else {
        return Err(RuntimeError::new("push() first argument must be List"));
    };
    values.borrow_mut().push(args[1].clone());
    Ok(Value::Null)
}

fn builtin_read(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("read", 1, args.len())?;
    let Value::String(path) = &args[0] else {
        return Err(RuntimeError::new("read() path must be String"));
    };
    std::fs::read_to_string(path)
        .map(Value::String)
        .map_err(|error| RuntimeError::new(format!("failed to read '{path}': {error}")))
}

fn builtin_write(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("write", 2, args.len())?;
    let (Value::String(path), Value::String(content)) = (&args[0], &args[1]) else {
        return Err(RuntimeError::new("write() expects (String, String)"));
    };
    std::fs::write(path, content)
        .map(|()| Value::Null)
        .map_err(|error| RuntimeError::new(format!("failed to write '{path}': {error}")))
}

fn require_type(name: &str, expected: &str, value: &Value) -> RuntimeResult<()> {
    let (base, nullable) = expected
        .strip_suffix('?')
        .map_or((expected, false), |base| (base, true));
    if nullable && matches!(value, Value::Null) {
        return Ok(());
    }
    let matches = match base {
        "Any" => true,
        "Number" => matches!(value, Value::Int(_) | Value::Float(_)),
        _ => value.type_name() == base,
    };
    if matches {
        Ok(())
    } else {
        Err(RuntimeError::new(format!(
            "'{name}' expects {expected}, got {}",
            value.type_name()
        )))
    }
}

fn require_arity(name: &str, expected: usize, actual: usize) -> RuntimeResult<()> {
    if expected == actual {
        Ok(())
    } else {
        Err(RuntimeError::new(format!(
            "{name} expects {expected} argument{}, got {actual}",
            if expected == 1 { "" } else { "s" }
        )))
    }
}

fn iterable_values(value: Value) -> RuntimeResult<Vec<Value>> {
    match value {
        Value::List(values) => Ok(values.borrow().clone()),
        Value::String(value) => Ok(value
            .chars()
            .map(|ch| Value::String(ch.to_string()))
            .collect()),
        Value::Int(end) if end >= 0 => Ok((0..end).map(Value::Int).collect()),
        value => Err(RuntimeError::new(format!(
            "{} is not iterable",
            value.type_name()
        ))),
    }
}

fn eval_unary(op: UnaryOp, value: Value) -> RuntimeResult<Value> {
    match (op, value) {
        (UnaryOp::Negate, Value::Int(value)) => value
            .checked_neg()
            .map(Value::Int)
            .ok_or_else(|| RuntimeError::new("integer overflow")),
        (UnaryOp::Negate, Value::Float(value)) => Ok(Value::Float(-value)),
        (UnaryOp::Not, Value::Bool(value)) => Ok(Value::Bool(!value)),
        (UnaryOp::Negate, value) => Err(RuntimeError::new(format!(
            "unary '-' does not accept {}",
            value.type_name()
        ))),
        (UnaryOp::Not, value) => Err(RuntimeError::new(format!(
            "unary '!' does not accept {}",
            value.type_name()
        ))),
    }
}

fn eval_binary_values(left: Value, op: BinaryOp, right: Value) -> RuntimeResult<Value> {
    use BinaryOp::*;

    match op {
        Equal => return Ok(Value::Bool(values_equal(&left, &right))),
        NotEqual => return Ok(Value::Bool(!values_equal(&left, &right))),
        Elvis | Or | And => unreachable!("short-circuit operators are handled earlier"),
        _ => {}
    }

    match (&left, &right) {
        (Value::Int(a), Value::Int(b)) => eval_ints(*a, op, *b),
        (Value::Float(a), Value::Float(b)) => eval_floats(*a, op, *b),
        (Value::Int(a), Value::Float(b)) => eval_floats(*a as f64, op, *b),
        (Value::Float(a), Value::Int(b)) => eval_floats(*a, op, *b as f64),
        (Value::String(a), Value::String(b)) => match op {
            Add => Ok(Value::String(format!("{a}{b}"))),
            Less => Ok(Value::Bool(a < b)),
            LessEqual => Ok(Value::Bool(a <= b)),
            Greater => Ok(Value::Bool(a > b)),
            GreaterEqual => Ok(Value::Bool(a >= b)),
            _ => invalid_binary(&left, op, &right),
        },
        (Value::List(a), Value::List(b)) if op == Add => {
            let mut values = a.borrow().clone();
            values.extend(b.borrow().iter().cloned());
            Ok(Value::List(Rc::new(RefCell::new(values))))
        }
        _ => invalid_binary(&left, op, &right),
    }
}

fn eval_ints(left: i64, op: BinaryOp, right: i64) -> RuntimeResult<Value> {
    use BinaryOp::*;
    match op {
        Add => left
            .checked_add(right)
            .map(Value::Int)
            .ok_or_else(|| RuntimeError::new("integer overflow")),
        Subtract => left
            .checked_sub(right)
            .map(Value::Int)
            .ok_or_else(|| RuntimeError::new("integer overflow")),
        Multiply => left
            .checked_mul(right)
            .map(Value::Int)
            .ok_or_else(|| RuntimeError::new("integer overflow")),
        Divide => {
            if right == 0 {
                Err(RuntimeError::new("division by zero"))
            } else {
                left.checked_div(right)
                    .map(Value::Int)
                    .ok_or_else(|| RuntimeError::new("integer overflow"))
            }
        }
        Remainder => {
            if right == 0 {
                Err(RuntimeError::new("division by zero"))
            } else {
                left.checked_rem(right)
                    .map(Value::Int)
                    .ok_or_else(|| RuntimeError::new("integer overflow"))
            }
        }
        Less => Ok(Value::Bool(left < right)),
        LessEqual => Ok(Value::Bool(left <= right)),
        Greater => Ok(Value::Bool(left > right)),
        GreaterEqual => Ok(Value::Bool(left >= right)),
        _ => unreachable!("non-numeric operator handled before numeric dispatch"),
    }
}

fn eval_floats(left: f64, op: BinaryOp, right: f64) -> RuntimeResult<Value> {
    use BinaryOp::*;
    match op {
        Add => Ok(Value::Float(left + right)),
        Subtract => Ok(Value::Float(left - right)),
        Multiply => Ok(Value::Float(left * right)),
        Divide if right != 0.0 => Ok(Value::Float(left / right)),
        Divide => Err(RuntimeError::new("division by zero")),
        Remainder if right != 0.0 => Ok(Value::Float(left % right)),
        Remainder => Err(RuntimeError::new("division by zero")),
        Less => Ok(Value::Bool(left < right)),
        LessEqual => Ok(Value::Bool(left <= right)),
        Greater => Ok(Value::Bool(left > right)),
        GreaterEqual => Ok(Value::Bool(left >= right)),
        _ => unreachable!("non-numeric operator handled before numeric dispatch"),
    }
}

fn invalid_binary(left: &Value, op: BinaryOp, right: &Value) -> RuntimeResult<Value> {
    Err(RuntimeError::new(format!(
        "operator {op:?} is not defined for {} and {}",
        left.type_name(),
        right.type_name()
    )))
}

fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => a.partial_cmp(b).is_some_and(|order| order.is_eq()),
        (Value::Int(a), Value::Float(b)) => (*a as f64)
            .partial_cmp(b)
            .is_some_and(|order| order.is_eq()),
        (Value::Float(a), Value::Int(b)) => a
            .partial_cmp(&(*b as f64))
            .is_some_and(|order| order.is_eq()),
        (Value::String(a), Value::String(b)) => a == b,
        (Value::List(a), Value::List(b)) => {
            let a = a.borrow();
            let b = b.borrow();
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| values_equal(a, b))
        }
        (Value::Record(a), Value::Record(b)) => {
            a.name == b.name
                && a.fields.len() == b.fields.len()
                && a.fields.iter().all(|(name, value)| {
                    b.fields
                        .get(name)
                        .is_some_and(|other| values_equal(value, other))
                })
        }
        _ => false,
    }
}

fn member(object: Value, name: &str) -> RuntimeResult<Value> {
    match object {
        Value::Record(record) => record
            .fields
            .get(name)
            .cloned()
            .ok_or_else(|| RuntimeError::new(format!("{} has no field '{name}'", record.name))),
        Value::String(value) if name == "length" => usize_to_int(value.chars().count()),
        Value::List(values) if name == "length" => usize_to_int(values.borrow().len()),
        value => Err(RuntimeError::new(format!(
            "{} has no member '{name}'",
            value.type_name()
        ))),
    }
}

fn index_value(object: Value, index: Value) -> RuntimeResult<Value> {
    match (object, index) {
        (Value::List(values), Value::Int(index)) => {
            let index = normalize_index(index, values.borrow().len())?;
            Ok(values.borrow()[index].clone())
        }
        (Value::String(value), Value::Int(index)) => {
            let chars: Vec<char> = value.chars().collect();
            let index = normalize_index(index, chars.len())?;
            Ok(Value::String(chars[index].to_string()))
        }
        (Value::Record(record), Value::String(name)) => record
            .fields
            .get(&name)
            .cloned()
            .ok_or_else(|| RuntimeError::new(format!("{} has no field '{name}'", record.name))),
        (object, index) => Err(RuntimeError::new(format!(
            "cannot index {} with {}",
            object.type_name(),
            index.type_name()
        ))),
    }
}

fn normalize_index(index: i64, len: usize) -> RuntimeResult<usize> {
    let len_i64 = i64::try_from(len).map_err(|_| RuntimeError::new("collection too large"))?;
    let normalized = if index < 0 { len_i64 + index } else { index };
    if normalized < 0 || normalized >= len_i64 {
        return Err(RuntimeError::new(format!(
            "index {index} is out of bounds for length {len}"
        )));
    }
    usize::try_from(normalized).map_err(|_| RuntimeError::new("invalid index"))
}

fn usize_to_int(value: usize) -> RuntimeResult<Value> {
    i64::try_from(value)
        .map(Value::Int)
        .map_err(|_| RuntimeError::new("value exceeds Int range"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check;

    fn eval(source: &str) -> RuntimeResult<Value> {
        let program = check(source).unwrap();
        Interpreter::new().eval_program(&program)
    }

    #[test]
    fn immutable_by_default_and_var_is_mutable() {
        assert!(eval("x = 1\nx = 2").unwrap_err().message.contains("immutable"));
        assert_eq!(eval("var x = 1\nx = 2\nx").unwrap().to_string(), "2");
    }

    #[test]
    fn functions_capture_lexical_scope() {
        let source = "base = 40\nadd(x) => base + x\nadd(2)";
        assert_eq!(eval(source).unwrap().to_string(), "42");
    }

    #[test]
    fn data_computed_fields_work() {
        let source = r#"
User(name: String) {
    greeting => "Hello, " + name
}
user = User("Ada")
user.greeting
"#;
        assert_eq!(eval(source).unwrap().to_string(), "Hello, Ada");
    }

    #[test]
    fn when_and_elvis_are_expressions() {
        let source = r#"
value = null ?: 2
when value {
    1 => "one"
    2 => "two"
    else => "other"
}
"#;
        assert_eq!(eval(source).unwrap().to_string(), "two");
    }

    #[test]
    fn return_bubbles_through_if_expression() {
        let source = r#"
choose(x) {
    if x > 0 {
        return "yes"
    }
    "no"
}
choose(1)
"#;
        assert_eq!(eval(source).unwrap().to_string(), "yes");
    }
}
