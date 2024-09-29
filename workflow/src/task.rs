use anyhow::Result;

use intern::InternStr;
use syntax::ast;
use util::IdVec;

use crate::{AbstractValueId, Error, IdentId, LiteralId, ModuleId, Value, WorkflowStrings};

const DEFAULT_VARS_LEN: usize = 8;

/// Utility representing a task's inputs, outputs, and params.
/// We'll use this with several different types throughout the process.
#[derive(Debug, Default, Clone)]
pub struct TaskVars<T> {
    pub inputs: Vec<T>,
    pub outputs: Vec<T>,
    pub params: Vec<T>,
}

impl<T> TaskVars<T> {
    /// Create a new `TaskVars`, potentially with a different type,
    /// with the same sizes as `other`.
    pub fn new_with_sizes<U>(other: &TaskVars<U>) -> Self {
        Self {
            inputs: Vec::with_capacity(other.inputs.len()),
            outputs: Vec::with_capacity(other.outputs.len()),
            params: Vec::with_capacity(other.params.len()),
        }
    }

    /// Create a new `TaskVars` where all three collections have the given capacity.
    pub fn with_default_capacity(cap: usize) -> Self {
        Self {
            inputs: Vec::with_capacity(cap),
            outputs: Vec::with_capacity(cap),
            params: Vec::with_capacity(cap),
        }
    }
}

/// Representation of a task defined in a workflow file.
#[derive(Debug, Default, Clone)]
pub struct Task {
    /// Inputs, Outputs and Params to this task (var name, value)
    pub vars: TaskVars<(IdentId, AbstractValueId)>,
    /// Id of string containing this task's execution code
    pub code: LiteralId,
    /// List of var names referenced in this task's code (for validation)
    pub referenced_vars: Vec<IdentId>,
    /// Optional id of module that this task should run in instead of its task directory
    pub module: Option<ModuleId>,
    /// So we can tell if this task is real, or just a default:
    pub exists: bool,
}

impl Task {
    /// Create a new task from its ast representation.
    pub fn create(
        block: ast::TasklikeBlock,
        strings: &mut WorkflowStrings,
        values: &mut IdVec<AbstractValueId, Value>,
    ) -> Result<Self> {
        // If there are few or zero specs, we may be able to avoid an alloc:
        let default_len = block.specs.len().min(DEFAULT_VARS_LEN);
        let mut vars = TaskVars::with_default_capacity(default_len);
        let mut module = None;

        use ast::BlockSpec::*;
        for spec in block.specs {
            match spec {
                Input { lhs, rhs } => vars.inputs.push(add_spec(lhs, rhs, strings, values)?),
                Output { lhs, rhs } => vars.outputs.push(add_spec(lhs, rhs, strings, values)?),
                Param { lhs, rhs, dot } => {
                    if dot {
                        return Err(Error::DotParamsUnsupported.into());
                    } else {
                        vars.params.push(add_spec(lhs, rhs, strings, values)?);
                    }
                }
                Module { name } => {
                    if module.is_none() {
                        module = Some(strings.modules.intern(name)?);
                    } else {
                        return Err(Error::MultipleModulesDefined.into());
                    }
                }
            }
        }

        let code = strings.literals.intern(block.code.text)?;
        let referenced_vars = block
            .code
            .vars
            .iter()
            .map(|id| strings.idents.intern(id))
            .collect::<Result<_, _>>()?;

        Ok(Self {
            vars,
            code,
            referenced_vars,
            module,
            exists: true,
        })
    }
}

fn add_spec(
    lhs: ast::Ident,
    rhs: ast::Rhs,
    strings: &mut WorkflowStrings,
    values: &mut IdVec<AbstractValueId, Value>,
) -> Result<(IdentId, AbstractValueId)> {
    let name = strings.idents.intern(lhs)?;
    let val = strings.create_value(lhs, rhs)?;
    let val_id = values.push(val);
    Ok((name, val_id))
}
