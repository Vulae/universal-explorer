use std::collections::HashMap;

use util_python::{
    pickle::{pickle_extract_class, Class, Value},
    pickle_extract_path,
};

use crate::indent;

use super::{inline_error_str, parse_node, RenPyCodeParserError};

pub fn parse_node_renpy_ast(class: &Class) -> Result<String, RenPyCodeParserError> {
    Ok(match class.module.name.as_str() {
        "Return" => String::new(),
        "Init" => {
            let block = pickle_extract_class::<Vec<Value>>(
                class,
                &pickle_extract_path!(class_state tuple[1] key["block"]),
            )?;
            block
                .iter()
                .map(|v| inline_error_str(parse_node(v)))
                .collect::<Vec<_>>()
                .join("\n")
        }
        "Define" => format!(
            "define {}.{} {} {}\n",
            pickle_extract_class::<String>(
                class,
                &pickle_extract_path!(class_state tuple[1] key["store"])
            )?,
            pickle_extract_class::<String>(
                class,
                &pickle_extract_path!(class_state tuple[1] key["varname"])
            )?,
            pickle_extract_class::<String>(
                class,
                &pickle_extract_path!(class_state tuple[1] key["operator"])
            )?,
            inline_error_str(parse_node(&pickle_extract_class::<Value>(
                class,
                &pickle_extract_path!(class_state tuple[1] key["code"])
            )?)),
        ),
        "Python" => {
            let code = parse_node(&pickle_extract_class::<Value>(
                class,
                &pickle_extract_path!(class_state tuple[1] key["code"]),
            )?)?;
            if code.split('\n').count() == 1 {
                format!("$ {}\n", code.replace("\n", ""))
            } else {
                format!("init python:\n{}\n", indent(code))
            }
        }
        "PyCode" => {
            match pickle_extract_class::<Value>(class, &pickle_extract_path!(class_state tuple[1]))?
            {
                Value::String(string) => string.trim()
                // TODO: WTFFFFFFFFFFFF, Don't do this.
                // There's a bunch of random whitespace.
                .replace("\n\n", "\n")
                .replace("\n\n", "\n")
                .replace("\n\n", "\n")
                .replace("\n\n", "\n")
                .replace("\n\n", "\n")
                .replace("\n\n", "\n")
                .replace("\n\n", "\n")
                .replace("\n\n", "\n")
                .replace("\n\n", "\n")
                .replace("\n\n", "\n")
                .to_owned(),
                node @ Value::Class(_) => inline_error_str(parse_node(&node)),
                _ => {
                    return Err(RenPyCodeParserError::Expected(
                        "renpy.ast/PyCode expected code to be string or node",
                    ))
                }
            }
        }
        "PyExpr" => {
            pickle_extract_class::<String>(class, &pickle_extract_path!(class_arg tuple[0]))?
        }
        "Default" => format!(
            "default {}.{} = {}\n",
            pickle_extract_class::<String>(
                class,
                &pickle_extract_path!(class_state tuple[1] key["store"])
            )?,
            pickle_extract_class::<String>(
                class,
                &pickle_extract_path!(class_state tuple[1] key["varname"])
            )?,
            inline_error_str(parse_node(&pickle_extract_class::<Value>(
                class,
                &pickle_extract_path!(class_state tuple[1] key["code"])
            )?)),
        ),
        "Label" => format!(
            "label {}:\n{}\n",
            pickle_extract_class::<String>(
                class,
                &pickle_extract_path!(class_state tuple[1] key["name"])
            )?,
            indent(
                pickle_extract_class::<Vec<Value>>(
                    class,
                    &pickle_extract_path!(class_state tuple[1] key["block"]),
                )?
                .iter()
                .map(|v| inline_error_str(parse_node(v)))
                .collect::<Vec<_>>()
                .join("\n")
            )
        ),
        "Style" => {
            let ident = pickle_extract_class::<String>(
                class,
                &pickle_extract_path!(class_state tuple[1] key["style_name"]),
            )?;
            let props = pickle_extract_class::<HashMap<String, Value>>(
                class,
                &pickle_extract_path!(class_state tuple[1] key["properties"]),
            )?;
            format!(
                "style {}{}\n",
                if let Some(parent) = pickle_extract_class::<Option<String>>(
                    class,
                    &pickle_extract_path!(class_state tuple[1] key["parent"])
                )? {
                    format!("{ident} is {parent}")
                } else {
                    ident
                },
                if props.is_empty() {
                    "".to_owned()
                } else {
                    format!(
                        ":\n{}",
                        indent(
                            pickle_extract_class::<HashMap<String, Value>>(
                                class,
                                &pickle_extract_path!(class_state tuple[1] key["properties"]),
                            )?
                            .iter()
                            .map(|(key, value)| {
                                let str = inline_error_str(parse_node(value));
                                if str.lines().count() == 1 {
                                    format!("{key} {str}\n")
                                } else {
                                    format!("{key}:\n{}", indent(str))
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\n")
                        ),
                    )
                },
            )
        }
        "Transform" => format!(
            "transform {}:\n{}\n",
            pickle_extract_class::<String>(
                class,
                &pickle_extract_path!(class_state tuple[1] key["varname"]),
            )?,
            indent(inline_error_str(parse_node(&pickle_extract_class(
                class,
                &pickle_extract_path!(class_state tuple[1] key["atl"]),
            )?))),
        ),
        "Screen" => inline_error_str(parse_node(&pickle_extract_class(
            class,
            &pickle_extract_path!(class_state tuple[1] key["screen"]),
        )?)),
        // TODO: This node seems to have original data which we are currently using.
        // But also has parsed data??? in the properties "parsed", probably would be better to use
        // the parsed data, bcuz IDK if the raw data is staying.
        "UserStatement" => pickle_extract_class::<String>(
            class,
            &pickle_extract_path!(class_state tuple[1] key["line"]),
        )?,
        "With" => format!(
            "with {}\n",
            match pickle_extract_class::<Value>(
                class,
                &pickle_extract_path!(class_state tuple[1] key["expr"]),
            )? {
                Value::String(string) if string == "None" => return Ok(String::new()),
                Value::String(string) => string,
                value => inline_error_str(parse_node(&value)),
            }
        ),
        "Scene" => format!(
            "scene {}\n",
            pickle_extract_class::<String>(
                class,
                &pickle_extract_path!(class_state tuple[1] key["imspec"] tuple[0] tuple[0]),
            )?,
        ),
        "Say" => {
            let what: String = pickle_extract_class(
                class,
                &pickle_extract_path!(class_state tuple[1] key["what"]),
            )?;
            if let Some(who) = pickle_extract_class::<Option<String>>(
                class,
                &pickle_extract_path!(class_state tuple[1] key["who"]),
            )? {
                format!("{who} \"{what}\"\n")
            } else {
                format!("\"{what}\"\n")
            }
        }
        "Show" => format!(
            "show {}\n",
            pickle_extract_class::<String>(
                class,
                &pickle_extract_path!(class_state tuple[1] key["imspec"] tuple[0] tuple[0]),
            )?,
        ),
        "Jump" => format!(
            "jump {}\n",
            pickle_extract_class::<String>(
                class,
                &pickle_extract_path!(class_state tuple[1] key["target"]),
            )?,
        ),
        "Hide" => format!(
            "hide {}\n",
            pickle_extract_class::<String>(
                class,
                &pickle_extract_path!(class_state tuple[1] key["imspec"] tuple[0] tuple[0]),
            )?,
        ),
        "If" => {
            let entries = pickle_extract_class::<Vec<(Value, Vec<Value>)>>(
                class,
                &pickle_extract_path!(class_state tuple[1] key["entries"]),
            )?;

            let mut str = String::new();

            for (i, (condition, body)) in entries.iter().enumerate() {
                let body_str = body
                    .iter()
                    .map(|node| inline_error_str(parse_node(node)))
                    .collect::<Vec<_>>()
                    .join("\n");
                if i == 0 {
                    str += &format!(
                        "if {}:\n{}\n",
                        inline_error_str(parse_node(condition)),
                        indent(body_str),
                    );
                } else if i < (entries.len() - 1)
                    || (!matches!(condition, Value::None)
                        && !matches!(condition, Value::String(str) if str == "True"))
                {
                    str += &format!(
                        "elif {}:\n{}\n",
                        inline_error_str(parse_node(condition)),
                        indent(body_str),
                    );
                } else {
                    str += &format!("else:\n{}\n", indent(body_str));
                }
            }

            str
        }
        "Menu" => format!(
            "menu:\n{}",
            indent(
                pickle_extract_class::<Vec<(Value, Value, Vec<Value>)>>(
                    class,
                    &pickle_extract_path!(class_state tuple[1] key["items"]),
                )?
                .into_iter()
                .map(|(name, condition, body)| {
                    let name = match name {
                        Value::String(str) => str,
                        node => inline_error_str(parse_node(&node)),
                    };
                    let condition = match condition {
                        Value::None => None,
                        Value::String(string) if string == "True" => None,
                        Value::String(string) => Some(string),
                        node => Some(inline_error_str(parse_node(&node))),
                    };
                    let body = body
                        .iter()
                        .map(|node| inline_error_str(parse_node(node)))
                        .collect::<Vec<_>>()
                        .join("\n");
                    if let Some(condition) = condition {
                        format!("\"{name}\" if {condition}:\n{}\n", indent(body))
                    } else {
                        format!("\"{name}\":\n{}\n", indent(body))
                    }
                })
                .collect::<Vec<_>>()
                .join("\n"),
            ),
        ),
        "While" => format!(
            "while {}:\n{}\n",
            inline_error_str(parse_node(&pickle_extract_class(
                class,
                &pickle_extract_path!(class_state tuple[1] key["condition"]),
            )?)),
            indent(
                pickle_extract_class::<Vec<Value>>(
                    class,
                    &pickle_extract_path!(class_state tuple[1] key["block"]),
                )?
                .iter()
                .map(|node| inline_error_str(parse_node(node)))
                .collect::<Vec<_>>()
                .join("\n"),
            ),
        ),
        _ => {
            log::warn!("Unimplemented parser {class:#?}");
            return Err(RenPyCodeParserError::UnimplementedModuleName(
                class.module.module.to_owned(),
                class.module.name.to_owned(),
            ));
        }
    })
}
