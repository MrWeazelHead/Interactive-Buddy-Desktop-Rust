//! A small, deliberately bounded interpreter for the safe ShockScript subset.
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
enum Value {
    Num(f64),
    Str(String),
    Bool(bool),
    Null,
}
impl Value {
    fn number(&self) -> f64 {
        match self {
            Self::Num(n) => *n,
            Self::Bool(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            Self::Str(s) => s.trim().parse().unwrap_or(f64::NAN),
            Self::Null => 0.0,
        }
    }
    fn string(&self) -> String {
        match self {
            Self::Num(n) => {
                if n.fract() == 0.0 {
                    format!("{n:.0}")
                } else {
                    n.to_string()
                }
            }
            Self::Str(s) => s.clone(),
            Self::Bool(b) => b.to_string(),
            Self::Null => "undefined".into(),
        }
    }
    fn truth(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            Self::Num(n) => *n != 0.0 && !n.is_nan(),
            Self::Str(s) => !s.is_empty(),
            Self::Null => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Id(String),
    Op(String),
    LParen,
    RParen,
    Comma,
    Semi,
    End,
}
fn lex(s: &str) -> Result<Vec<Tok>, String> {
    let b = s.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        if b[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if b[i] == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                i += 1
            }
            continue;
        }
        if b[i].is_ascii_digit() || (b[i] == b'.' && b.get(i + 1).is_some_and(u8::is_ascii_digit)) {
            let start = i;
            i += 1;
            while i < b.len()
                && (b[i].is_ascii_digit()
                    || b[i] == b'.'
                    || b[i] == b'e'
                    || b[i] == b'E'
                    || ((b[i] == b'+' || b[i] == b'-')
                        && i > start
                        && (b[i - 1] == b'e' || b[i - 1] == b'E')))
            {
                i += 1
            }
            let n = s[start..i]
                .parse::<f64>()
                .map_err(|_| format!("invalid number at byte {start}"))?;
            out.push(Tok::Num(n));
            continue;
        }
        if b[i] == b'"' || b[i] == b'\'' {
            let q = b[i];
            i += 1;
            let mut v = String::new();
            let mut closed = false;
            while i < b.len() {
                if b[i] == q {
                    i += 1;
                    closed = true;
                    break;
                }
                if b[i] == b'\\' && i + 1 < b.len() {
                    i += 1;
                    v.push(match b[i] {
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        x => x as char,
                    });
                    i += 1;
                } else {
                    let ch = s[i..].chars().next().unwrap();
                    v.push(ch);
                    i += ch.len_utf8();
                }
            }
            if !closed {
                return Err("unterminated string literal".into());
            }
            out.push(Tok::Str(v));
            continue;
        }
        if b[i].is_ascii_alphabetic() || b[i] == b'_' || b[i] == b'$' {
            let st = i;
            i += 1;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b'$') {
                i += 1
            }
            out.push(Tok::Id(s[st..i].into()));
            continue;
        }
        match b[i] {
            b'(' => out.push(Tok::LParen),
            b')' => out.push(Tok::RParen),
            b',' => out.push(Tok::Comma),
            b';' => out.push(Tok::Semi),
            _ => {
                let rest = &s[i..];
                let op = ["==", "!=", "<=", ">=", "&&", "||", "<<", ">>>", ">>"]
                    .iter()
                    .find(|x| rest.starts_with(**x))
                    .copied()
                    .or_else(|| {
                        let c = b[i] as char;
                        if "+-*/%<>&^|=!".contains(c) {
                            Some(&s[i..i + 1])
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| format!("unexpected character '{}'", b[i] as char))?;
                out.push(Tok::Op(op.into()));
                i += op.len();
                continue;
            }
        }
        i += 1;
    }
    out.push(Tok::End);
    Ok(out)
}
#[derive(Clone, Debug)]
enum Expr {
    Val(Value),
    Var(String),
    Assign(String, Box<Expr>),
    Unary(String, Box<Expr>),
    Bin(String, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}
struct Parser {
    t: Vec<Tok>,
    i: usize,
}
impl Parser {
    fn peek(&self) -> &Tok {
        &self.t[self.i]
    }
    fn take(&mut self) -> Tok {
        let x = self.t[self.i].clone();
        self.i += 1;
        x
    }
    fn parse(mut self) -> Result<Vec<Expr>, String> {
        let mut a = vec![];
        while !matches!(self.peek(), Tok::End) {
            if matches!(self.peek(), Tok::Semi) {
                self.take();
                continue;
            }
            a.push(self.expr(0)?);
            if matches!(self.peek(), Tok::Semi) {
                self.take();
            } else if !matches!(self.peek(), Tok::End) {
                return Err("expected ';' between statements".into());
            }
        }
        Ok(a)
    }
    fn expr(&mut self, min: u8) -> Result<Expr, String> {
        let mut left = match self.take() {
            Tok::Num(n) => Expr::Val(Value::Num(n)),
            Tok::Str(s) => Expr::Val(Value::Str(s)),
            Tok::Id(id) => {
                if id == "true" {
                    Expr::Val(Value::Bool(true))
                } else if id == "false" {
                    Expr::Val(Value::Bool(false))
                } else if id == "null" || id == "undefined" {
                    Expr::Val(Value::Null)
                } else if matches!(self.peek(), Tok::LParen) {
                    self.take();
                    let mut args = vec![];
                    if !matches!(self.peek(), Tok::RParen) {
                        loop {
                            args.push(self.expr(0)?);
                            if matches!(self.peek(), Tok::Comma) {
                                self.take();
                                continue;
                            }
                            break;
                        }
                    }
                    if !matches!(self.take(), Tok::RParen) {
                        return Err("expected ')'".into());
                    }
                    Expr::Call(id, args)
                } else {
                    Expr::Var(id)
                }
            }
            Tok::Op(op) if op == "-" || op == "+" || op == "!" => {
                Expr::Unary(op, Box::new(self.expr(10)?))
            }
            Tok::LParen => {
                let e = self.expr(0)?;
                if !matches!(self.take(), Tok::RParen) {
                    return Err("expected ')'".into());
                }
                e
            }
            x => return Err(format!("expected expression, got {x:?}")),
        };
        loop {
            if let Tok::Op(op) = self.peek().clone() {
                if op == "=" {
                    if min > 0 {
                        break;
                    }
                    self.take();
                    let Expr::Var(name) = left else {
                        return Err("assignment target must be a variable".into());
                    };
                    left = Expr::Assign(name, Box::new(self.expr(0)?));
                    continue;
                }
                let p = prec(&op);
                if p < min || p == 0 {
                    break;
                }
                self.take();
                let right = self.expr(p + 1)?;
                left = Expr::Bin(op, Box::new(left), Box::new(right));
                continue;
            }
            break;
        }
        Ok(left)
    }
}
fn prec(s: &str) -> u8 {
    match s {
        "||" => 2,
        "&&" => 3,
        "|" => 4,
        "^" => 5,
        "&" => 6,
        "==" | "!=" => 7,
        "<" | "<=" | ">" | ">=" => 8,
        "<<" | ">>" | ">>>" => 9,
        "+" | "-" => 10,
        "%" => 11,
        "*" | "/" => 12,
        _ => 0,
    }
}
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Str(x), Value::Str(y)) => x == y,
        _ => a.number() == b.number(),
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Context {
    pub buddy_x: f64,
    pub buddy_y: f64,
    pub buddy_rotation: f64,
    pub emotion: f64,
    pub mouse_x: f64,
    pub mouse_y: f64,
    pub mouse_down: bool,
    pub time: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Cash(f64),
    Explode(f64, f64, f64),
    Velocity(f64, f64),
    Say(String),
    Create(String, f64, f64, f64, f64),
    Gravity(f64),
    SetVelocity(f64, f64),
    SetPosition(f64, f64),
    Rotation(f64, bool),
    Emotion(f64),
    UnlockAll,
    Bullet(f64, f64, f64, f64, f64, f64),
    Sound(String),
}
pub struct Engine {
    vars: HashMap<String, Value>,
    buffers: HashMap<(usize, usize), Value>,
    rng: u64,
    first_run: bool,
    bullet_count: u64,
}
impl Default for Engine {
    fn default() -> Self {
        Self {
            vars: HashMap::new(),
            buffers: HashMap::new(),
            rng: 0x9e3779b97f4a7c15,
            first_run: true,
            bullet_count: 0,
        }
    }
}
impl Engine {
    pub fn run(&mut self, source: &str, context: &Context) -> Result<Vec<Effect>, String> {
        let tokens = lex(source)?;
        let ast = Parser { t: tokens, i: 0 }.parse()?;
        let mut effects = vec![];
        let mut budget = 100_000usize;
        for e in ast {
            self.eval(&e, context, &mut effects, &mut budget)?;
        }
        self.first_run = false;
        Ok(effects)
    }
    /// Mark the next execution as a fresh ShockScript run, as Run Once does.
    pub fn reset_first_run(&mut self) {
        self.first_run = true;
    }
    fn eval(
        &mut self,
        e: &Expr,
        c: &Context,
        out: &mut Vec<Effect>,
        budget: &mut usize,
    ) -> Result<Value, String> {
        if *budget == 0 {
            return Err("script execution budget exceeded".into());
        }
        *budget -= 1;
        match e {
            Expr::Val(v) => Ok(v.clone()),
            Expr::Var(n) => Ok(self.vars.get(n).cloned().unwrap_or(Value::Num(0.0))),
            Expr::Assign(n, v) => {
                let x = self.eval(v, c, out, budget)?;
                self.vars.insert(n.clone(), x.clone());
                Ok(x)
            }
            Expr::Unary(op, a) => {
                let x = self.eval(a, c, out, budget)?;
                Ok(match op.as_str() {
                    "-" => Value::Num(-x.number()),
                    "+" => Value::Num(x.number()),
                    "!" => Value::Bool(!x.truth()),
                    _ => unreachable!(),
                })
            }
            Expr::Bin(op, a, b) => {
                let x = self.eval(a, c, out, budget)?;
                if op == "&&" && !x.truth() {
                    return Ok(x);
                }
                if op == "||" && x.truth() {
                    return Ok(x);
                }
                let y = self.eval(b, c, out, budget)?;
                let (u, v) = (x.number(), y.number());
                Ok(match op.as_str() {
                    "+" => {
                        if matches!(x, Value::Str(_)) || matches!(y, Value::Str(_)) {
                            Value::Str(x.string() + &y.string())
                        } else {
                            Value::Num(u + v)
                        }
                    }
                    "-" => Value::Num(u - v),
                    "*" => Value::Num(u * v),
                    "/" => Value::Num(u / v),
                    "%" => Value::Num(u % v),
                    "<" => Value::Bool(u < v),
                    "<=" => Value::Bool(u <= v),
                    ">" => Value::Bool(u > v),
                    ">=" => Value::Bool(u >= v),
                    "==" => Value::Bool(values_equal(&x, &y)),
                    "!=" => Value::Bool(!values_equal(&x, &y)),
                    "&&" => {
                        if y.truth() {
                            y
                        } else {
                            Value::Bool(false)
                        }
                    }
                    "||" => {
                        if y.truth() {
                            y
                        } else {
                            Value::Bool(false)
                        }
                    }
                    "&" => Value::Num((u as i32 & v as i32) as f64),
                    "|" => Value::Num((u as i32 | v as i32) as f64),
                    "^" => Value::Num((u as i32 ^ v as i32) as f64),
                    "<<" => Value::Num(((u as i32).wrapping_shl((v as u32) & 31)) as f64),
                    ">>" => Value::Num(((u as i32) >> ((v as u32) & 31)) as f64),
                    ">>>" => Value::Num(((u as u32) >> ((v as u32) & 31)) as f64),
                    _ => unreachable!(),
                })
            }
            Expr::Call(n, args) => self.call(n, args, c, out, budget),
        }
    }
    fn call(
        &mut self,
        n: &str,
        a: &[Expr],
        c: &Context,
        out: &mut Vec<Effect>,
        b: &mut usize,
    ) -> Result<Value, String> {
        let arity = |min: usize, max: usize| -> Result<(), String> {
            if a.len() < min || a.len() > max {
                Err(format!(
                    "{n} expects {min}{} argument(s), got {}",
                    if min == max { "" } else { " or more" },
                    a.len()
                ))
            } else {
                Ok(())
            }
        };
        if n == "if" {
            arity(3, 3)?;
            let cond = self.eval(&a[0], c, out, b)?;
            return self.eval(&a[if cond.truth() { 1 } else { 2 }], c, out, b);
        }
        if n == "loop" {
            arity(2, 2)?;
            let count = self.eval(&a[0], c, out, b)?.number();
            if count.is_nan() {
                return Err("loop count must not be NaN".into());
            }
            let limited = count >= 2000.0;
            let count = count.ceil().clamp(0.0, 2000.0) as usize;
            for _ in 0..count {
                self.eval(&a[1], c, out, b)?;
            }
            return Ok(Value::Bool(!limited));
        }
        if n == "assign" {
            arity(2, 2)?;
            let Expr::Var(name) = &a[0] else {
                return Err(
                    "assign first argument must be a variable name, e.g. assign(x,10)".into(),
                );
            };
            let v = self.eval(&a[1], c, out, b)?;
            self.vars.insert(name.clone(), v.clone());
            return Ok(v);
        }
        if n == "resetVariables" {
            arity(0, 0)?;
            self.vars.clear();
            return Ok(Value::Bool(true));
        }
        if n == "firstRun" {
            arity(0, 0)?;
            return Ok(Value::Bool(self.first_run));
        }
        if n == "setBuffer" {
            arity(3, 3)?;
            let index = num_index(self.eval(&a[0], c, out, b)?.number(), 1000, "buffer")?;
            let pos = num_index(
                self.eval(&a[1], c, out, b)?.number(),
                100000,
                "buffer position",
            )?;
            let value = self.eval(&a[2], c, out, b)?;
            self.buffers.insert((index, pos), value.clone());
            return Ok(Value::Bool(true));
        }
        if n == "getBuffer" {
            arity(2, 2)?;
            let index = num_index(self.eval(&a[0], c, out, b)?.number(), 1000, "buffer")?;
            let pos = num_index(
                self.eval(&a[1], c, out, b)?.number(),
                100000,
                "buffer position",
            )?;
            return Ok(self
                .buffers
                .get(&(index, pos))
                .cloned()
                .unwrap_or(Value::Num(0.0)));
        }
        if n == "exec2" {
            arity(2, 2)?;
            self.eval(&a[0], c, out, b)?;
            return self.eval(&a[1], c, out, b);
        }
        if n == "evaluate" {
            arity(1, 1)?;
            let source = self.eval(&a[0], c, out, b)?.string();
            if source.len() > 16_384 {
                return Err("evaluate source exceeds 16384 bytes".into());
            }
            let tokens = lex(&source)?;
            let ast = Parser { t: tokens, i: 0 }.parse()?;
            let mut value = Value::Null;
            for expression in ast {
                value = self.eval(&expression, c, out, b)?;
            }
            return Ok(value);
        }
        if n == "bor" || n == "band" {
            arity(2, 2)?;
            let left = self.eval(&a[0], c, out, b)?.number();
            if n == "bor" && left != 0.0 && !left.is_nan() {
                return Ok(Value::Num(left));
            }
            if n == "band" && (left == 0.0 || left.is_nan()) {
                return Ok(Value::Num(0.0));
            }
            return Ok(Value::Num(self.eval(&a[1], c, out, b)?.number()));
        }
        if !matches!(
            n,
            "getBuddyX"
                | "getBuddyY"
                | "getBuddyRot"
                | "getEmotion"
                | "getMouseDown"
                | "getXMouse"
                | "getYMouse"
                | "gettime"
                | "addBuddyVel"
                | "explode"
                | "create"
                | "say"
                | "secret_addCash"
                | "addCash"
                | "setGrav"
                | "setGravity"
                | "setBuddyVel"
                | "setBuddyPos"
                | "setBuddyRot"
                | "addBuddyRot"
                | "secret_addEmotion"
                | "secret_allItems"
                | "bullet"
                | "bulletRot"
                | "playSound"
                | "sin"
                | "cos"
                | "tan"
                | "atan"
                | "atan2"
                | "asin"
                | "acos"
                | "sqrt"
                | "abs"
                | "floor"
                | "ceil"
                | "round"
                | "pow"
                | "min"
                | "max"
                | "sign"
                | "pi"
                | "e"
                | "random"
                | "randomBet"
                | "bnot"
                | "bor"
                | "band"
                | "above"
                | "below"
                | "equal"
                | "sqr"
                | "exp"
                | "log"
                | "log10"
                | "rand"
                | "charAt"
                | "trace"
                | "exec2"
                | "evaluate"
        ) {
            return Err(format!("unsupported function '{n}'"));
        }
        let vals: Vec<Value> = a
            .iter()
            .map(|x| self.eval(x, c, out, b))
            .collect::<Result<_, _>>()?;
        let nums: Vec<f64> = vals.iter().map(Value::number).collect();
        macro_rules! ar {
            ($n:expr) => {
                arity($n, $n)?
            };
        }
        macro_rules! num {
            ($i:expr) => {
                nums[$i]
            };
        }
        Ok(match n {
            "getBuddyX" => {
                ar!(0);
                Value::Num(c.buddy_x)
            }
            "getBuddyY" => {
                ar!(0);
                Value::Num(c.buddy_y)
            }
            "getBuddyRot" => {
                ar!(0);
                Value::Num(c.buddy_rotation)
            }
            "getEmotion" => {
                ar!(0);
                Value::Num(c.emotion)
            }
            "getMouseDown" => {
                ar!(0);
                Value::Bool(c.mouse_down)
            }
            "getXMouse" => {
                ar!(0);
                Value::Num(c.mouse_x)
            }
            "getYMouse" => {
                ar!(0);
                Value::Num(c.mouse_y)
            }
            "gettime" => {
                ar!(0);
                Value::Num(c.time)
            }
            "addBuddyVel" => {
                ar!(2);
                out.push(Effect::Velocity(num!(0), num!(1)));
                Value::Null
            }
            "setBuddyVel" => {
                ar!(2);
                out.push(Effect::SetVelocity(num!(0), num!(1)));
                Value::Null
            }
            "setBuddyPos" => {
                ar!(2);
                out.push(Effect::SetPosition(num!(0), num!(1)));
                Value::Null
            }
            "setBuddyRot" => {
                ar!(1);
                out.push(Effect::Rotation(num!(0), false));
                Value::Num(num!(0))
            }
            "addBuddyRot" => {
                ar!(1);
                out.push(Effect::Rotation(num!(0), true));
                Value::Num(c.buddy_rotation + num!(0))
            }
            "explode" => {
                ar!(3);
                out.push(Effect::Explode(num!(0), num!(1), num!(2)));
                Value::Null
            }
            "create" => {
                ar!(5);
                out.push(Effect::Create(
                    vals[0].string(),
                    num!(1),
                    num!(2),
                    num!(3),
                    num!(4),
                ));
                Value::Str(format!("object{}", out.len()))
            }
            "say" => {
                arity(1, 2)?;
                let s = vals[0].string();
                out.push(Effect::Say(s.clone()));
                Value::Str(s)
            }
            "secret_addCash" | "addCash" => {
                ar!(1);
                out.push(Effect::Cash(num!(0)));
                Value::Num(num!(0))
            }
            "secret_addEmotion" => {
                ar!(1);
                out.push(Effect::Emotion(num!(0)));
                Value::Num(c.emotion + num!(0))
            }
            "secret_allItems" => {
                ar!(0);
                out.push(Effect::UnlockAll);
                Value::Bool(true)
            }
            "bullet" => {
                ar!(6);
                self.bullet_count += 1;
                out.push(Effect::Bullet(
                    num!(0),
                    num!(1),
                    num!(2),
                    num!(3),
                    num!(4),
                    num!(5),
                ));
                Value::Num(self.bullet_count as f64)
            }
            "bulletRot" => {
                ar!(6);
                let speed = num!(2);
                let angle = num!(3);
                self.bullet_count += 1;
                out.push(Effect::Bullet(
                    num!(0),
                    num!(1),
                    speed * angle.sin(),
                    speed * angle.cos(),
                    num!(4),
                    num!(5),
                ));
                Value::Num(self.bullet_count as f64)
            }
            "playSound" => {
                arity(1, 2)?;
                out.push(Effect::Sound(vals[0].string()));
                Value::Num(nums.get(1).copied().unwrap_or(100.0))
            }
            "setGrav" | "setGravity" => {
                ar!(1);
                out.push(Effect::Gravity(num!(0)));
                Value::Num(num!(0))
            }
            "sin" => {
                ar!(1);
                Value::Num(num!(0).sin())
            }
            "cos" => {
                ar!(1);
                Value::Num(num!(0).cos())
            }
            "tan" => {
                ar!(1);
                Value::Num(num!(0).tan())
            }
            "atan" => {
                ar!(1);
                Value::Num(num!(0).atan())
            }
            "atan2" => {
                ar!(2);
                Value::Num(num!(0).atan2(num!(1)))
            }
            "asin" => {
                ar!(1);
                Value::Num(num!(0).asin())
            }
            "acos" => {
                ar!(1);
                Value::Num(num!(0).acos())
            }
            "sqrt" => {
                ar!(1);
                Value::Num(num!(0).sqrt())
            }
            "abs" => {
                ar!(1);
                Value::Num(num!(0).abs())
            }
            "floor" => {
                ar!(1);
                Value::Num(num!(0).floor())
            }
            "ceil" => {
                ar!(1);
                Value::Num(num!(0).ceil())
            }
            "round" => {
                ar!(1);
                Value::Num(num!(0).round())
            }
            "sqr" => {
                ar!(1);
                Value::Num(num!(0) * num!(0))
            }
            "exp" => {
                ar!(1);
                Value::Num(num!(0).exp())
            }
            "log" => {
                ar!(1);
                Value::Num(num!(0).ln())
            }
            // The original log10 case also calls Math.log; keep that quirk.
            "log10" => {
                ar!(1);
                Value::Num(num!(0).ln())
            }
            "rand" => {
                ar!(1);
                self.rng ^= self.rng << 13;
                self.rng ^= self.rng >> 7;
                self.rng ^= self.rng << 17;
                Value::Num((self.rng as f64 / u64::MAX as f64 * num!(0)).floor())
            }
            "charAt" => {
                ar!(2);
                let index = num!(1);
                let character = if index.is_finite() && index >= 0.0 {
                    vals[0]
                        .string()
                        .chars()
                        .nth(index as usize)
                        .map(|ch| ch.to_string())
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                Value::Str(character)
            }
            // The app has no trace sink in Effect; return the argument unchanged.
            "trace" => {
                ar!(1);
                vals[0].clone()
            }
            "pow" => {
                ar!(2);
                Value::Num(num!(0).powf(num!(1)))
            }
            "min" => {
                ar!(2);
                Value::Num(num!(0).min(num!(1)))
            }
            "max" => {
                ar!(2);
                Value::Num(num!(0).max(num!(1)))
            }
            "sign" => {
                ar!(1);
                Value::Num(if num!(0) > 0.0 {
                    1.0
                } else if num!(0) < 0.0 {
                    -1.0
                } else {
                    0.0
                })
            }
            "pi" => {
                ar!(0);
                Value::Num(std::f64::consts::PI)
            }
            "e" => {
                ar!(0);
                Value::Num(std::f64::consts::E)
            }
            "bnot" => {
                ar!(1);
                Value::Bool(!vals[0].truth())
            }
            "bor" => {
                unreachable!("bor is evaluated lazily above")
            }
            "band" => {
                unreachable!("band is evaluated lazily above")
            }
            "above" => {
                ar!(2);
                Value::Bool(num!(0) > num!(1))
            }
            "below" => {
                ar!(2);
                Value::Bool(num!(0) < num!(1))
            }
            "equal" => {
                ar!(2);
                Value::Bool(vals[0] == vals[1] || num!(0) == num!(1))
            }
            "random" | "randomBet" => {
                arity(
                    if n == "random" { 0 } else { 2 },
                    if n == "random" { 0 } else { 2 },
                )?;
                self.rng ^= self.rng << 13;
                self.rng ^= self.rng >> 7;
                self.rng ^= self.rng << 17;
                let r = ((self.rng as f64) / (u64::MAX as f64)).clamp(0.0, 1.0);
                Value::Num(if n == "randomBet" {
                    num!(0) + (num!(1) - num!(0)) * r
                } else {
                    r.min(f64::from_bits(1.0f64.to_bits() - 1))
                })
            }
            _ => return Err(format!("unsupported function '{n}'")),
        })
    }
}

fn num_index(n: f64, max: usize, label: &str) -> Result<usize, String> {
    if !n.is_finite() || n < 0.0 || n.fract() != 0.0 || n > max as f64 {
        Err(format!("{label} index must be an integer from 0 to {max}"))
    } else {
        Ok(n as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ctx() -> Context {
        Context {
            buddy_x: 12.,
            buddy_y: 34.,
            buddy_rotation: 0.25,
            emotion: 18.0,
            mouse_x: 5.,
            mouse_y: 6.,
            mouse_down: true,
            time: 7.,
        }
    }
    #[test]
    fn precedence_and_assignment() {
        let mut e = Engine::default();
        assert_eq!(
            e.run("x=1+2*3; addBuddyVel(x, -2)", &ctx()).unwrap(),
            vec![Effect::Velocity(7., -2.)]
        );
    }
    #[test]
    fn lazy_if_and_nested_calls() {
        let mut e = Engine::default();
        assert_eq!(
            e.run(
                "if(getMouseDown(), explode(getBuddyX(),getBuddyY(),sin(pi()/2)), unknown())",
                &ctx()
            )
            .unwrap(),
            vec![Effect::Explode(12., 34., 1.)]
        );
    }
    #[test]
    fn loop_cap_and_iteration() {
        let mut e = Engine::default();
        e.run("loop(3000, x=x+1)", &ctx()).unwrap();
        assert_eq!(e.vars["x"], Value::Num(2000.));
    }
    #[test]
    fn strings_and_effects() {
        let mut e = Engine::default();
        assert_eq!(
            e.run(
                "say(\"hi \"+2); create('ball',1,2,3,4); secret_addCash(5); setGrav(.25)",
                &ctx()
            )
            .unwrap(),
            vec![
                Effect::Say("hi 2".into()),
                Effect::Create("ball".into(), 1., 2., 3., 4.),
                Effect::Cash(5.),
                Effect::Gravity(0.25)
            ]
        );
    }
    #[test]
    fn unsupported_and_syntax_errors() {
        let mut e = Engine::default();
        assert!(
            e.run("water(1)", &ctx())
                .unwrap_err()
                .contains("unsupported function")
        );
        assert!(
            e.run("x=1 y=2", &ctx())
                .unwrap_err()
                .contains("expected ';'")
        );
    }
    #[test]
    fn shockscript_utilities_and_first_run_state() {
        let mut e = Engine::default();
        assert_eq!(e.run("if(firstRun(), assign(n,0), 0); setBuffer(0,2,sqr(4)); n=n+getBuffer(0,2); addBuddyVel(atan2(1,0),log(e()))", &ctx()).unwrap(), vec![Effect::Velocity(std::f64::consts::FRAC_PI_2, 1.0)]);
        assert_eq!(
            e.run(
                "if(firstRun(), addBuddyVel(100,100), 0); addBuddyVel(band(1,1), below(1,2))",
                &ctx()
            )
            .unwrap(),
            vec![Effect::Velocity(1.0, 1.0)]
        );
    }
    #[test]
    fn source_precedence_truth_values_and_dynamic_eval() {
        let mut e = Engine::default();
        assert_eq!(
            e.run("addBuddyVel(8%3*2,8%3+1); if(\"a\"==\"b\",addBuddyVel(99,99),0); bor(1,unknown()); evaluate(\"x=4; addBuddyVel(x,sign(0))\")", &ctx()).unwrap(),
            vec![Effect::Velocity(2.0, 3.0), Effect::Velocity(4.0, 0.0)]
        );
        assert_eq!(
            e.run("addBuddyVel(1&&7,0||8); loop(2000,x=x+1)", &ctx())
                .unwrap(),
            vec![Effect::Velocity(7.0, 8.0)]
        );
        assert_eq!(e.vars["x"], Value::Num(2004.0));
        assert_eq!(
            e.run(
                "if(loop(-3,x=x+1), addBuddyVel(1,1), addBuddyVel(0,0)); setBuffer(0,0,7)",
                &ctx()
            )
            .unwrap(),
            vec![Effect::Velocity(1.0, 1.0)]
        );
        assert_eq!(e.vars["x"], Value::Num(2004.0));
    }
    #[test]
    fn host_wired_buddy_and_effect_functions() {
        let mut e = Engine::default();
        let effects = e.run("addBuddyVel(1,2); setBuddyVel(3,4); setBuddyPos(10,20); addBuddyRot(pi()); setBuddyRot(0.5); secret_addEmotion(-2); secret_allItems(); bullet(1,2,3,4,5,6); bulletRot(7,8,10,pi()/2,11,12); playSound('shock',100)", &ctx()).unwrap();
        assert_eq!(
            &effects[..8],
            &[
                Effect::Velocity(1., 2.),
                Effect::SetVelocity(3., 4.),
                Effect::SetPosition(10., 20.),
                Effect::Rotation(std::f64::consts::PI, true),
                Effect::Rotation(0.5, false),
                Effect::Emotion(-2.),
                Effect::UnlockAll,
                Effect::Bullet(1., 2., 3., 4., 5., 6.)
            ]
        );
        assert!(
            matches!(effects[8], Effect::Bullet(7., 8., 10., yv, 11., 12.) if yv.abs() < 1e-12)
        );
        assert_eq!(effects[9], Effect::Sound("shock".into()));
        assert_eq!(
            e.run("addBuddyVel(getEmotion(),getBuddyRot())", &ctx())
                .unwrap(),
            vec![Effect::Velocity(18., 0.25)]
        );
    }
}
