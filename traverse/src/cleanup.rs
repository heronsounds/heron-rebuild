use anyhow::Result;
use colored::Colorize;

use intern::GetStr;
use util::Bitmask;
use workflow::{BranchMask, BranchSpec, RealInput, Workflow};

use super::{Node, Traversal};

/// Reverse a Traversal, which starts out in reverse-execution order,
/// and update all node indexes to match new ordering.
pub fn reverse(traversal: &mut Traversal) {
    let nodes = &mut traversal.nodes;
    let len = nodes.len();
    let final_idx = len - 1;

    // first correct links between nodes:
    for i in 0..(len / 2) {
        let to_swap = final_idx - i;
        correct_next_idx(&mut nodes[i], final_idx);
        correct_next_idx(&mut nodes[to_swap], final_idx);
        nodes.swap(i, to_swap);
    }

    // now correct all task inputs:
    let task_ids = traversal.inputs.iter_mut().filter_map(|val| {
        if let RealInput::Task(t, _) = val {
            Some(t)
        } else {
            None
        }
    });

    for task_id in task_ids {
        let reversed_task_id = final_idx - usize::from(*task_id);
        *task_id = reversed_task_id.into();
    }
}

fn correct_next_idx(node: &mut Node, final_idx: usize) {
    node.next_idx = final_idx - node.next_idx;
}

/// Filter out unneeded branches from a Traversal,
/// so that branches can be used as keys to dedupe tasks in the next step.
pub fn clean_branches(traversal: &mut Traversal, wf: &Workflow) -> Result<()> {
    log::info!(
        "Cleaning branches for traversal with {} roots",
        traversal.num_roots
    );
    let mut root_count = 0;
    for i in 0..traversal.nodes.len() {
        let mut node = &mut traversal.nodes[i];
        if node.is_root {
            let mut idx = i;
            let mut traversal_mask = BranchMask::default();
            loop {
                log::debug!(
                    "Cleaning branches for {}[{}]",
                    wf.strings.tasks.get(node.key.abstract_task_id).cyan(),
                    traversal.branch_strs.get(&node.key.branch)?,
                );

                log::trace!("traversal mask: {:#b}", traversal_mask);
                log::trace!("this node removes: {:#b}", node.masks.rm);
                log::trace!("this node adds: {:#b}", node.masks.add);

                // filter first, then add, b/c we can prune a branchpoint and then add it in the same node:
                traversal_mask &= !node.masks.rm;
                traversal_mask |= node.masks.add;

                rm_filtered_branchpoints(&mut node.key.branch, &traversal_mask, wf);

                log::debug!(
                    "After cleaning: {}",
                    traversal.branch_strs.get_or_insert(&node.key.branch, wf),
                );

                // if node is terminal/is a goal node, this traversal is done:
                if node.next_idx == idx {
                    break;
                } else {
                    idx = node.next_idx;
                    node = &mut traversal.nodes[idx];
                }
            }
            // once we've traversed from each root to the end, we're done:
            root_count += 1;
            if root_count >= traversal.num_roots {
                break;
            }
        }
    }

    Ok(())
}

/// Replace branches that have been filtered out with baseline/NULL_IDENT.
fn rm_filtered_branchpoints<T: Bitmask>(branch: &mut BranchSpec, mask: &T, wf: &Workflow) {
    for i in 0..wf.strings.branchpoints.len() {
        let branchpoint_id = i.into();
        log::trace!("checking branchpoint {}", wf.strings.branchpoints.get(branchpoint_id));
        if !mask.get(i) {
            log::trace!("not in mask; removing.");
            branch.unset(branchpoint_id);
        } else if branch.is_unspecified(branchpoint_id) {
            log::trace!("branch is in mask, but not specified in node. Adding baseline.");
            // log::debug!("missing branchpoint {:?}: {}",
            //     branchpoint_id,
            //     wf.strings.branchpoints.get(branchpoint_id),
            // );
            branch.insert(branchpoint_id, wf.strings.baselines.get(branchpoint_id));
        } else {
            log::trace!("branch is all good.");
        }
    }
}
