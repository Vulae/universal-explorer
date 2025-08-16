use thiserror::Error;
use util_python::{
    pickle::{pickle_extract, PickleError, Value},
    pickle_extract_path,
};

mod renpy_ast;

#[derive(Debug, Error)]
pub enum RenPyCodeParserError {
    #[error("Expected: {0}")]
    Expected(&'static str),
    #[error("Unimplemented module: {0}")]
    UnimplementedModule(String),
    #[error("Unimplemented module name: {0} {1}")]
    UnimplementedModuleName(String, String),
    #[error("Unimplemented: {0}")]
    Unimplemented(&'static str),
    #[error("Unknown script version: {0:?}")]
    BadScriptVersion(i128),
    #[error("Bad script key: {0:?}")]
    BadScriptKey(Option<String>),
    #[error(transparent)]
    PickleError(#[from] PickleError),
}

fn parse_node(node: &Value) -> Result<String, RenPyCodeParserError> {
    let Value::Class(class) = node else {
        return Err(RenPyCodeParserError::Expected("node root to be a class"));
    };
    Ok(match class.module.module.as_str() {
        "renpy.ast" => renpy_ast::parse_node_renpy_ast(class)?,
        _ => {
            return Err(RenPyCodeParserError::UnimplementedModule(
                class.module.module.to_owned(),
            ))
        }
    })
}

pub(crate) fn inline_error_str(result: Result<String, RenPyCodeParserError>) -> String {
    match result {
        Ok(str) => str,
        Err(err) => format!("***DECOMPILE ERROR: {err} ***\n"),
    }
}

pub(crate) fn indent<S: AsRef<str>>(str: S) -> String {
    str.as_ref()
        .split('\n')
        .map(|line| format!("    {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn renpy_parse_code(pickle: Value) -> Result<String, RenPyCodeParserError> {
    let header: Value = pickle_extract(&pickle, &pickle_extract_path!(tuple[0]))?;

    let version: i128 = pickle_extract(&header, &pickle_extract_path!(key["version"]))?;
    let key: Option<String> = pickle_extract(&header, &pickle_extract_path!(key["key"]))?;

    if version != 5003000 {
        return Err(RenPyCodeParserError::BadScriptVersion(version));
    }
    if key != Some("unlocked".to_owned()) {
        return Err(RenPyCodeParserError::BadScriptKey(key));
    }

    let ast: Vec<Value> = pickle_extract(&pickle, &pickle_extract_path!(tuple[1]))?;

    let mut str = String::new();

    ast.into_iter().try_for_each(|node| {
        str += &inline_error_str(parse_node(&node));
        Ok::<_, RenPyCodeParserError>(())
    })?;

    // TODO: Make node parsing consistent so we don't have to filter out empty lines.
    let str = str
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    Ok(format!(
        "# Decompiled Ren'Py script.\n# Decompilation is not accurate to original code.\n{}\n\n{str}",
        format!("HEADER: {header:#?}")
            .split('\n')
            .map(|v| format!("# {v}"))
            .collect::<Vec<_>>().join("\n"),
    ))
}
