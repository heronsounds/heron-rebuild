use anyhow::Result;
use colored::Colorize;
use std::collections::VecDeque;

use intern::GetStr;
use util::IdVec;
use workflow::{
    AbstractValueId, BranchStrs, PartialRealInput, RealInput, RealOutputOrParam, RealValueId,
    Workflow,
};

use super::{Node, RealTaskKey, Traversal};

const QUEUE_CAPACITY: usize = 32;

struct QueueNode {
    key: RealTaskKey,
    next_idx: usize,
}

/// Breadth-first search traversal strategy
pub struct BfsTraverser<'a> {
    /// workflow info
    wf: &'a Workflow,
    /// used internally to create bfs traversal
    queue: VecDeque<QueueNode>,
    /// traversal we will build iteratively w/ calls to traverse()
    traversal: Traversal,
    /// verbosity level (for logging):
    verbose: bool,
}

impl<'a> BfsTraverser<'a> {
    /// Create a new BfsTraverser with the given workflow info
    pub fn new(wf: &'a Workflow, verbose: bool) -> Self {
        let len_x2 = wf.strings.tasks.len() * 2;
        let len_x8 = len_x2 * 4;
        Self {
            wf,
            verbose,
            queue: VecDeque::with_capacity(QUEUE_CAPACITY),
            traversal: Traversal {
                nodes: Vec::with_capacity(len_x2),
                inputs: IdVec::with_capacity(len_x2),
                outputs_params: IdVec::with_capacity(len_x8),
                num_roots: 0,
                branch_strs: BranchStrs::with_capacity_and_avg_len(32, 32),
            },
        }
    }

    /// Consume this struct and return its completed Traversal.
    pub fn into_traversal(self) -> Traversal {
        self.traversal
    }

    /// Add a traversal to the given goal node to our internal Traversal.
    pub fn traverse(&mut self, key: RealTaskKey) -> Result<()> {
        self.enqueue(key, self.traversal.nodes.len());
        while let Some(node) = self.queue.pop_front() {
            self.handle(node)?;
        }
        Ok(())
    }

    /// Handle a single node popped off the queue.
    fn handle(&mut self, node: QueueNode) -> Result<()> {
        if self.verbose {
            log::info!(
                "Handling enqueued node {}[{}]",
                self.wf.strings.tasks.get(node.key.abstract_task_id).cyan(),
                self.traversal.branch_strs.get(&node.key.branch)?,
            );
        }

        let this_node_id = self.traversal.nodes.len();
        let task = self.wf.get_task(node.key.abstract_task_id);
        let mut node = Node::new(node.key, node.next_idx, task);

        for (k, input) in &task.vars.inputs {
            let val_id = self.handle_input(*input, &mut node, this_node_id)?;
            node.vars.inputs.push((*k, val_id));
        }
        if node.is_root {
            self.traversal.num_roots += 1;
        }

        for (k, param) in &task.vars.params {
            let val_id = self.handle_output_or_param(*param, &mut node)?;
            node.vars.params.push((*k, val_id));
        }

        for (k, output) in &task.vars.outputs {
            let val_id = self.handle_output_or_param(*output, &mut node)?;
            node.vars.outputs.push((*k, val_id));
        }

        self.traversal.nodes.push(node);
        Ok(())
    }

    fn enqueue(&mut self, key: RealTaskKey, next_idx: usize) {
        if self.verbose {
            log::info!(
                "Enqueueing {}[{}]",
                self.wf.strings.tasks.get(key.abstract_task_id).cyan(),
                self.traversal
                    .branch_strs
                    .get_or_insert(&key.branch, self.wf),
            );
        }
        self.queue.push_back(QueueNode { key, next_idx });
    }

    fn handle_input(
        &mut self,
        val: AbstractValueId,
        node: &mut Node,
        this_node_id: usize,
    ) -> Result<RealValueId> {
        let val = self.wf.get_value(val);
        let (val, masks) = self.wf.resolve::<PartialRealInput>(val, &node.key.branch)?;

        let real_val = match val {
            PartialRealInput::Task(abstract_task_id, ident, branch) => {
                node.is_root = false;
                let key = RealTaskKey {
                    abstract_task_id,
                    branch,
                };
                self.enqueue(key, this_node_id);
                let real_task_id = (this_node_id + self.queue.len()).into();
                RealInput::Task(real_task_id, ident)
            }
            PartialRealInput::Literal(lit_id) => RealInput::Literal(lit_id),
        };

        let val_id = self.traversal.inputs.push(real_val);
        node.masks.or_eq(&masks);
        Ok(val_id)
    }

    fn handle_output_or_param(
        &mut self,
        val: AbstractValueId,
        node: &mut Node,
    ) -> Result<RealValueId> {
        let val = self.wf.get_value(val);
        let (val, masks) = self
            .wf
            .resolve::<RealOutputOrParam>(val, &node.key.branch)?;
        let val_id = self.traversal.outputs_params.push(val);
        node.masks.or_eq(&masks);
        Ok(val_id)
    }
}
