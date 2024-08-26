use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use intern::{GetStr, InternStr};
use syntax::ast;
use util::{HashMap, Hasher, IdVec, PathEncodingError};

use crate::{
    AbstractTaskId, AbstractValueId, BranchMasks, BranchSpec, Error, IdentId, LiteralId, ModuleId,
    Plan, RealValueLike, Task, Value, ValueResolver, WorkflowStrings,
};

/// Used to initialize collections later in the process.
#[derive(Debug, Default)]
pub struct SizeHints {
    pub max_inputs: u8,
    pub max_outputs: u8,
    pub max_params: u8,
    pub max_vars: u8,
}

/// Contains all the information about a workflow,
/// in a form that can be used to generate a traversal to run.
#[derive(Debug)]
pub struct Workflow {
    pub strings: WorkflowStrings,
    config: HashMap<IdentId, AbstractValueId>,
    tasks: IdVec<AbstractTaskId, Task>,
    plans: Vec<(IdentId, Plan)>,
    modules: IdVec<ModuleId, LiteralId>,
    values: IdVec<AbstractValueId, Value>,
    sizes: SizeHints,
    resolver: ValueResolver,
}

impl Default for Workflow {
    fn default() -> Self {
        Self {
            strings: WorkflowStrings::default(),
            config: HashMap::with_capacity_and_hasher(64, Hasher::default()),
            tasks: IdVec::with_capacity(16),
            plans: Vec::with_capacity(8),
            modules: IdVec::with_capacity(8),
            values: IdVec::with_capacity(128),
            sizes: SizeHints::default(),
            resolver: ValueResolver,
        }
    }
}

impl Workflow {
    /// Load the given ast representations of blocks into this `Workflow`.
    /// `config_dir` is used to interpret relative paths to modules.
    #[rustfmt::skip]
    pub fn load(&mut self, blocks: Vec<ast::Item>, config_dir: &Path) -> Result<()> {
        for block in blocks {
            match block {
                ast::Item::GlobalConfig(assts)  => self.add_config(assts)?,
                ast::Item::Task(task)           => self.add_task(task)?,
                ast::Item::Plan(plan)           => self.add_plan(plan)?,
                ast::Item::Module(name, path)   => self.add_module(name, path, config_dir)?,
                _ => {
                    return Err(Error::Unsupported(
                        "blocks other than config, task, plan, module".to_owned(),
                    )
                    .into())
                }
            }
        }
        Ok(())
    }

    /// Get a reference to size hints for initializing collections.
    #[inline]
    pub fn sizes(&self) -> &SizeHints {
        &self.sizes
    }

    /// Get a string containing the path to the module with the given id.
    #[inline]
    pub fn get_module_path(&self, module: ModuleId) -> &str {
        let lit_id = self.modules.get(module);
        self.strings.literals.get(*lit_id)
    }

    /// Get the task with the given id.
    #[inline]
    pub fn get_task(&self, task: AbstractTaskId) -> &Task {
        self.tasks.get(task)
    }

    /// Get the value with the given id.
    #[inline]
    pub fn get_value(&self, value: AbstractValueId) -> &Value {
        self.values.get(value)
    }

    /// Get the value id of a config value defined with the given identifier.
    #[inline]
    pub fn get_config_value(&self, ident: IdentId) -> AbstractValueId {
        self.config[&ident]
    }

    /// Total number of values defined (including task variables and config values).
    #[inline]
    pub fn num_values(&self) -> usize {
        self.values.len()
    }

    /// Resolve the given value for execution on the given branch.
    #[inline]
    pub fn resolve<T: RealValueLike>(
        &self,
        val: &Value,
        branch: &BranchSpec,
    ) -> Result<(T, BranchMasks)> {
        self.resolver.resolve(val, branch, self)
    }

    /// Get a reference to the plan defined with the given identifier.
    pub fn get_plan(&self, plan_name: IdentId) -> Result<&Plan> {
        for (k, plan) in &self.plans {
            if *k == plan_name {
                return Ok(plan);
            }
        }
        let plan_name = self.strings.idents.get(plan_name);
        Err(Error::PlanNotFound(plan_name.to_owned()).into())
    }
}

// building the workflow /////////////
impl Workflow {
    fn add_config(&mut self, assts: Vec<(&str, ast::Rhs)>) -> Result<()> {
        for (k, v) in assts {
            let v = self.strings.create_value(k, v);
            let vid = self.values.push(v);
            let k = self.strings.idents.intern(k);
            self.config.insert(k, vid);
        }
        Ok(())
    }

    fn add_task(&mut self, task: ast::TasklikeBlock) -> Result<()> {
        let name_id = self.strings.tasks.intern(task.name);
        let task = Task::create(task, &mut self.strings, &mut self.values)?;
        self.update_sizes(&task);
        // NB we have no easy, surefire way to tell if a task with the same
        // name was added, so if that happens then the task will just be
        // overwritten. Wd be nice to make that an error eventually.
        self.tasks.insert(name_id, task);
        Ok(())
    }

    fn update_sizes(&mut self, task: &Task) {
        let num_inputs = task.vars.inputs.len() as u8;
        let num_outputs = task.vars.outputs.len() as u8;
        let num_params = task.vars.params.len() as u8;
        let num_vars = num_inputs + num_outputs + num_params;
        self.sizes.max_inputs = self.sizes.max_inputs.max(num_inputs);
        self.sizes.max_outputs = self.sizes.max_outputs.max(num_outputs);
        self.sizes.max_params = self.sizes.max_params.max(num_params);
        self.sizes.max_vars = self.sizes.max_vars.max(num_vars);
    }

    fn add_plan(&mut self, plan: ast::Plan) -> Result<()> {
        let plan_id = self.strings.idents.intern(plan.name);
        let ast::Plan { cross_products, .. } = plan;
        let plan = Plan::create(&mut self.strings, cross_products)
            .with_context(|| format!("while creating AST for plan \"{}\"", plan.name))?;

        // NB we don't use an IdVec bc plans use the idents table,
        // so the vec would be very sparse.
        self.plans.push((plan_id, plan));
        Ok(())
    }

    fn add_module(&mut self, name: &str, path: ast::Rhs, config_dir: &Path) -> Result<()> {
        let id = self.strings.modules.intern(name);
        if let ast::Rhs::Literal { val } = path {
            let mut path = PathBuf::from(val);
            // if path is relative, it is relative to the config_dir:
            // TODO! this fails cryptically if the module dir doesn't exist!
            if path.is_relative() {
                path = config_dir.join(path).canonicalize()?;
            }
            let path_str = path.to_str().ok_or(PathEncodingError)?;
            let literal_id = self.strings.literals.intern(path_str);
            self.modules.insert(id, literal_id);
            Ok(())
        } else {
            Err(Error::Unsupported(format!(
                "Module values other than literal strings (in module \"{}\")",
                name
            ))
            .into())
        }
    }
}
