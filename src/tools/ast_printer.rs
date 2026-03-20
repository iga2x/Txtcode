use crate::parser::ast::common::{BinaryOperator, InterpolatedSegment, Literal, UnaryOperator};
/// AST-to-source printer for Txt-code
///
/// Converts a parsed Program back into Txt-code source text.
/// Used by the migration framework to write migrated AST back to disk.
use crate::parser::ast::*;

pub struct AstPrinter {
    indent: usize,
}

impl Default for AstPrinter {
    fn default() -> Self {
        Self::new()
    }
}

impl AstPrinter {
    pub fn new() -> Self {
        Self { indent: 0 }
    }

    /// Print a full program to source text.
    pub fn print_program(&mut self, program: &Program) -> String {
        let mut out = String::new();
        for stmt in &program.statements {
            out.push_str(&self.print_statement(stmt));
            out.push('\n');
        }
        out
    }

    fn ind(&self) -> String {
        "  ".repeat(self.indent)
    }

    pub fn print_statement(&mut self, stmt: &Statement) -> String {
        let ind = self.ind();
        match stmt {
            Statement::Assignment { pattern, value, .. } => {
                format!(
                    "{}store → {} → {}",
                    ind,
                    self.print_pattern(pattern),
                    self.print_expr(value)
                )
            }
            Statement::IndexAssignment {
                target,
                index,
                value,
                ..
            } => {
                format!(
                    "{}store → {}[{}] → {}",
                    ind,
                    self.print_expr(target),
                    self.print_expr(index),
                    self.print_expr(value)
                )
            }
            Statement::CompoundAssignment {
                name, op, value, ..
            } => {
                format!(
                    "{}store → {} → {} {} {}",
                    ind,
                    name,
                    name,
                    self.print_binop(op),
                    self.print_expr(value)
                )
            }
            Statement::FunctionDef {
                name,
                params,
                body,
                return_type,
                ..
            } => {
                let params_str = params
                    .iter()
                    .map(|p| {
                        if let Some(ref t) = p.type_annotation {
                            format!("{}: {:?}", p.name, t)
                        } else {
                            p.name.clone()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let ret = if let Some(ref t) = return_type {
                    format!(" → {:?}", t)
                } else {
                    String::new()
                };
                let mut out = format!("{}define → {} → ({}){}\n", ind, name, params_str, ret);
                self.indent += 1;
                for s in body {
                    out.push_str(&self.print_statement(s));
                    out.push('\n');
                }
                self.indent -= 1;
                out.push_str(&format!("{}end", ind));
                out
            }
            Statement::Return { value, .. } => {
                if let Some(v) = value {
                    format!("{}return → {}", ind, self.print_expr(v))
                } else {
                    format!("{}return", ind)
                }
            }
            Statement::Break { .. } => format!("{}break", ind),
            Statement::Continue { .. } => format!("{}continue", ind),
            Statement::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                ..
            } => {
                let mut out = format!("{}if → {}\n", ind, self.print_expr(condition));
                self.indent += 1;
                for s in then_branch {
                    out.push_str(&self.print_statement(s));
                    out.push('\n');
                }
                self.indent -= 1;
                for (cond, body) in else_if_branches {
                    out.push_str(&format!("{}elseif → {}\n", ind, self.print_expr(cond)));
                    self.indent += 1;
                    for s in body {
                        out.push_str(&self.print_statement(s));
                        out.push('\n');
                    }
                    self.indent -= 1;
                }
                if let Some(els) = else_branch {
                    out.push_str(&format!("{}else\n", ind));
                    self.indent += 1;
                    for s in els {
                        out.push_str(&self.print_statement(s));
                        out.push('\n');
                    }
                    self.indent -= 1;
                }
                out.push_str(&format!("{}end", ind));
                out
            }
            Statement::While {
                condition, body, ..
            } => {
                let mut out = format!("{}while → {}\n", ind, self.print_expr(condition));
                self.indent += 1;
                for s in body {
                    out.push_str(&self.print_statement(s));
                    out.push('\n');
                }
                self.indent -= 1;
                out.push_str(&format!("{}end", ind));
                out
            }
            Statement::DoWhile {
                body, condition, ..
            } => {
                let mut out = format!("{}do\n", ind);
                self.indent += 1;
                for s in body {
                    out.push_str(&self.print_statement(s));
                    out.push('\n');
                }
                self.indent -= 1;
                out.push_str(&format!("{}while → {}\n", ind, self.print_expr(condition)));
                out.push_str(&format!("{}end", ind));
                out
            }
            Statement::For {
                variable,
                iterable,
                body,
                ..
            } => {
                let mut out = format!(
                    "{}for → {} in {}\n",
                    ind,
                    variable,
                    self.print_expr(iterable)
                );
                self.indent += 1;
                for s in body {
                    out.push_str(&self.print_statement(s));
                    out.push('\n');
                }
                self.indent -= 1;
                out.push_str(&format!("{}end", ind));
                out
            }
            Statement::Repeat { count, body, .. } => {
                let mut out = format!("{}repeat → {}\n", ind, self.print_expr(count));
                self.indent += 1;
                for s in body {
                    out.push_str(&self.print_statement(s));
                    out.push('\n');
                }
                self.indent -= 1;
                out.push_str(&format!("{}end", ind));
                out
            }
            Statement::Expression(expr) => {
                format!("{}{}", ind, self.print_expr(expr))
            }
            Statement::Assert {
                condition, message, ..
            } => {
                if let Some(msg) = message {
                    format!(
                        "{}assert → {} → {}",
                        ind,
                        self.print_expr(condition),
                        self.print_expr(msg)
                    )
                } else {
                    format!("{}assert → {}", ind, self.print_expr(condition))
                }
            }
            Statement::Import {
                modules,
                from,
                alias,
                ..
            } => {
                let mut out = String::new();
                if let Some(f) = from {
                    out.push_str(&format!(
                        "{}import → {} from {}",
                        ind,
                        modules.join(", "),
                        f
                    ));
                } else {
                    out.push_str(&format!("{}import → {}", ind, modules.join(", ")));
                }
                if let Some(a) = alias {
                    out.push_str(&format!(" as {}", a));
                }
                out
            }
            Statement::Export { names, .. } => {
                format!("{}export → {}", ind, names.join(", "))
            }
            Statement::Const { name, value, .. } => {
                format!("{}const → {} → {}", ind, name, self.print_expr(value))
            }
            Statement::Enum { name, variants, .. } => {
                let vars = variants
                    .iter()
                    .map(|(v, val)| {
                        if let Some(e) = val {
                            format!("{} = {}", v, self.print_expr(e))
                        } else {
                            v.clone()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}enum → {} → {{{}}}", ind, name, vars)
            }
            Statement::Struct { name, fields, .. } => {
                let fields_str = fields
                    .iter()
                    .map(|(fname, ftype)| format!("{}: {:?}", fname, ftype))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}struct {}({})", ind, name, fields_str)
            }
            Statement::Match {
                value,
                cases,
                default,
                ..
            } => {
                let mut out = format!("{}match → {}\n", ind, self.print_expr(value));
                self.indent += 1;
                for (pat, guard, body) in cases {
                    let pat_str = self.print_pattern(pat);
                    let guard_str = if let Some(g) = guard {
                        format!(" if {}", self.print_expr(g))
                    } else {
                        String::new()
                    };
                    out.push_str(&format!("{}case {}{} →\n", ind, pat_str, guard_str));
                    self.indent += 1;
                    for s in body {
                        out.push_str(&self.print_statement(s));
                        out.push('\n');
                    }
                    self.indent -= 1;
                }
                if let Some(def) = default {
                    out.push_str(&format!("{}default →\n", ind));
                    self.indent += 1;
                    for s in def {
                        out.push_str(&self.print_statement(s));
                        out.push('\n');
                    }
                    self.indent -= 1;
                }
                self.indent -= 1;
                out.push_str(&format!("{}end", ind));
                out
            }
            Statement::Try {
                body,
                catch,
                finally,
                ..
            } => {
                let mut out = format!("{}try\n", ind);
                self.indent += 1;
                for s in body {
                    out.push_str(&self.print_statement(s));
                    out.push('\n');
                }
                self.indent -= 1;
                if let Some((var, catch_body)) = catch {
                    out.push_str(&format!("{}catch {}\n", ind, var));
                    self.indent += 1;
                    for s in catch_body {
                        out.push_str(&self.print_statement(s));
                        out.push('\n');
                    }
                    self.indent -= 1;
                }
                if let Some(fin) = finally {
                    out.push_str(&format!("{}finally\n", ind));
                    self.indent += 1;
                    for s in fin {
                        out.push_str(&self.print_statement(s));
                        out.push('\n');
                    }
                    self.indent -= 1;
                }
                out.push_str(&format!("{}end", ind));
                out
            }
            Statement::Permission {
                action,
                resource,
                scope,
                ..
            } => {
                if let Some(s) = scope {
                    format!("{}permission → {} → {} {}", ind, action, resource, s)
                } else {
                    format!("{}permission → {} → {}", ind, action, resource)
                }
            }
            Statement::TypeAlias { name, target, .. } => {
                format!("{}type → {} → {}", ind, name, target)
            }
            Statement::NamedError { name, message, .. } => {
                format!("{}error → {} → {}", ind, name, self.print_expr(message))
            }
        }
    }

    pub fn print_expr(&self, expr: &Expression) -> String {
        match expr {
            Expression::Literal(lit) => self.print_literal(lit),
            Expression::Identifier(name) => name.clone(),
            Expression::BinaryOp {
                left, op, right, ..
            } => {
                format!(
                    "{} {} {}",
                    self.print_expr(left),
                    self.print_binop(op),
                    self.print_expr(right)
                )
            }
            Expression::UnaryOp { op, operand, .. } => {
                let op_str = match op {
                    UnaryOperator::Minus => "-",
                    UnaryOperator::Not => "not ",
                    UnaryOperator::BitNot => "~",
                    UnaryOperator::Increment => "++",
                    UnaryOperator::Decrement => "--",
                };
                format!("{}{}", op_str, self.print_expr(operand))
            }
            Expression::FunctionCall {
                name, arguments, ..
            } => {
                let args = arguments
                    .iter()
                    .map(|a| self.print_expr(a))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({})", name, args)
            }
            Expression::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                let args = arguments
                    .iter()
                    .map(|a| self.print_expr(a))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}.{}({})", self.print_expr(object), method, args)
            }
            Expression::Array { elements, .. } => {
                let elems = elements
                    .iter()
                    .map(|e| self.print_expr(e))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", elems)
            }
            Expression::Map { entries, .. } => {
                let pairs_str = entries
                    .iter()
                    .map(|(k, v)| format!("{}: {}", self.print_expr(k), self.print_expr(v)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{}}}", pairs_str)
            }
            Expression::Set { elements, .. } => {
                let elems = elements
                    .iter()
                    .map(|e| self.print_expr(e))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{}}}", elems)
            }
            Expression::Index { target, index, .. } => {
                format!("{}[{}]", self.print_expr(target), self.print_expr(index))
            }
            Expression::Member { target, name, .. } => {
                format!("{}.{}", self.print_expr(target), name)
            }
            Expression::OptionalMember { target, name, .. } => {
                format!("{}?.{}", self.print_expr(target), name)
            }
            Expression::OptionalCall {
                target, arguments, ..
            } => {
                let args = arguments
                    .iter()
                    .map(|a| self.print_expr(a))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}?.({})", self.print_expr(target), args)
            }
            Expression::OptionalIndex { target, index, .. } => {
                format!("{}?.[{}]", self.print_expr(target), self.print_expr(index))
            }
            Expression::Slice {
                target, start, end, ..
            } => {
                let s = start
                    .as_ref()
                    .map(|e| self.print_expr(e))
                    .unwrap_or_default();
                let e = end.as_ref().map(|e| self.print_expr(e)).unwrap_or_default();
                format!("{}[{}:{}]", self.print_expr(target), s, e)
            }
            Expression::Lambda { params, body, .. } => {
                let params_str = params
                    .iter()
                    .map(|p| p.name.clone())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({}) → {}", params_str, self.print_expr(body))
            }
            Expression::Ternary {
                condition,
                true_expr,
                false_expr,
                ..
            } => {
                format!(
                    "{} ? {} : {}",
                    self.print_expr(condition),
                    self.print_expr(true_expr),
                    self.print_expr(false_expr)
                )
            }
            Expression::Await { expression, .. } => {
                format!("await {}", self.print_expr(expression))
            }
            Expression::InterpolatedString { segments, .. } => {
                let mut s = String::from("f\"");
                for seg in segments {
                    match seg {
                        InterpolatedSegment::Text(t) => s.push_str(&t.replace('"', "\\\"")),
                        InterpolatedSegment::Expression(e) => {
                            s.push('{');
                            s.push_str(&self.print_expr(e));
                            s.push('}');
                        }
                    }
                }
                s.push('"');
                s
            }
            Expression::StructLiteral { name, fields, .. } => {
                let fields_str = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, self.print_expr(v)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{} {{ {} }}", name, fields_str)
            }
            Expression::Spread { value, .. } => {
                format!("...{}", self.print_expr(value))
            }
            Expression::Propagate { value, .. } => {
                format!("{}?", self.print_expr(value))
            }
        }
    }

    fn print_literal(&self, lit: &Literal) -> String {
        match lit {
            Literal::Integer(n) => n.to_string(),
            Literal::Float(f) => format!("{}", f),
            Literal::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
            Literal::Boolean(b) => b.to_string(),
            Literal::Null => "null".to_string(),
            Literal::Char(c) => format!("'{}'", c),
        }
    }

    fn print_binop(&self, op: &BinaryOperator) -> &'static str {
        match op {
            BinaryOperator::Add => "+",
            BinaryOperator::Subtract => "-",
            BinaryOperator::Multiply => "*",
            BinaryOperator::Divide => "/",
            BinaryOperator::Modulo => "%",
            BinaryOperator::Power => "**",
            BinaryOperator::Equal => "==",
            BinaryOperator::NotEqual => "!=",
            BinaryOperator::Less => "<",
            BinaryOperator::LessEqual => "<=",
            BinaryOperator::Greater => ">",
            BinaryOperator::GreaterEqual => ">=",
            BinaryOperator::And => "and",
            BinaryOperator::Or => "or",
            BinaryOperator::BitwiseAnd => "&",
            BinaryOperator::BitwiseOr => "|",
            BinaryOperator::BitwiseXor => "^",
            BinaryOperator::LeftShift => "<<",
            BinaryOperator::RightShift => ">>",
            BinaryOperator::NullCoalesce => "??",
            BinaryOperator::Pipe => "|>",
        }
    }

    fn print_pattern(&self, pattern: &Pattern) -> String {
        match pattern {
            Pattern::Identifier(name) => name.clone(),
            Pattern::Ignore => "_".to_string(),
            Pattern::Array(parts) => {
                let p = parts
                    .iter()
                    .map(|p| self.print_pattern(p))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{}]", p)
            }
            Pattern::Struct { fields, rest } => {
                let mut parts: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, self.print_pattern(v)))
                    .collect();
                if let Some(r) = rest {
                    parts.push(format!("...{}", r));
                }
                format!("{{{}}}", parts.join(", "))
            }
            Pattern::Constructor { type_name, args } => {
                let a = args
                    .iter()
                    .map(|p| self.print_pattern(p))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({})", type_name, a)
            }
            Pattern::Or(pats) => pats.iter().map(|p| self.print_pattern(p)).collect::<Vec<_>>().join(" | "),
            Pattern::Range(start, end) => format!("{:?}..={:?}", start, end),
        }
    }
}

