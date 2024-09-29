use anyhow::Result;

use intern::InternStr;
use syntax::ast;

use crate::{BranchSpec, IdentId, WorkflowStrings};

use super::{BaseValue, DirectValue, Value};

// TODO these cd be added to a zero-sized struct...

/// Create a `Value` from the left-hand and right-hand side ast representations.
pub fn create_value(
    strings: &mut WorkflowStrings,
    lhs: ast::Ident,
    rhs: ast::Rhs,
) -> Result<Value> {
    use ast::Rhs::*;
    match rhs {
        Branchpoint { branchpoint, vals } => {
            let outer_k = strings.branchpoints.intern(branchpoint)?;
            let mut flattened_vals = Vec::with_capacity(vals.len());
            for (branch_lhs, val) in vals {
                let outer_v = strings.idents.intern(branch_lhs)?;
                strings.baselines.add(outer_k, outer_v);
                match create_value(strings, branch_lhs, val)? {
                    Value::Branched(nested_vals) => {
                        for (mut nested_branch, nested_val) in nested_vals {
                            nested_branch.insert(outer_k, outer_v);
                            flattened_vals.push((nested_branch, nested_val));
                        }
                    }
                    Value::Direct(val) => {
                        let branch = BranchSpec::simple(outer_k, outer_v);
                        flattened_vals.push((branch, val));
                    }
                }
            }
            Ok(Value::Branched(flattened_vals))
        }
        direct_rhs => Ok(Value::Direct(create_direct(strings, lhs, direct_rhs)?)),
    }
}

fn create_direct(
    strings: &mut WorkflowStrings,
    lhs: ast::Ident,
    rhs: ast::Rhs,
) -> Result<DirectValue> {
    use ast::Rhs::*;
    match rhs {
        GraftedVariable { name, branch } => {
            let name = strings.idents.intern(name)?;
            let value = BaseValue::Config(name);
            let branch = create_branch(strings, branch)?;
            Ok(DirectValue::Graft(value, branch))
        }
        GraftedTaskOutput {
            task,
            output,
            branch,
        } => {
            let task = strings.tasks.intern(task)?;
            let output = strings.idents.intern(output)?;
            let value = BaseValue::Task(task, output);
            let branch = create_branch(strings, branch)?;
            Ok(DirectValue::Graft(value, branch))
        }
        ShorthandGraftedTaskOutput { task, branch } => {
            let task = strings.tasks.intern(task)?;
            let output = strings.idents.intern(lhs)?;
            let value = BaseValue::Task(task, output);
            let branch = create_branch(strings, branch)?;
            Ok(DirectValue::Graft(value, branch))
        }
        _ => Ok(DirectValue::Simple(create_base(strings, lhs, rhs)?)),
    }
}

#[rustfmt::skip]
fn create_base(strings: &mut WorkflowStrings, lhs: ast::Ident, rhs: ast::Rhs) -> Result<BaseValue> {
    use ast::Rhs::*;
    match rhs {
        Unbound             => strings.literals.intern(lhs).map(BaseValue::Literal),
        Literal { val }     => strings.literals.intern(val).map(BaseValue::Literal),
        Variable { name }   => strings.idents.intern(name).map(BaseValue::Config),
        ShorthandVariable   => strings.idents.intern(lhs).map(BaseValue::Config),
        TaskOutput { task, output } => {
            let task = strings.tasks.intern(task)?;
            let output = strings.idents.intern(output)?;
            Ok(BaseValue::Task(task, output))
        }
        ShorthandTaskOutput { task } => {
            let task = strings.tasks.intern(task)?;
            let output = strings.idents.intern(lhs)?;
            Ok(BaseValue::Task(task, output))
        }
        Interp { text, vars } => {
            let val = strings.literals.intern(text)?;
            let mut vars: Vec<IdentId> = vars
                .into_iter()
                .map(|var| strings.idents.intern(var))
                .collect::<Result<_, _>>()?;
            // our parser puts interp vars in reverse order,
            // but we want them ordered so we can optimize interpolation down the line:
            vars.reverse();
            Ok(BaseValue::Interp(val, vars))
        }
        _ => {
            unreachable!("Should not be handling grafted or branched values here")
        }
    }
}

fn create_branch(strings: &mut WorkflowStrings, branch: ast::Branch) -> Result<BranchSpec> {
    let mut spec = BranchSpec::default();
    for (k, v) in branch {
        let k = strings.branchpoints.intern(k)?;
        let v = strings.idents.intern(v)?;
        spec.insert(k, v);
    }
    Ok(spec)
}
