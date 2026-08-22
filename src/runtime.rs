use crate::ast::{
    BinaryOp, Block, DataDecl, ElseBranch, EnumDecl, Expr, FunctionBody, FunctionDecl, Param,
    Program, Stmt, UnaryOp, WhenBody,
};
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::rc::Rc;

pub type RuntimeResult<T> = Result<T, RuntimeError>;

type EnvRef = Rc<RefCell<Environment>>;
type NativeFn = fn(Vec<Value>) -> RuntimeResult<Value>;

#[derive(Debug, Clone)]
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
    Map(Rc<RefCell<Vec<(Value, Value)>>>),
    Set(Rc<RefCell<Vec<Value>>>),
    Cursor(Rc<RefCell<CursorState>>),
    Record(Rc<Record>),
    Function(Rc<UserFunction>),
    Constructor(Rc<DataType>),
    Native(NativeFunction),
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
            Self::Map(_) => "Map",
            Self::Set(_) => "Set",
            Self::Cursor(_) => "Cursor",
            Self::Record(record) => &record.type_name,
            Self::Function(_) => "Function",
            Self::Constructor(_) => "Type",
            Self::Native(_) => "Function",
        }
    }

    fn expect_bool(&self, context: &str) -> RuntimeResult<bool> {
        match self {
            Self::Bool(value) => Ok(*value),
            _ => Err(RuntimeError::new(format!(
                "{context} expects Bool, got {}",
                self.type_name()
            ))),
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("Null"),
            Self::Bool(value) => f.debug_tuple("Bool").field(value).finish(),
            Self::Int(value) => f.debug_tuple("Int").field(value).finish(),
            Self::Float(value) => f.debug_tuple("Float").field(value).finish(),
            Self::String(value) => f.debug_tuple("String").field(value).finish(),
            Self::List(values) => f.debug_tuple("List").field(&*values.borrow()).finish(),
            Self::Map(entries) => f.debug_tuple("Map").field(&*entries.borrow()).finish(),
            Self::Set(values) => f.debug_tuple("Set").field(&*values.borrow()).finish(),
            Self::Cursor(cursor) => f.debug_tuple("Cursor").field(&*cursor.borrow()).finish(),
            Self::Record(record) => f.debug_tuple("Record").field(record).finish(),
            Self::Function(function) => f.debug_tuple("Function").field(&function.name).finish(),
            Self::Constructor(data) => f.debug_tuple("Constructor").field(&data.name).finish(),
            Self::Native(function) => f.debug_tuple("Native").field(&function.name).finish(),
        }
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
            Self::Map(entries) => {
                f.write_str("{")?;
                for (index, (key, value)) in entries.borrow().iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{key}: {value}")?;
                }
                f.write_str("}")
            }
            Self::Set(values) => {
                f.write_str("#{")?;
                for (index, value) in values.borrow().iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{value}")?;
                }
                f.write_str("}")
            }
            Self::Cursor(cursor) => {
                let cursor = cursor.borrow();
                write!(f, "<cursor {}/{}>", cursor.position, cursor.values.len())
            }
            Self::Record(record) => {
                write!(f, "{}(", record.name)?;
                for (index, (name, value)) in record.fields.iter().enumerate() {
                    if index > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{name}={value}")?;
                }
                f.write_str(")")
            }
            Self::Function(function) => write!(f, "<function {}>", function.name),
            Self::Constructor(data) => write!(f, "<type {}>", data.name),
            Self::Native(function) => write!(f, "<native {}>", function.name),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CursorState {
    values: Vec<Value>,
    position: usize,
    is_string: bool,
}

#[derive(Debug)]
pub struct Record {
    pub name: String,
    pub type_name: String,
    pub fields: BTreeMap<String, Value>,
    pub positional_fields: Vec<String>,
}

#[derive(Clone)]
pub struct UserFunction {
    name: String,
    params: Vec<Param>,
    body: FunctionBody,
    closure: EnvRef,
}

#[derive(Clone)]
pub struct DataType {
    name: String,
    type_name: String,
    params: Vec<Param>,
    computed: Vec<(String, Expr)>,
    closure: EnvRef,
}

#[derive(Clone, Copy)]
pub struct NativeFunction {
    name: &'static str,
    call: NativeFn,
}

#[derive(Clone)]
struct Binding {
    value: Value,
    mutable: bool,
}

