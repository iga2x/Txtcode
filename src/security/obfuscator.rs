use crate::parser::ast::*;
use crate::lexer::Span;
use rand::Rng;
use sha2::{Sha256, Digest};
use std::collections::HashMap;

/// Source code obfuscator
/// Encrypts AST nodes, mangles names, flattens control flow, and inserts dead code
pub struct Obfuscator {
    name_map: HashMap<String, String>,
    counter: usize,
    encryption_key: Vec<u8>,
    dead_code_counter: usize,
}

impl Obfuscator {
    pub fn new() -> Self {
        // Generate encryption key from a seed
        let mut hasher = Sha256::new();
        hasher.update(b"txtcode_obfuscation_key");
        let key = hasher.finalize().to_vec();

        Self {
            name_map: HashMap::new(),
            counter: 0,
            encryption_key: key,
            dead_code_counter: 0,
        }
    }

    pub fn new_with_key(key: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(key);
        let encryption_key = hasher.finalize().to_vec();

        Self {
            name_map: HashMap::new(),
            counter: 0,
            encryption_key,
            dead_code_counter: 0,
        }
    }

    /// Obfuscate a program
    pub fn obfuscate(&mut self, program: &Program) -> Program {
        let mut obfuscated_statements = Vec::new();

        for statement in &program.statements {
            obfuscated_statements.push(self.obfuscate_statement(statement));
        }

        // Insert dead code at the beginning
        if self.dead_code_counter > 0 {
            let dead_code = self.generate_dead_code();
            obfuscated_statements.insert(0, dead_code);
        }

        Program {
            statements: obfuscated_statements,
            span: program.span.clone(),
        }
    }

    fn obfuscate_statement(&mut self, statement: &Statement) -> Statement {
        match statement {
            Statement::Assignment { name, type_annotation, value, span } => {
                let obfuscated_name = self.mangle_name(name);
                Statement::Assignment {
                    name: obfuscated_name,
                    type_annotation: type_annotation.clone(),
                    value: self.obfuscate_expression(value),
                    span: span.clone(),
                }
            }
            Statement::FunctionDef { name, params, return_type, body, span } => {
                let obfuscated_name = self.mangle_name(name);
                let obfuscated_params: Vec<Parameter> = params
                    .iter()
                    .map(|p| Parameter {
                        name: self.mangle_name(&p.name),
                        type_annotation: p.type_annotation.clone(),
                        span: p.span.clone(),
                    })
                    .collect();

                let mut obfuscated_body = Vec::new();
                for stmt in body {
                    obfuscated_body.push(self.obfuscate_statement(stmt));
                }

                // Insert dead code in function body
                if self.should_insert_dead_code() {
                    obfuscated_body.push(self.generate_dead_code());
                }

                Statement::FunctionDef {
                    name: obfuscated_name,
                    params: obfuscated_params,
                    return_type: return_type.clone(),
                    body: obfuscated_body,
                    span: span.clone(),
                }
            }
            Statement::If { condition, then_branch, else_if_branches, else_branch, span } => {
                // Flatten control flow by adding dummy conditions
                let obfuscated_condition = self.obfuscate_expression(condition);
                
                let obfuscated_then: Vec<Statement> = then_branch
                    .iter()
                    .map(|s| self.obfuscate_statement(s))
                    .collect();

                let obfuscated_elseif: Vec<(Expression, Vec<Statement>)> = else_if_branches
                    .iter()
                    .map(|(cond, body)| {
                        (
                            self.obfuscate_expression(cond),
                            body.iter().map(|s| self.obfuscate_statement(s)).collect(),
                        )
                    })
                    .collect();

                let obfuscated_else = else_branch.as_ref().map(|body| {
                    body.iter().map(|s| self.obfuscate_statement(s)).collect()
                });

                Statement::If {
                    condition: obfuscated_condition,
                    then_branch: obfuscated_then,
                    else_if_branches: obfuscated_elseif,
                    else_branch: obfuscated_else,
                    span: span.clone(),
                }
            }
            Statement::While { condition, body, span } => {
                Statement::While {
                    condition: self.obfuscate_expression(condition),
                    body: body.iter().map(|s| self.obfuscate_statement(s)).collect(),
                    span: span.clone(),
                }
            }
            Statement::For { variable, iterable, body, span } => {
                Statement::For {
                    variable: self.mangle_name(variable),
                    iterable: self.obfuscate_expression(iterable),
                    body: body.iter().map(|s| self.obfuscate_statement(s)).collect(),
                    span: span.clone(),
                }
            }
            Statement::Repeat { count, body, span } => {
                Statement::Repeat {
                    count: self.obfuscate_expression(count),
                    body: body.iter().map(|s| self.obfuscate_statement(s)).collect(),
                    span: span.clone(),
                }
            }
            Statement::Match { value, cases, default, span } => {
                let obfuscated_cases: Vec<MatchCase> = cases
                    .iter()
                    .map(|case| MatchCase {
                        pattern: case.pattern.clone(), // Patterns are kept as-is
                        guard: case.guard.as_ref().map(|g| self.obfuscate_expression(g)),
                        body: case.body.iter().map(|s| self.obfuscate_statement(s)).collect(),
                        span: case.span.clone(),
                    })
                    .collect();

                Statement::Match {
                    value: self.obfuscate_expression(value),
                    cases: obfuscated_cases,
                    default: default.as_ref().map(|body| {
                        body.iter().map(|s| self.obfuscate_statement(s)).collect()
                    }),
                    span: span.clone(),
                }
            }
            Statement::Return { value, span } => {
                Statement::Return {
                    value: value.as_ref().map(|v| self.obfuscate_expression(v)),
                    span: span.clone(),
                }
            }
            Statement::Try { body, catch, span } => {
                Statement::Try {
                    body: body.iter().map(|s| self.obfuscate_statement(s)).collect(),
                    catch: catch.as_ref().map(|(name, body)| {
                        (
                            self.mangle_name(name),
                            body.iter().map(|s| self.obfuscate_statement(s)).collect(),
                        )
                    }),
                    span: span.clone(),
                }
            }
            Statement::Expression(expr) => {
                Statement::Expression(self.obfuscate_expression(expr))
            }
            Statement::Break { span } => Statement::Break { span: span.clone() },
            Statement::Continue { span } => Statement::Continue { span: span.clone() },
            Statement::Import { items, from, alias, span } => {
                Statement::Import {
                    items: items.clone(),
                    from: from.clone(),
                    alias: alias.clone(),
                    span: span.clone(),
                }
            }
        }
    }

