use anyhow::Result;
use colored::Colorize;
use std::collections::VecDeque;

use intern::GetStr;
use util::{Bitmask, IdVec};
use workflow::{AbstractValueId, BranchStrs, IdentId, RealValueId, Workflow};

use super::value::{PartialRealInput, RealInput, ValueResolver};
use super::{Error, Errors, NodeBuilder, NodeIdx, RealTaskKey, TraversalBuilder};

const QUEUE_CAPACITY: usize = 32;
const ROOTS_CAPACITY: usize = 8;

struct QueueNode {
    key: RealTaskKey,
    next_idx: NodeIdx,
}

/// Breadth-first search traversal strategy
pub struct BfsTraverser<'a, B> {
    /// workflow info
    wf: &'a Workflow,
    /// used internally to create bfs traversal
    queue: VecDeque<QueueNode>,
    /// traversal we will build iteratively w/ calls to traverse()
    traversal: TraversalBuilder<B>,
    /// turns abstract values into real values:
    resolver: ValueResolver,
}

impl<'a, B: Bitmask> BfsTraverser<'a, B> {
    /// Create a new BfsTraverser with the given workflow info
    pub fn new(wf: &'a Workflow) -> Self {
        let len_x2 = wf.strings.tasks.len() * 2;
        let len_x8 = len_x2 * 4;
        Self {
            wf,
            queue: VecDeque::with_capacity(QUEUE_CAPACITY),
            traversal: TraversalBuilder {
                nodes: Vec::with_capacity(len_x2),
                inputs: IdVec::with_capacity(len_x2),
                outputs_params: IdVec::with_capacity(len_x8),
                branch_strs: BranchStrs::with_capacity_and_avg_len(32, 32),
                roots: Vec::with_capacity(ROOTS_CAPACITY),
                errors: Errors::default(), //Vec::with_capacity(0),
            },
            resolver: ValueResolver,
        }
    }

    /// Consume this struct and return its completed Traversal.
    pub fn into_traversal(self) -> TraversalBuilder<B> {
        self.traversal
    }

    /// Add a traversal to the given goal node to our internal Traversal.
    pub fn traverse(&mut self, key: RealTaskKey) -> Result<()> {
        self.enqueue(key, downcast(self.traversal.nodes.len())?)?;
        while let Some(node) = self.queue.pop_front() {
            self.handle(node)?;
        }
        Ok(())
    }

    /// Handle a single node popped off the queue.
    fn handle(&mut self, node: QueueNode) -> Result<()> {
        let task_id = node.key.id;
        log::debug!(
            "Handling enqueued node {}[{}]",
            self.wf.strings.tasks.get(task_id)?.cyan(),
            self.traversal.branch_strs.get(&node.key.branch)?,
        );

        // fetch task info and create new node
        let this_node_id = downcast(self.traversal.nodes.len())?;
        let task = self.wf.get_task(task_id)?;
        let mut node = NodeBuilder::new(node.key, node.next_idx, task);

        // handle inputs
        for (k, input) in &task.vars.inputs {
            log::trace!("handling input {}", self.wf.strings.idents.get(*k)?);
            match self.handle_input(*input, &mut node, this_node_id) {
                Ok(val_id) => node.vars.inputs.push((*k, val_id)),
                Err(e) => self.handle_err(&node.key, *k, "input", e)?,
            }
        }
        // if node is still root (i.e. no inputs were from other tasks), add it to roots vec:
        if node.is_root {
            self.traversal.roots.push(this_node_id);
        }

        // handle params
        for (k, param) in &task.vars.params {
            log::trace!("handling param {}", self.wf.strings.idents.get(*k)?);
            match self.handle_output_or_param(*param, &mut node) {
                Ok(val_id) => node.vars.params.push((*k, val_id)),
                Err(e) => self.handle_err(&node.key, *k, "param", e)?,
            }
        }

        // handle outputs
        for (k, output) in &task.vars.outputs {
            log::trace!("handling output {}", self.wf.strings.idents.get(*k)?);
            match self.handle_output_or_param(*output, &mut node) {
                Ok(val_id) => node.vars.outputs.push((*k, val_id)),
                Err(e) => self.handle_err(&node.key, *k, "output", e)?,
            }
        }

        log::trace!("node now adds: {:#b}", node.masks.add);
        log::trace!("node now rms: {:#b}", node.masks.rm);

        self.traversal.nodes.push(node);
        Ok(())
    }

    fn handle_err(
        &mut self,
        key: &RealTaskKey,
        k: IdentId,
        ty: &str,
        e: anyhow::Error,
    ) -> Result<()> {
        let msg = format!(
            "Invalid {ty} '{}' in task '{}[{}]'",
            self.wf.strings.idents.get(k)?.yellow(),
            self.wf.strings.tasks.get(key.id)?.cyan(),
            self.traversal.branch_strs.get_or_insert(&key.branch, self.wf)?,
        );
        self.traversal.errors.add_context(e, msg);
        Ok(())
    }

    fn enqueue(&mut self, key: RealTaskKey, next_idx: NodeIdx) -> Result<()> {
        log::debug!(
            "Enqueueing {}[{}]",
            self.wf.strings.tasks.get(key.id)?.cyan(),
            self.traversal.branch_strs.get_or_insert(&key.branch, self.wf)?,
        );
        self.queue.push_back(QueueNode { key, next_idx });
        Ok(())
    }

    fn handle_input(
        &mut self,
        val: AbstractValueId,
        node: &mut NodeBuilder<B>,
        this_node_id: NodeIdx,
    ) -> Result<RealValueId> {
        let val = self.wf.get_value(val)?;
        let (val, masks) = self.resolver.resolve::<_, B>(val, &node.key.branch, self.wf)?;

        let real_val = match val {
            PartialRealInput::Task(id, ident, branch) => {
                node.is_root = false;

                if id == node.key.id {
                    return Err(
                        Error::ReflexiveTask(self.wf.strings.tasks.get(id)?.to_owned()).into(),
                    );
                }

                let key = RealTaskKey { id, branch };
                self.enqueue(key, this_node_id)?;

                let real_task_id = downcast(this_node_id as usize + self.queue.len())?.into();

                // TODO check if task actually has output with this ident here?

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
        node: &mut NodeBuilder<B>,
    ) -> Result<RealValueId> {
        let val = self.wf.get_value(val)?;
        let (val, masks) = self.resolver.resolve::<_, B>(val, &node.key.branch, self.wf)?;
        log::trace!("value adds branches: {:#b}", masks.add);
        log::trace!("value rms branches: {:#b}", masks.rm);
        let val_id = self.traversal.outputs_params.push(val);
        node.masks.or_eq(&masks);
        Ok(val_id)
    }
}

fn downcast(val: usize) -> Result<NodeIdx, Error> {
    val.try_into().map_err(|_| Error::OutOfIndices(val))
}
