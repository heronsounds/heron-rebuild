use anyhow::Result;

use intern::InternStr;
use syntax::ast;

use crate::{AbstractTaskId, BranchSpec, Error, WorkflowStrings};

// i.e. cross product
// #[derive(Debug)]
// pub struct SubPlan {

// }

// this is a simplified version of the plan api,
// with only a single goal node
// and a single branch spec.
/// A plan defined in a config file.
#[derive(Debug)]
pub struct Plan {
    /// Task we want to reach.
    pub goal: AbstractTaskId,
    /// Branch for the task we want to reach.
    pub branch: BranchSpec,
}

impl Plan {
    pub fn create(
        strings: &mut WorkflowStrings,
        cross_products: Vec<ast::CrossProduct>,
    ) -> Result<Self> {
        if cross_products.len() != 1 {
            return Err(Error::Unsupported("plans with multiple subplans".to_owned()).into());
        }

        let ast::CrossProduct { goals, branches } = &cross_products[0];
        if goals.len() != 1 {
            return Err(Error::Unsupported("plans with multiple goal nodes".to_owned()).into());
        }

        let goal = goals[0];
        let goal_id = strings.tasks.intern(goal);

        let mut branch = BranchSpec::default();
        for (k, vs) in branches {
            let k = strings.branchpoints.intern(k);
            let v = match vs {
                ast::Branches::Specified(vec) if vec.len() == 1 => vec[0],
                _ => {
                    return Err(Error::Unsupported(
                        "plans with globbed or multiple branches".to_owned(),
                    )
                    .into())
                }
            };
            let v = strings.idents.intern(v);
            branch.insert(k, v);
        }

        Ok(Self {
            goal: goal_id,
            branch,
        })
    }
}