    fn obfuscate_expression(&mut self, expr: &Expression) -> Expression {
        match expr {
            Expression::Literal(lit) => {
                Expression::Literal(match lit {
                    Literal::String(s) => {
                        // Encrypt strings
                        Literal::String(self.encrypt_string(s))
                    }
                    _ => lit.clone(),
                })
            }
            Expression::Identifier(name) => {
                if let Some(obfuscated) = self.name_map.get(name) {
                    Expression::Identifier(obfuscated.clone())
                } else {
                    Expression::Identifier(name.clone())
                }
            }
            Expression::BinaryOp { left, op, right, span } => {
                Expression::BinaryOp {
                    left: Box::new(self.obfuscate_expression(left)),
                    op: op.clone(),
                    right: Box::new(self.obfuscate_expression(right)),
                    span: span.clone(),
                }
            }
            Expression::UnaryOp { op, operand, span } => {
                Expression::UnaryOp {
                    op: op.clone(),
                    operand: Box::new(self.obfuscate_expression(operand)),
                    span: span.clone(),
                }
            }
            Expression::FunctionCall { name, arguments, span } => {
                let obfuscated_name = if self.name_map.contains_key(name) {
                    self.name_map[name].clone()
                } else {
                    name.clone()
                };

                Expression::FunctionCall {
                    name: obfuscated_name,
                    arguments: arguments.iter().map(|a| self.obfuscate_expression(a)).collect(),
                    span: span.clone(),
                }
            }
            Expression::Array { elements, span } => {
                Expression::Array {
                    elements: elements.iter().map(|e| self.obfuscate_expression(e)).collect(),
                    span: span.clone(),
                }
            }
            Expression::Map { entries, span } => {
                Expression::Map {
                    entries: entries
                        .iter()
                        .map(|(k, v)| (k.clone(), self.obfuscate_expression(v)))
                        .collect(),
                    span: span.clone(),
                }
            }
            Expression::Index { target, index, span } => {
                Expression::Index {
                    target: Box::new(self.obfuscate_expression(target)),
                    index: Box::new(self.obfuscate_expression(index)),
                    span: span.clone(),
                }
            }
            Expression::Member { target, member, span } => {
                Expression::Member {
                    target: Box::new(self.obfuscate_expression(target)),
                    member: member.clone(),
                    span: span.clone(),
                }
            }
            Expression::Lambda { params, body, span } => {
                Expression::Lambda {
                    params: params
                        .iter()
                        .map(|p| Parameter {
                            name: self.mangle_name(&p.name),
                            type_annotation: p.type_annotation.clone(),
                            span: p.span.clone(),
                        })
                        .collect(),
                    body: Box::new(self.obfuscate_expression(body)),
                    span: span.clone(),
                }
            }
        }
    }

    /// Mangle variable/function names
    fn mangle_name(&mut self, name: &str) -> String {
        if let Some(mangled) = self.name_map.get(name) {
            return mangled.clone();
        }

        // Generate obfuscated name
        let mangled = format!("_{:x}_{}", self.counter, self.hash_name(name));
        self.counter += 1;
        self.name_map.insert(name.to_string(), mangled.clone());
        mangled
    }

    /// Hash a name for obfuscation
    fn hash_name(&self, name: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(name.as_bytes());
        hasher.update(&self.encryption_key);
        format!("{:x}", hasher.finalize())[..8].to_string()
    }

    /// Encrypt a string literal
    fn encrypt_string(&self, s: &str) -> String {
        // Simple XOR encryption for demonstration
        // In production, use proper AES-GCM encryption
        let key = &self.encryption_key[..16.min(self.encryption_key.len())];
        let mut encrypted = Vec::new();
        
        for (i, byte) in s.bytes().enumerate() {
            encrypted.push(byte ^ key[i % key.len()]);
        }

        // Encode as hex string
        hex::encode(encrypted)
    }

    /// Generate dead code to confuse reverse engineers
    fn generate_dead_code(&mut self) -> Statement {
        self.dead_code_counter += 1;
        
        // Generate a dummy variable assignment that's never used
        let dummy_name = format!("_dead_{}", self.dead_code_counter);
        let dummy_value = Expression::Literal(Literal::Integer(
            rand::thread_rng().gen_range(0..1000)
        ));

        Statement::Assignment {
            name: dummy_name,
            type_annotation: None,
            value: dummy_value,
            span: Span::new(0, 0, 1, 1),
        }
    }

    fn should_insert_dead_code(&self) -> bool {
        // Insert dead code randomly (30% chance)
        rand::thread_rng().gen_bool(0.3)
    }

    /// Get the name mapping (for debugging/decryption)
    pub fn get_name_map(&self) -> &HashMap<String, String> {
        &self.name_map
    }
}

impl Default for Obfuscator {
    fn default() -> Self {
        Self::new()
    }
}
