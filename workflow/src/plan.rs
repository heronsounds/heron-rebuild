use anyhow::Result;

use intern::InternStr;
use syntax::ast;

use crate::{AbstractTaskId, BranchSpec, Error, WorkflowStrings};

/// Representation of a plan defined in a config file.
#[derive(Debug, Clone)]
pub struct Plan {
    pub subplans: Vec<Subplan>,
}

impl Plan {
    pub fn create(
        strings: &mut WorkflowStrings,
        cross_products: Vec<ast::CrossProduct>,
    ) -> Result<Self> {
        // TODO error if cross_products is empty?
        let mut subplans = Vec::with_capacity(cross_products.len());
        for cross_product in cross_products {
            subplans.push(Subplan::create(strings, cross_product)?);
        }
        Ok(Self { subplans })
    }
}

/// One line of a plan (aka a cross-product; e.g. "reach task via (Branch: val1 val2)").
#[derive(Debug, Clone)]
pub struct Subplan {
    /// Tasks we want to reach.
    pub goals: Vec<AbstractTaskId>,
    /// Branches to realize tasks for.
    pub branches: Vec<BranchSpec>,
}

impl Subplan {
    pub fn create(strings: &mut WorkflowStrings, cross_product: ast::CrossProduct) -> Result<Self> {
        let mut goals = Vec::with_capacity(cross_product.goals.len());
        for goal in &cross_product.goals {
            let id = strings.tasks.intern(goal);
            goals.push(id);
        }

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
                    // for i in 1..len {
                    for v in vs.iter().skip(1) {
                        let v = strings.add_branch(k, v);
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

        Ok(Self { goals, branches })
    }
}