/// Detect version string from source file header comment.
/// Looks for `# version: X.Y.Z` in the first 20 lines.
pub fn detect_version_from_source(source: &str) -> Option<String> {
    for line in source.lines().take(20) {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("# version:") {
            let ver = rest.trim();
            if !ver.is_empty() {
                return Some(ver.to_string());
            }
        }
        if let Some(rest) = trimmed.strip_prefix("#version:") {
            let ver = rest.trim();
            if !ver.is_empty() {
                return Some(ver.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::common::Span;

    #[test]
    fn test_print_assignment() {
        let mut printer = AstPrinter::new();
        let stmt = Statement::Assignment {
            pattern: Pattern::Identifier("x".to_string()),
            type_annotation: None,
            value: Expression::Literal(Literal::Integer(42)),
            span: Span::default(),
        };
        let out = printer.print_statement(&stmt);
        assert!(out.contains("store"), "expected 'store' in: {}", out);
        assert!(out.contains("x"), "expected 'x' in: {}", out);
        assert!(out.contains("42"), "expected '42' in: {}", out);
    }

    #[test]
    fn test_print_return() {
        let mut printer = AstPrinter::new();
        let stmt = Statement::Return {
            value: Some(Expression::Literal(Literal::Integer(1))),
            span: Span::default(),
        };
        assert_eq!(printer.print_statement(&stmt), "return → 1");
    }

    #[test]
    fn test_detect_version_from_source() {
        let src = "# version: 0.1.0\nstore → x → 1";
        assert_eq!(detect_version_from_source(src), Some("0.1.0".to_string()));
    }

    #[test]
    fn test_detect_version_missing() {
        let src = "store → x → 1";
        assert_eq!(detect_version_from_source(src), None);
    }

    #[test]
    fn test_print_if_statement() {
        let mut printer = AstPrinter::new();
        let stmt = Statement::If {
            condition: Expression::Literal(Literal::Boolean(true)),
            then_branch: vec![Statement::Return {
                value: Some(Expression::Literal(Literal::Integer(1))),
                span: Span::default(),
            }],
            else_if_branches: vec![],
            else_branch: None,
            span: Span::default(),
        };
        let out = printer.print_statement(&stmt);
        assert!(
            out.contains("if → true"),
            "expected 'if → true' in: {}",
            out
        );
        assert!(
            out.contains("return → 1"),
            "expected 'return → 1' in: {}",
            out
        );
        assert!(out.contains("end"), "expected 'end' in: {}", out);
    }

    #[test]
    fn test_print_binary_op() {
        let printer = AstPrinter::new();
        let expr = Expression::BinaryOp {
            left: Box::new(Expression::Literal(Literal::Integer(1))),
            op: BinaryOperator::Add,
            right: Box::new(Expression::Literal(Literal::Integer(2))),
            span: Span::default(),
        };
        assert_eq!(printer.print_expr(&expr), "1 + 2");
    }
}
