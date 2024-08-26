use anyhow::Result;

use intern::InternStr;
use syntax::ast;

use crate::{AbstractTaskId, BranchSpec, Error, WorkflowStrings};

// i.e. cross product
// #[derive(Debug)]
// pub struct SubPlan {
//     pub goals: Vec<AbstractTaskId>,
//     // TODO allow multidimensional branches:
//     pub branch: BranchSpec,
// }

/// A plan defined in a config file.
#[derive(Debug, Clone)]
pub struct Plan {
    /// Tasks we want to reach.
    pub goals: Vec<AbstractTaskId>,
    /// Branches to realize tasks for.
    pub branches: Vec<BranchSpec>,
}

impl Plan {
    pub fn create(
        strings: &mut WorkflowStrings,
        cross_products: Vec<ast::CrossProduct>,
    ) -> Result<Self> {
        if cross_products.len() != 1 {
            return Err(Error::Unsupported("plans with multiple subplans".to_owned()).into());
        }

        let cross_product = &cross_products[0];

        // let ast::CrossProduct { goals, branches } = &cross_products[0];
        // if goals.len() != 1 {
        //     return Err(Error::Unsupported("plans with multiple goal nodes".to_owned()).into());
        // }

        let mut goals = Vec::with_capacity(cross_product.goals.len());
        for goal in &cross_product.goals {
            let id = strings.tasks.intern(goal);
            goals.push(id);
        }

        // let goal = goals[0];
        // let goal_id = strings.tasks.intern(goal);

        let mut branches = vec![BranchSpec::default()];
        for (k, vs) in &cross_product.branches {
            let k = strings.add_branchpoint(k); // strings.branchpoints.intern(k);
            let vs = match vs {
                ast::Branches::Specified(vec) => vec,
                _ => {
                    return Err(Error::Unsupported(
                        "plans with branch glob specifications".to_owned(),
                    )
                    .into())
                }
            };

            match vs.len() {
                0 => todo!("this probably shouldn't happen"),
                1 => {
                    // if len is 1, no need to split. just add to each existing branch.
                    let v = strings.add_branch(k, vs[0]);
                    for branch in &mut branches {
                        branch.insert(k, v);
                    }
                }
                len => {
                    branches.reserve(branches.len() * len);
                    // insert the first val:
                    let v0 = strings.add_branch(k, vs[0]);
                    for branch in &mut branches {
                        branch.insert(k, v0);
                    }
                    // now clone for each subsequent val, and insert:
                    let mut new_branches = Vec::with_capacity(branches.len() * len);
                    for i in 1..len {
                        let v = strings.add_branch(k, vs[i]);
                        for branch in &branches {
                            let mut new_branch = branch.clone();
                            new_branch.insert(k, v);
                            new_branches.push(new_branch);
                        }
                    }
                    // now add those to the original branches array:
                    branches.append(&mut new_branches);
                }
            }
        }

        // if branches.len() > 1 {
        //     return Err(Error::Unsupported("plans with multiple branches".to_owned()).into());
        // }

        // let branch = branches.pop().unwrap();

        Ok(Self {
            goals,
            branches,
        })
    }
}
