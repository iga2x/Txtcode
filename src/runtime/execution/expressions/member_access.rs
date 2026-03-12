// Member access evaluation (struct fields, map keys, enum variants)

use super::ExpressionVM;
use crate::parser::ast::Expression;
use crate::runtime::core::Value;
use crate::runtime::errors::RuntimeError;

pub fn evaluate_member<VM: ExpressionVM>(
    vm: &mut VM,
    target: &Expression,
    name: &str,
) -> Result<Value, RuntimeError> {
    // Special handling for enum type access (e.g., Color.Red)
    if let Expression::Identifier(enum_type_name) = target {
        if let Some(variants) = vm.enum_defs().get(enum_type_name) {
            for (variant_name, _variant_value) in variants {
                if variant_name == name {
                    crate::tools::logger::log_debug(&format!(
                        "Accessing enum variant {}.{}",
                        enum_type_name, name
                    ));
                    return Ok(Value::Enum(enum_type_name.clone(), name.to_string()));
                }
            }
            return Err(RuntimeError::new(format!(
                "Enum '{}' has no variant '{}'",
                enum_type_name, name
            )));
        }
    }

    // Evaluate target expression
    let obj = super::ExpressionEvaluator::evaluate(vm, target)?;
    match obj {
        Value::Map(map) => map
            .get(name)
            .cloned()
            .ok_or_else(|| RuntimeError::new(format!("Member not found: {}", name))),
        Value::Struct(_struct_name, fields) => fields
            .get(name)
            .cloned()
            .ok_or_else(|| RuntimeError::new(format!("Struct field '{}' not found", name))),
        Value::String(s) => match name {
            "length" | "len" => Ok(Value::Integer(s.chars().count() as i64)),
            "isEmpty" => Ok(Value::Boolean(s.is_empty())),
            _ => Err(RuntimeError::new(format!(
                "String has no property '{}'. Use {}.{}() as a method call.",
                name, "str", name
            ))),
        },
        Value::Array(arr) => match name {
            "length" | "len" => Ok(Value::Integer(arr.len() as i64)),
            "isEmpty" => Ok(Value::Boolean(arr.is_empty())),
            _ => Err(RuntimeError::new(format!(
                "Array has no property '{}'. Use arr.{}() as a method call.",
                name, name
            ))),
        },
        Value::Set(set) => match name {
            "length" | "len" | "size" => Ok(Value::Integer(set.len() as i64)),
            "isEmpty" => Ok(Value::Boolean(set.is_empty())),
            _ => Err(RuntimeError::new(format!(
                "Set has no property '{}'.",
                name
            ))),
        },
        Value::Enum(enum_name, _variant) => Err(RuntimeError::new(format!(
            "Cannot access members of enum value {}.{}",
            enum_name, _variant
        ))),
        _ => Err(RuntimeError::new(format!(
            "Member access only works on maps, structs, or enums, got {:?}",
            obj
        ))),
    }
}
