use anyhow::Result;
use colored::Colorize;

use intern::GetStr;
use util::Bitmask;
use workflow::{BranchSpec, Workflow};

use super::{value::RealInput, Node, Traversal, TraversalBuilder};

/// Reverse the traversal, and convert to `Traversal` type,
/// stripping unnecessary info from the TraversalBuilder.
pub fn reverse_and_strip<B>(mut traversal: TraversalBuilder<B>) -> Traversal {
    let nodes: Vec<_> = traversal.nodes.into_iter().map(Node::from).rev().collect();

    let final_idx = nodes.len() - 1;
    for val in traversal.inputs.iter_mut() {
        if let RealInput::Task(t, _) = val {
            let reversed_task_id = final_idx - usize::from(*t);
            *t = reversed_task_id.into();
        }
    }

    Traversal {
        nodes,
        inputs: traversal.inputs,
        outputs_params: traversal.outputs_params,
        branch_strs: traversal.branch_strs,
    }
}

pub fn clean_branches_reversed<B: Bitmask>(
    traversal: &mut TraversalBuilder<B>,
    wf: &Workflow,
) -> Result<()> {
    log::debug!(
        "Cleaning branches for traversal with {} roots",
        traversal.roots.len(),
    );
    for root_idx in &traversal.roots {
        let mut idx = *root_idx;
        let mut node = &mut traversal.nodes[idx as usize];
        let mut traversal_mask = B::default();
        loop {
            log::debug!(
                "Cleaning branches for {}[{}]",
                wf.strings.tasks.get(node.key.id)?.cyan(),
                traversal.branch_strs.get(&node.key.branch)?,
            );

            log::trace!("traversal mask: {:#b}", traversal_mask);
            log::trace!("this node removes: {:#b}", node.masks.rm);
            log::trace!("this node adds: {:#b}", node.masks.add);

            // filter first, then add, b/c we can prune a branchpoint and then add it in the same node:
            traversal_mask &= !node.masks.rm;
            traversal_mask |= node.masks.add;

            rm_filtered_branchpoints(&mut node.key.branch, &traversal_mask, wf)?;

            log::debug!(
                "After cleaning: {}",
                traversal.branch_strs.get_or_insert(&node.key.branch, wf)?,
            );

            // if node is terminal/is a goal node, this traversal is done:
            if node.next_idx == idx {
                break;
            } else {
                idx = node.next_idx;
                node = &mut traversal.nodes[idx as usize];
            }
        }
    }
    Ok(())
}

/// Replace branches that have been filtered out with baseline/NULL_IDENT.
fn rm_filtered_branchpoints<B: Bitmask>(
    branch: &mut BranchSpec,
    mask: &B,
    wf: &Workflow,
) -> Result<()> {
    for i in 0..wf.strings.branchpoints.len() {
        let branchpoint_id = i.into();
        log::trace!(
            "checking branchpoint {}",
            wf.strings.branchpoints.get(branchpoint_id)?
        );
        if !mask.get(i) {
            log::trace!("not in mask; removing.");
            branch.unset(branchpoint_id);
        } else if branch.is_unspecified(branchpoint_id) {
            log::trace!("branch is in mask, but not specified in node. Adding baseline.");
            branch.insert(branchpoint_id, wf.strings.baselines.get(branchpoint_id));
        } else {
            log::trace!("branch is all good.");
        }
    }
    Ok(())
}