#[derive(Default)]
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
    Break,
    Continue,
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
            Err(Signal::Break) => Err(RuntimeError::new("'break' used outside a loop")),
            Err(Signal::Continue) => Err(RuntimeError::new("'continue' used outside a loop")),
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
            Stmt::While { condition, body } => {
                let mut last = Value::Null;
                loop {
                    let condition = self.eval_expr(condition, Rc::clone(&env))?;
                    if !condition.expect_bool("while condition")? {
                        break;
                    }
                    match self.eval_block(body, Rc::clone(&env)) {
                        Ok(value) => last = value,
                        Err(Signal::Break) => break,
                        Err(Signal::Continue) => continue,
                        Err(signal) => return Err(signal),
                    }
                }
                Ok(last)
            }
            Stmt::Break => Err(Signal::Break),
            Stmt::Continue => Err(Signal::Continue),
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
                    match self.eval_block(body, iteration_env) {
                        Ok(value) => last = value,
                        Err(Signal::Break) => break,
                        Err(Signal::Continue) => continue,
                        Err(signal) => return Err(signal),
                    }
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
            Stmt::Enum(decl) => {
                self.define_enum(decl, env)?;
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
            type_name: decl.name.clone(),
            params: decl.params.clone(),
            computed: decl.computed.clone(),
            closure: Rc::clone(&env),
        };
        define(&env, &decl.name, Value::Constructor(Rc::new(data)), false)?;
        Ok(())
    }

    fn define_enum(&self, decl: &EnumDecl, env: EnvRef) -> EvalResult<()> {
        for variant in &decl.variants {
            let value = if variant.params.is_empty() {
                Value::Record(Rc::new(Record {
                    name: variant.name.clone(),
                    type_name: decl.name.clone(),
                    fields: BTreeMap::new(),
                    positional_fields: Vec::new(),
                }))
            } else {
                Value::Constructor(Rc::new(DataType {
                    name: variant.name.clone(),
                    type_name: decl.name.clone(),
                    params: variant.params.clone(),
                    computed: Vec::new(),
                    closure: Rc::clone(&env),
                }))
            };
            define(&env, &variant.name, value, false)?;
        }
        Ok(())
    }

    fn assign_target(&mut self, target: &Expr, value: Value, env: EnvRef) -> EvalResult<()> {
        match target {
            Expr::Ident(name) if name == "_" => Ok(()),
            Expr::Ident(name) => {
                assign_or_define(&env, name, value)?;
                Ok(())
            }
            Expr::List(patterns) => {
                let Value::List(values) = value else {
                    return Err(RuntimeError::new("list destructuring expects List").into());
                };
                let values = values.borrow().clone();
                let rest = patterns.last().and_then(|pattern| match pattern {
                    Expr::Rest(name) => Some(name),
                    _ => None,
                });
                let fixed = patterns.len().saturating_sub(usize::from(rest.is_some()));
                if (rest.is_none() && fixed != values.len())
                    || (rest.is_some() && values.len() < fixed)
                {
                    return Err(RuntimeError::new(format!(
                        "list destructuring expects {} values{}, got {}",
                        fixed,
                        if rest.is_some() { " or more" } else { "" },
                        values.len()
                    ))
                    .into());
                }
                for (pattern, value) in patterns[..fixed].iter().zip(values[..fixed].iter()) {
                    self.assign_target(pattern, value.clone(), Rc::clone(&env))?;
                }
                if let Some(name) = rest {
                    self.assign_target(
                        &Expr::Ident(name.clone()),
                        Value::List(Rc::new(RefCell::new(values[fixed..].to_vec()))),
                        env,
                    )?;
                }
                Ok(())
            }
            Expr::Rest(_) => {
                Err(RuntimeError::new("rest pattern is only valid inside a list pattern").into())
            }
            Expr::Call { callee, args } => {
                let Expr::Ident(expected_name) = callee.as_ref() else {
                    return Err(
                        RuntimeError::new("constructor destructuring expects a type name").into(),
                    );
                };
                let Value::Record(record) = value else {
                    return Err(RuntimeError::new(format!(
                        "{expected_name} destructuring expects a record"
                    ))
                    .into());
                };
                if &record.name != expected_name {
                    return Err(RuntimeError::new(format!(
                        "expected {expected_name}, got {}",
                        record.name
                    ))
                    .into());
                }
                if args.len() != record.positional_fields.len() {
                    return Err(RuntimeError::new(format!(
                        "{} pattern expects {} fields, got {}",
                        record.name,
                        record.positional_fields.len(),
                        args.len()
                    ))
                    .into());
                }
                let values: Vec<Value> = record
                    .positional_fields
                    .iter()
                    .map(|name| {
                        record
                            .fields
                            .get(name)
                            .expect("positional record field must exist")
                            .clone()
                    })
                    .collect();
                for (pattern, value) in args.iter().zip(values) {
                    self.assign_target(pattern, value, Rc::clone(&env))?;
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

    fn match_pattern(&mut self, pattern: &Expr, value: &Value, env: EnvRef) -> EvalResult<bool> {
        match pattern {
            Expr::Ident(name) if name == "_" => Ok(true),
            Expr::Ident(name) if is_binding_pattern_name(name) => {
                define(&env, name, value.clone(), false)?;
                Ok(true)
            }
            Expr::List(patterns) => {
                let Value::List(values) = value else {
                    return Ok(false);
                };
                let values = values.borrow().clone();
                let rest = patterns.last().and_then(|pattern| match pattern {
                    Expr::Rest(name) => Some(name),
                    _ => None,
                });
                let fixed = patterns.len().saturating_sub(usize::from(rest.is_some()));
                if (rest.is_none() && fixed != values.len())
                    || (rest.is_some() && values.len() < fixed)
                {
                    return Ok(false);
                }
                for (pattern, value) in patterns[..fixed].iter().zip(values[..fixed].iter()) {
                    if !self.match_pattern(pattern, value, Rc::clone(&env))? {
                        return Ok(false);
                    }
                }
                if let Some(name) = rest {
                    if name != "_" {
                        define(
                            &env,
                            name,
                            Value::List(Rc::new(RefCell::new(values[fixed..].to_vec()))),
                            false,
                        )?;
                    }
                }
                Ok(true)
            }
            Expr::Rest(_) => {
                Err(RuntimeError::new("rest pattern is only valid inside a list pattern").into())
            }
            Expr::Call { callee, args } => {
                let Expr::Ident(expected_name) = callee.as_ref() else {
                    let expected = self.eval_expr(pattern, env)?;
                    return Ok(values_equal(value, &expected));
                };
                let Value::Record(record) = value else {
                    return Ok(false);
                };
                if &record.name != expected_name {
                    return Ok(false);
                }
                if args.len() != record.positional_fields.len() {
                    return Err(RuntimeError::new(format!(
                        "{} pattern expects {} fields, got {}",
                        record.name,
                        record.positional_fields.len(),
                        args.len()
                    ))
                    .into());
                }
                for (pattern, field_name) in args.iter().zip(record.positional_fields.iter()) {
                    let field = record
                        .fields
                        .get(field_name)
                        .expect("positional record field must exist");
                    if !self.match_pattern(pattern, field, Rc::clone(&env))? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            _ => {
                let expected = self.eval_expr(pattern, env)?;
                Ok(values_equal(value, &expected))
            }
        }
    }

    fn eval_when_body(&mut self, body: &WhenBody, env: EnvRef) -> EvalResult<Value> {
        match body {
            WhenBody::Expr(expr) => self.eval_expr(expr, env),
            WhenBody::Block(block) => self.eval_block(block, env),
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
            Expr::Map(entries) => {
                let mut values: Vec<(Value, Value)> = Vec::with_capacity(entries.len());
                for (key, value) in entries {
                    let key = self.eval_expr(key, Rc::clone(&env))?;
                    let value = self.eval_expr(value, Rc::clone(&env))?;
                    if let Some((_, existing)) = values
                        .iter_mut()
                        .find(|(existing, _)| values_equal(existing, &key))
                    {
                        *existing = value;
                    } else {
                        values.push((key, value));
                    }
                }
                Ok(Value::Map(Rc::new(RefCell::new(values))))
            }
            Expr::Set(items) => {
                let mut values = Vec::with_capacity(items.len());
                for item in items {
                    let value = self.eval_expr(item, Rc::clone(&env))?;
                    if !values.iter().any(|existing| values_equal(existing, &value)) {
                        values.push(value);
                    }
                }
                Ok(Value::Set(Rc::new(RefCell::new(values))))
            }
            Expr::Rest(_) => {
                Err(RuntimeError::new("rest pattern cannot be evaluated as a value").into())
            }
            Expr::Unary { op, expr } => {
                let value = self.eval_expr(expr, env)?;
                eval_unary(*op, value).map_err(Into::into)
            }
            Expr::Binary { left, op, right } => self.eval_binary(left, *op, right, env),
            Expr::Call { callee, args } => {
                if let Expr::Member { object, name, safe } = callee.as_ref() {
                    let receiver = self.eval_expr(object, Rc::clone(&env))?;
                    if *safe && matches!(receiver, Value::Null) {
                        return Ok(Value::Null);
                    }
                    let mut argument_values = Vec::with_capacity(args.len());
                    for arg in args {
                        argument_values.push(self.eval_expr(arg, Rc::clone(&env))?);
                    }
                    if matches!(receiver, Value::Cursor(_))
                        && matches!(
                            name.as_str(),
                            "take_while" | "take_until" | "skip_while" | "skip_until"
                        )
                    {
                        return self.call_cursor_predicate_method(receiver, name, argument_values);
                    }
                    match member(receiver.clone(), name) {
                        Ok(member_value) => return self.call(member_value, argument_values),
                        Err(member_error) => {
                            if let Some(function) = get(&env, name) {
                                if is_callable(&function) {
                                    let mut values = Vec::with_capacity(argument_values.len() + 1);
                                    values.push(receiver);
                                    values.extend(argument_values);
                                    return self.call(function, values);
                                }
                            }
                            return Err(member_error.into());
                        }
                    }
                }

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
            Expr::Propagate { expr } => {
                let value = self.eval_expr(expr, env)?;
                match value {
                    Value::Record(record) if record.name == "Ok" || record.name == "Some" => {
                        record_payload(&record).ok_or_else(|| {
                            Signal::Runtime(RuntimeError::new(format!(
                                "{} must contain one payload value",
                                record.name
                            )))
                        })
                    }
                    Value::Record(record) if record.name == "Err" || record.name == "None" => {
                        Err(Signal::Return(Value::Record(record)))
                    }
                    value => Err(RuntimeError::new(format!(
                        "postfix '?' expects Ok/Err or Some/None, got {}",
                        value.type_name()
                    ))
                    .into()),
                }
            }
            Expr::Lambda { params, body } => Ok(Value::Function(Rc::new(UserFunction {
                name: "<lambda>".into(),
                params: params.clone(),
                body: FunctionBody::Expr((**body).clone()),
                closure: env,
            }))),
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
            Expr::When {
                binding,
                subject,
                cases,
            } => {
                let subject = match subject {
                    Some(subject) => Some(self.eval_expr(subject, Rc::clone(&env))?),
                    None => None,
                };

                for case in cases {
                    if case.is_else {
                        let case_env = Environment::child(&env);
                        if let (Some(binding), Some(subject)) = (binding, subject.as_ref()) {
                            define(&case_env, binding, subject.clone(), false)?;
                        }
                        return self.eval_when_body(&case.body, case_env);
                    }

                    if let Some(subject) = &subject {
                        for pattern in &case.patterns {
                            let case_env = Environment::child(&env);
                            if let Some(binding) = binding {
                                define(&case_env, binding, subject.clone(), false)?;
                            }
                            if !self.match_pattern(pattern, subject, Rc::clone(&case_env))? {
                                continue;
                            }
                            if let Some(guard) = &case.guard {
                                let guard = self.eval_expr(guard, Rc::clone(&case_env))?;
                                if !guard.expect_bool("when guard")? {
                                    continue;
                                }
                            }
                            return self.eval_when_body(&case.body, case_env);
                        }
                    } else {
                        for condition in &case.patterns {
                            let case_env = Environment::child(&env);
                            let condition = self.eval_expr(condition, Rc::clone(&case_env))?;
                            if !condition.expect_bool("when condition")? {
                                continue;
                            }
                            if let Some(guard) = &case.guard {
                                let guard = self.eval_expr(guard, Rc::clone(&case_env))?;
                                if !guard.expect_bool("when guard")? {
                                    continue;
                                }
                            }
                            return self.eval_when_body(&case.body, case_env);
                        }
                    }
                }
                Ok(Value::Null)
            }
        }
    }

    fn call_cursor_predicate_method(
        &mut self,
        receiver: Value,
        name: &str,
        args: Vec<Value>,
    ) -> EvalResult<Value> {
        require_arity(name, 1, args.len())?;
        let predicate = args[0].clone();
        let cursor = cursor_ref(&receiver)?;
        let mut taken = Vec::new();

        loop {
            let current = {
                let cursor = cursor.borrow();
                cursor.values.get(cursor.position).cloned()
            };
            let Some(current) = current else { break };
            let matched = self
                .call(predicate.clone(), vec![current.clone()])?
                .expect_bool(name)?;
            let should_consume = match name {
                "take_while" | "skip_while" => matched,
                "take_until" | "skip_until" => !matched,
                _ => unreachable!("validated cursor predicate method"),
            };
            if !should_consume {
                break;
            }
            if name.starts_with("take_") {
                taken.push(current);
            }
            cursor.borrow_mut().position += 1;
        }

        if name.starts_with("skip_") {
            return Ok(Value::Null);
        }
        let is_string = cursor.borrow().is_string;
        cursor_values_to_value(is_string, taken).map_err(Into::into)
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
            BinaryOp::Or => {
                if left.expect_bool("'||' left operand")? {
                    Ok(Value::Bool(true))
                } else {
                    let right = self.eval_expr(right, env)?;
                    Ok(Value::Bool(right.expect_bool("'||' right operand")?))
                }
            }
            BinaryOp::And => {
                if !left.expect_bool("'&&' left operand")? {
                    Ok(Value::Bool(false))
                } else {
                    let right = self.eval_expr(right, env)?;
                    Ok(Value::Bool(right.expect_bool("'&&' right operand")?))
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
                    Err(Signal::Break) => {
                        Err(RuntimeError::new("'break' used outside a loop").into())
                    }
                    Err(Signal::Continue) => {
                        Err(RuntimeError::new("'continue' used outside a loop").into())
                    }
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
                    type_name: data.type_name.clone(),
                    fields,
                    positional_fields: data.params.iter().map(|param| param.name.clone()).collect(),
                })))
            }
            value => Err(RuntimeError::new(format!(
                "{} value {value:?} is not callable",
                value.type_name()
            ))
            .into()),
        }
    }
}

fn define(env: &EnvRef, name: &str, value: Value, mutable: bool) -> RuntimeResult<()> {
    let mut env = env.borrow_mut();
    match env.values.entry(name.to_owned()) {
        std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(Binding { value, mutable });
            Ok(())
        }
        std::collections::hash_map::Entry::Occupied(_) => Err(RuntimeError::new(format!(
            "name '{name}' is already defined in this scope"
        ))),
    }
}

fn get(env: &EnvRef, name: &str) -> Option<Value> {
    let mut current = Rc::clone(env);
    loop {
        let (value, parent) = {
            let current = current.borrow();
            (
                current
                    .values
                    .get(name)
                    .map(|binding| binding.value.clone()),
                current.parent.clone(),
            )
        };
        if value.is_some() {
            return value;
        }
        current = parent?;
    }
}

fn assign_or_define(env: &EnvRef, name: &str, value: Value) -> RuntimeResult<()> {
    let mut current = Rc::clone(env);
    loop {
        let parent = {
            let mut current = current.borrow_mut();
            if let Some(binding) = current.values.get_mut(name) {
                if !binding.mutable {
                    return Err(RuntimeError::new(format!(
                        "cannot assign to immutable binding '{name}'"
                    )));
                }
                binding.value = value;
                return Ok(());
            }
            current.parent.clone()
        };
        let Some(parent) = parent else { break };
        current = parent;
    }

    define(env, name, value, false)
}

fn install_builtins(env: &EnvRef) {
    for (name, call) in [
        ("print", builtin_print as NativeFn),
        ("println", builtin_println as NativeFn),
        ("len", builtin_len as NativeFn),
        ("type", builtin_type as NativeFn),
        ("assert", builtin_assert as NativeFn),
        ("range", builtin_range as NativeFn),
        ("range_inclusive", builtin_range_inclusive as NativeFn),
        ("push", builtin_push as NativeFn),
        ("contains", builtin_contains as NativeFn),
        ("get", builtin_get as NativeFn),
        ("slice", builtin_slice as NativeFn),
        ("slice_inclusive", builtin_slice_inclusive as NativeFn),
        ("cursor", builtin_cursor as NativeFn),
        ("current", builtin_current as NativeFn),
        ("peek", builtin_peek as NativeFn),
        ("peek_string", builtin_peek_string as NativeFn),
        ("advance", builtin_advance as NativeFn),
        ("take", builtin_take as NativeFn),
        ("done", builtin_done as NativeFn),
        ("starts_with", builtin_starts_with as NativeFn),
        ("consume", builtin_consume as NativeFn),
        ("expect_next", builtin_expect_next as NativeFn),
        ("match_longest", builtin_match_longest as NativeFn),
        ("position", builtin_position as NativeFn),
        ("is_digit", builtin_is_digit as NativeFn),
        ("is_upper", builtin_is_upper as NativeFn),
        ("is_lower", builtin_is_lower as NativeFn),
        ("is_letter", builtin_is_letter as NativeFn),
        ("is_alphanumeric", builtin_is_alphanumeric as NativeFn),
        ("is_whitespace", builtin_is_whitespace as NativeFn),
        ("first", builtin_first as NativeFn),
        ("last", builtin_last as NativeFn),
        ("is_empty", builtin_is_empty as NativeFn),
        ("trim", builtin_trim as NativeFn),
        ("upper", builtin_upper as NativeFn),
        ("lower", builtin_lower as NativeFn),
        ("string", builtin_string as NativeFn),
        ("is_ok", builtin_is_ok as NativeFn),
        ("is_err", builtin_is_err as NativeFn),
        ("is_some", builtin_is_some as NativeFn),
        ("is_none", builtin_is_none as NativeFn),
        ("unwrap", builtin_unwrap as NativeFn),
        ("unwrap_or", builtin_unwrap_or as NativeFn),
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

    install_variant(env, "Ok", &["value"]);
    install_variant(env, "Err", &["error"]);
    install_variant(env, "Some", &["value"]);
    install_variant(env, "None", &[]);
}

fn install_variant(env: &EnvRef, name: &str, fields: &[&str]) {
    let params = fields
        .iter()
        .map(|field| Param {
            name: (*field).to_owned(),
            type_name: None,
        })
        .collect();
    let data = DataType {
        name: name.to_owned(),
        type_name: name.to_owned(),
        params,
        computed: Vec::new(),
        closure: Rc::clone(env),
    };
    env.borrow_mut().values.insert(
        name.to_owned(),
        Binding {
            value: Value::Constructor(Rc::new(data)),
            mutable: false,
        },
    );
}

fn record_payload(record: &Record) -> Option<Value> {
    let field = record.positional_fields.first()?;
    record.fields.get(field).cloned()
}

fn is_variant(value: &Value, name: &str) -> bool {
    matches!(value, Value::Record(record) if record.name == name)
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
        Value::Map(entries) => entries.borrow().len(),
        Value::Set(values) => values.borrow().len(),
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

fn builtin_range_inclusive(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("range_inclusive", 2, args.len())?;
    let [Value::Int(start), Value::Int(end)] = args.as_slice() else {
        return Err(RuntimeError::new("range_inclusive() arguments must be Int"));
    };
    let values = (*start..=*end).map(Value::Int).collect();
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

fn builtin_contains(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("contains", 2, args.len())?;
    let contains = match (&args[0], &args[1]) {
        (Value::List(values), needle) => values
            .borrow()
            .iter()
            .any(|value| values_equal(value, needle)),
        (Value::String(value), Value::String(needle)) => value.contains(needle),
        (Value::Map(entries), key) => entries
            .borrow()
            .iter()
            .any(|(existing, _)| values_equal(existing, key)),
        (Value::Set(values), needle) => values
            .borrow()
            .iter()
            .any(|value| values_equal(value, needle)),
        (Value::Record(record), Value::String(field)) => record.fields.contains_key(field),
        (value, _) => {
            return Err(RuntimeError::new(format!(
                "contains() does not accept {}",
                value.type_name()
            )))
        }
    };
    Ok(Value::Bool(contains))
}

fn builtin_get(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("get", 2, args.len())?;
    match (&args[0], &args[1]) {
        (Value::List(values), Value::Int(index)) => {
            let values = values.borrow();
            let Some(index) = safe_index(*index, values.len()) else {
                return Ok(Value::Null);
            };
            Ok(values[index].clone())
        }
        (Value::String(value), Value::Int(index)) => {
            let chars: Vec<char> = value.chars().collect();
            let Some(index) = safe_index(*index, chars.len()) else {
                return Ok(Value::Null);
            };
            Ok(Value::String(chars[index].to_string()))
        }
        (Value::Map(entries), key) => Ok(entries
            .borrow()
            .iter()
            .find(|(existing, _)| values_equal(existing, key))
            .map(|(_, value)| value.clone())
            .unwrap_or(Value::Null)),
        (Value::Record(record), Value::String(name)) => {
            Ok(record.fields.get(name).cloned().unwrap_or(Value::Null))
        }
        (value, _) => Err(RuntimeError::new(format!(
            "get() does not accept {}",
            value.type_name()
        ))),
    }
}

fn builtin_slice(args: Vec<Value>) -> RuntimeResult<Value> {
    slice_value(&args, false)
}

fn builtin_slice_inclusive(args: Vec<Value>) -> RuntimeResult<Value> {
    slice_value(&args, true)
}

fn slice_value(args: &[Value], inclusive: bool) -> RuntimeResult<Value> {
    let name = if inclusive {
        "slice_inclusive"
    } else {
        "slice"
    };
    require_arity(name, 3, args.len())?;
    let (Value::Int(start), Value::Int(end)) = (&args[1], &args[2]) else {
        return Err(RuntimeError::new(format!("{name}() bounds must be Int")));
    };

    match &args[0] {
        Value::List(values) => {
            let values = values.borrow();
            let (start, end) = normalize_slice_bounds(*start, *end, values.len(), inclusive)?;
            Ok(Value::List(Rc::new(RefCell::new(
                values[start..end].to_vec(),
            ))))
        }
        Value::String(value) => {
            let chars: Vec<char> = value.chars().collect();
            let (start, end) = normalize_slice_bounds(*start, *end, chars.len(), inclusive)?;
            Ok(Value::String(chars[start..end].iter().collect()))
        }
        value => Err(RuntimeError::new(format!(
            "{name}() does not accept {}",
            value.type_name()
        ))),
    }
}

fn builtin_cursor(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("cursor", 1, args.len())?;
    let (values, is_string) = match &args[0] {
        Value::List(values) => (values.borrow().clone(), false),
        Value::String(value) => (
            value
                .chars()
                .map(|ch| Value::String(ch.to_string()))
                .collect(),
            true,
        ),
        value => {
            return Err(RuntimeError::new(format!(
                "cursor() does not accept {}",
                value.type_name()
            )))
        }
    };
    Ok(Value::Cursor(Rc::new(RefCell::new(CursorState {
        values,
        position: 0,
        is_string,
    }))))
}

fn cursor_ref(value: &Value) -> RuntimeResult<Rc<RefCell<CursorState>>> {
    match value {
        Value::Cursor(cursor) => Ok(Rc::clone(cursor)),
        value => Err(RuntimeError::new(format!(
            "cursor operation expects Cursor, got {}",
            value.type_name()
        ))),
    }
}

fn builtin_current(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("current", 1, args.len())?;
    let cursor = cursor_ref(&args[0])?;
    let cursor = cursor.borrow();
    Ok(cursor
        .values
        .get(cursor.position)
        .cloned()
        .unwrap_or(Value::Null))
}

fn builtin_peek(args: Vec<Value>) -> RuntimeResult<Value> {
    if !(1..=2).contains(&args.len()) {
        return Err(RuntimeError::new(format!(
            "peek() expects 1 or 2 arguments, got {}",
            args.len()
        )));
    }
    let offset = match args.get(1) {
        None => 1usize,
        Some(Value::Int(value)) if *value >= 0 => {
            usize::try_from(*value).map_err(|_| RuntimeError::new("peek() offset is too large"))?
        }
        Some(_) => {
            return Err(RuntimeError::new(
                "peek() offset must be a non-negative Int",
            ))
        }
    };
    let cursor = cursor_ref(&args[0])?;
    let cursor = cursor.borrow();
    Ok(cursor
        .position
        .checked_add(offset)
        .and_then(|index| cursor.values.get(index))
        .cloned()
        .unwrap_or(Value::Null))
}

fn builtin_peek_string(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("peek_string", 2, args.len())?;
    let Value::Int(count) = args[1] else {
        return Err(RuntimeError::new("peek_string() count must be Int"));
    };
    if count < 0 {
        return Err(RuntimeError::new(
            "peek_string() count must be non-negative",
        ));
    }
    let count = usize::try_from(count).map_err(|_| RuntimeError::new("count is too large"))?;
    let cursor = cursor_ref(&args[0])?;
    let cursor = cursor.borrow();
    let end = cursor
        .position
        .saturating_add(count)
        .min(cursor.values.len());
    let mut output = String::new();
    for value in &cursor.values[cursor.position..end] {
        let Value::String(value) = value else {
            return Err(RuntimeError::new("peek_string() requires a String cursor"));
        };
        output.push_str(value);
    }
    Ok(Value::String(output))
}

fn builtin_advance(args: Vec<Value>) -> RuntimeResult<Value> {
    if !(1..=2).contains(&args.len()) {
        return Err(RuntimeError::new(format!(
            "advance() expects 1 or 2 arguments, got {}",
            args.len()
        )));
    }
    let count = match args.get(1) {
        None => 1usize,
        Some(Value::Int(value)) if *value >= 0 => usize::try_from(*value)
            .map_err(|_| RuntimeError::new("advance() count is too large"))?,
        Some(_) => {
            return Err(RuntimeError::new(
                "advance() count must be a non-negative Int",
            ))
        }
    };
    let cursor = cursor_ref(&args[0])?;
    let mut cursor = cursor.borrow_mut();
    cursor.position = cursor
        .position
        .saturating_add(count)
        .min(cursor.values.len());
    Ok(Value::Null)
}

fn builtin_take(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("take", 1, args.len())?;
    let cursor = cursor_ref(&args[0])?;
    let mut cursor = cursor.borrow_mut();
    let value = cursor
        .values
        .get(cursor.position)
        .cloned()
        .unwrap_or(Value::Null);
    cursor.position = cursor.position.saturating_add(1).min(cursor.values.len());
    Ok(value)
}

fn builtin_done(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("done", 1, args.len())?;
    let cursor = cursor_ref(&args[0])?;
    let cursor = cursor.borrow();
    Ok(Value::Bool(cursor.position >= cursor.values.len()))
}

fn builtin_starts_with(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("starts_with", 2, args.len())?;
    let cursor = cursor_ref(&args[0])?;
    let cursor = cursor.borrow();
    let matched = match &args[1] {
        Value::String(value) => cursor_matches_string(&cursor, value).is_some(),
        Value::List(values) => cursor_matches(&cursor, &values.borrow()),
        value => {
            return Err(RuntimeError::new(format!(
                "starts_with() does not accept {}",
                value.type_name()
            )))
        }
    };
    Ok(Value::Bool(matched))
}

fn cursor_values_to_value(is_string: bool, values: Vec<Value>) -> RuntimeResult<Value> {
    if is_string {
        let mut output = String::new();
        for value in values {
            let Value::String(value) = value else {
                return Err(RuntimeError::new(
                    "String cursor contained a non-String value",
                ));
            };
            output.push_str(&value);
        }
        Ok(Value::String(output))
    } else {
        Ok(Value::List(Rc::new(RefCell::new(values))))
    }
}

fn cursor_matches_string(cursor: &CursorState, needle: &str) -> Option<usize> {
    let len = needle.chars().count();
    if cursor.position.saturating_add(len) > cursor.values.len() {
        return None;
    }

    let matches = cursor.values[cursor.position..cursor.position + len]
        .iter()
        .zip(needle.chars())
        .all(|(value, expected)| {
            let Value::String(value) = value else {
                return false;
            };
            let mut chars = value.chars();
            chars.next() == Some(expected) && chars.next().is_none()
        });
    matches.then_some(len)
}

fn cursor_matches(cursor: &CursorState, needle: &[Value]) -> bool {
    if cursor.position.saturating_add(needle.len()) > cursor.values.len() {
        return false;
    }
    cursor.values[cursor.position..cursor.position + needle.len()]
        .iter()
        .zip(needle.iter())
        .all(|(left, right)| values_equal(left, right))
}

fn cursor_match_len(cursor: &CursorState, needle: &Value) -> Option<usize> {
    match needle {
        Value::String(value) => cursor_matches_string(cursor, value),
        Value::List(values) => {
            let values = values.borrow();
            cursor_matches(cursor, &values).then_some(values.len())
        }
        value => cursor_matches(cursor, std::slice::from_ref(value)).then_some(1),
    }
}

fn builtin_consume(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("consume", 2, args.len())?;
    let cursor = cursor_ref(&args[0])?;
    let match_len = {
        let cursor = cursor.borrow();
        cursor_match_len(&cursor, &args[1])
    };
    if let Some(len) = match_len {
        cursor.borrow_mut().position += len;
    }
    Ok(Value::Bool(match_len.is_some()))
}

fn builtin_expect_next(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("expect_next", 2, args.len())?;
    let cursor = cursor_ref(&args[0])?;
    let match_len = {
        let cursor = cursor.borrow();
        cursor_match_len(&cursor, &args[1])
    };
    let Some(len) = match_len else {
        return Err(RuntimeError::new(format!(
            "cursor expected {}, found {}",
            args[1],
            builtin_current(vec![args[0].clone()])?
        )));
    };
    cursor.borrow_mut().position += len;
    Ok(args[1].clone())
}

fn builtin_match_longest(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("match_longest", 2, args.len())?;
    let cursor = cursor_ref(&args[0])?;
    let candidates = match &args[1] {
        Value::List(values) | Value::Set(values) => Rc::clone(values),
        value => {
            return Err(RuntimeError::new(format!(
                "match_longest() candidates must be List or Set, got {}",
                value.type_name()
            )))
        }
    };
    let mut best: Option<(String, usize)> = None;
    let cursor_state = cursor.borrow();
    for candidate in candidates.borrow().iter() {
        let Value::String(text) = candidate else {
            return Err(RuntimeError::new(
                "match_longest() candidates must contain only String values",
            ));
        };
        if let Some(len) = cursor_matches_string(&cursor_state, text) {
            if best.as_ref().map_or(true, |(_, best_len)| len > *best_len) {
                best = Some((text.clone(), len));
            }
        }
    }
    drop(cursor_state);
    if let Some((text, len)) = best {
        cursor.borrow_mut().position += len;
        Ok(Value::String(text))
    } else {
        Ok(Value::Null)
    }
}

fn single_char(value: &Value, name: &str) -> RuntimeResult<char> {
    let Value::String(value) = value else {
        return Err(RuntimeError::new(format!("{name}() expects String")));
    };
    let mut chars = value.chars();
    let Some(ch) = chars.next() else {
        return Err(RuntimeError::new(format!("{name}() expects one character")));
    };
    if chars.next().is_some() {
        return Err(RuntimeError::new(format!("{name}() expects one character")));
    }
    Ok(ch)
}

fn builtin_is_digit(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("is_digit", 1, args.len())?;
    Ok(Value::Bool(single_char(&args[0], "is_digit")?.is_numeric()))
}

fn builtin_is_upper(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("is_upper", 1, args.len())?;
    Ok(Value::Bool(
        single_char(&args[0], "is_upper")?.is_uppercase(),
    ))
}

fn builtin_is_lower(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("is_lower", 1, args.len())?;
    Ok(Value::Bool(
        single_char(&args[0], "is_lower")?.is_lowercase(),
    ))
}

fn builtin_is_letter(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("is_letter", 1, args.len())?;
    Ok(Value::Bool(
        single_char(&args[0], "is_letter")?.is_alphabetic(),
    ))
}

fn builtin_is_alphanumeric(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("is_alphanumeric", 1, args.len())?;
    Ok(Value::Bool(
        single_char(&args[0], "is_alphanumeric")?.is_alphanumeric(),
    ))
}

fn builtin_is_whitespace(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("is_whitespace", 1, args.len())?;
    Ok(Value::Bool(
        single_char(&args[0], "is_whitespace")?.is_whitespace(),
    ))
}

fn builtin_position(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("position", 1, args.len())?;
    let cursor = cursor_ref(&args[0])?;
    let position = cursor.borrow().position;
    usize_to_int(position)
}

fn builtin_first(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("first", 1, args.len())?;
    match &args[0] {
        Value::List(values) => Ok(values.borrow().first().cloned().unwrap_or(Value::Null)),
        Value::String(value) => Ok(value
            .chars()
            .next()
            .map(|ch| Value::String(ch.to_string()))
            .unwrap_or(Value::Null)),
        value => Err(RuntimeError::new(format!(
            "first() does not accept {}",
            value.type_name()
        ))),
    }
}

fn builtin_last(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("last", 1, args.len())?;
    match &args[0] {
        Value::List(values) => Ok(values.borrow().last().cloned().unwrap_or(Value::Null)),
        Value::String(value) => Ok(value
            .chars()
            .next_back()
            .map(|ch| Value::String(ch.to_string()))
            .unwrap_or(Value::Null)),
        value => Err(RuntimeError::new(format!(
            "last() does not accept {}",
            value.type_name()
        ))),
    }
}

fn builtin_is_empty(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("is_empty", 1, args.len())?;
    let empty = match &args[0] {
        Value::List(values) => values.borrow().is_empty(),
        Value::Map(entries) => entries.borrow().is_empty(),
        Value::Set(values) => values.borrow().is_empty(),
        Value::String(value) => value.is_empty(),
        Value::Record(record) => record.fields.is_empty(),
        value => {
            return Err(RuntimeError::new(format!(
                "is_empty() does not accept {}",
                value.type_name()
            )))
        }
    };
    Ok(Value::Bool(empty))
}

fn builtin_trim(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("trim", 1, args.len())?;
    let Value::String(value) = &args[0] else {
        return Err(RuntimeError::new("trim() expects String"));
    };
    Ok(Value::String(value.trim().to_owned()))
}

fn builtin_upper(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("upper", 1, args.len())?;
    let Value::String(value) = &args[0] else {
        return Err(RuntimeError::new("upper() expects String"));
    };
    Ok(Value::String(value.to_uppercase()))
}

fn builtin_lower(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("lower", 1, args.len())?;
    let Value::String(value) = &args[0] else {
        return Err(RuntimeError::new("lower() expects String"));
    };
    Ok(Value::String(value.to_lowercase()))
}

fn builtin_string(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("string", 1, args.len())?;
    Ok(Value::String(args[0].to_string()))
}

fn builtin_is_ok(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("is_ok", 1, args.len())?;
    Ok(Value::Bool(is_variant(&args[0], "Ok")))
}

fn builtin_is_err(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("is_err", 1, args.len())?;
    Ok(Value::Bool(is_variant(&args[0], "Err")))
}

fn builtin_is_some(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("is_some", 1, args.len())?;
    Ok(Value::Bool(is_variant(&args[0], "Some")))
}

fn builtin_is_none(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("is_none", 1, args.len())?;
    Ok(Value::Bool(is_variant(&args[0], "None")))
}

fn builtin_unwrap(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("unwrap", 1, args.len())?;
    match &args[0] {
        Value::Record(record) if record.name == "Ok" || record.name == "Some" => {
            record_payload(record).ok_or_else(|| {
                RuntimeError::new(format!("{} must contain one payload value", record.name))
            })
        }
        Value::Record(record) if record.name == "Err" => {
            let detail = record_payload(record)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown error".into());
            Err(RuntimeError::new(format!(
                "unwrap() called on Err: {detail}"
            )))
        }
        Value::Record(record) if record.name == "None" => {
            Err(RuntimeError::new("unwrap() called on None"))
        }
        value => Err(RuntimeError::new(format!(
            "unwrap() expects Ok/Err or Some/None, got {}",
            value.type_name()
        ))),
    }
}

fn builtin_unwrap_or(args: Vec<Value>) -> RuntimeResult<Value> {
    require_arity("unwrap_or", 2, args.len())?;
    match &args[0] {
        Value::Record(record) if record.name == "Ok" || record.name == "Some" => {
            record_payload(record).ok_or_else(|| {
                RuntimeError::new(format!("{} must contain one payload value", record.name))
            })
        }
        Value::Record(record) if record.name == "Err" || record.name == "None" => {
            Ok(args[1].clone())
        }
        value => Err(RuntimeError::new(format!(
            "unwrap_or() expects Ok/Err or Some/None, got {}",
            value.type_name()
        ))),
    }
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
        Value::Set(values) => Ok(values.borrow().clone()),
        Value::Map(entries) => Ok(entries
            .borrow()
            .iter()
            .map(|(key, value)| {
                Value::List(Rc::new(RefCell::new(vec![key.clone(), value.clone()])))
            })
            .collect()),
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

    match (left, right) {
        (Value::Int(a), Value::Int(b)) => eval_ints(a, op, b),
        (Value::Float(a), Value::Float(b)) => eval_floats(a, op, b),
        (Value::Int(a), Value::Float(b)) => eval_floats(a as f64, op, b),
        (Value::Float(a), Value::Int(b)) => eval_floats(a, op, b as f64),
        (Value::String(mut a), Value::String(b)) => match op {
            Add => {
                a.push_str(&b);
                Ok(Value::String(a))
            }
            Less => Ok(Value::Bool(a < b)),
            LessEqual => Ok(Value::Bool(a <= b)),
            Greater => Ok(Value::Bool(a > b)),
            GreaterEqual => Ok(Value::Bool(a >= b)),
            _ => invalid_binary(&Value::String(a), op, &Value::String(b)),
        },
        (Value::List(a), Value::List(b)) if op == Add => {
            let mut values = a.borrow().clone();
            values.extend(b.borrow().iter().cloned());
            Ok(Value::List(Rc::new(RefCell::new(values))))
        }
        (left, right) => invalid_binary(&left, op, &right),
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

fn is_callable(value: &Value) -> bool {
    matches!(
        value,
        Value::Function(_) | Value::Constructor(_) | Value::Native(_)
    )
}

fn is_binding_pattern_name(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_lowercase())
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
        (Value::Set(a), Value::Set(b)) => {
            let a = a.borrow();
            let b = b.borrow();
            a.len() == b.len()
                && a.iter()
                    .all(|value| b.iter().any(|other| values_equal(value, other)))
        }
        (Value::Map(a), Value::Map(b)) => {
            let a = a.borrow();
            let b = b.borrow();
            a.len() == b.len()
                && a.iter().all(|(key, value)| {
                    b.iter().any(|(other_key, other_value)| {
                        values_equal(key, other_key) && values_equal(value, other_value)
                    })
                })
        }
        (Value::Cursor(a), Value::Cursor(b)) => Rc::ptr_eq(a, b),
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
        (Value::Map(entries), key) => entries
            .borrow()
            .iter()
            .find(|(existing, _)| values_equal(existing, &key))
            .map(|(_, value)| value.clone())
            .ok_or_else(|| RuntimeError::new(format!("map key not found: {key}"))),
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

fn safe_index(index: i64, len: usize) -> Option<usize> {
    let len = i64::try_from(len).ok()?;
    let normalized = if index < 0 { len + index } else { index };
    if normalized < 0 || normalized >= len {
        return None;
    }
    usize::try_from(normalized).ok()
}

fn normalize_slice_bound(index: i64, len: usize, allow_end: bool) -> RuntimeResult<usize> {
    let len_i64 = i64::try_from(len).map_err(|_| RuntimeError::new("collection too large"))?;
    let normalized = if index < 0 { len_i64 + index } else { index };
    let upper = if allow_end {
        len_i64
    } else {
        len_i64.saturating_sub(1)
    };
    if normalized < 0 || normalized > upper {
        return Err(RuntimeError::new(format!(
            "slice bound {index} is out of bounds for length {len}"
        )));
    }
    usize::try_from(normalized).map_err(|_| RuntimeError::new("invalid slice bound"))
}

fn normalize_slice_bounds(
    start: i64,
    end: i64,
    len: usize,
    inclusive: bool,
) -> RuntimeResult<(usize, usize)> {
    let start = normalize_slice_bound(start, len, true)?;
    let mut end = normalize_slice_bound(end, len, !inclusive)?;
    if inclusive {
        end = end
            .checked_add(1)
            .ok_or_else(|| RuntimeError::new("slice bound overflow"))?;
    }
    if start > end {
        return Err(RuntimeError::new("slice start must not exceed slice end"));
    }
    Ok((start, end))
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
        assert!(eval("x = 1\nx = 2")
            .unwrap_err()
            .message
            .contains("immutable"));
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
