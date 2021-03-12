/// type alias just to make type signatures look more consistent.
pub type Ident<'a> = &'a str;
/// type alias to make branch-related type signatures more readable.
pub type Branch<'a> = Vec<(&'a str, &'a str)>;

/// The right-hand side of any value expression.
/// Ducttape originally had another rhs type:
/// Sequential branchpoint expressions, written (Branchpoint: 0..10..1).
#[derive(Debug, PartialEq, Eq)]
pub enum Rhs<'a> {
    /// no rhs (e.g. in output specs)
    Unbound,
    /// "some quoted value" or unquoted_value_without_spaces
    Literal { val: &'a str },
    /// $var
    Variable { name: &'a str },
    /// @
    ShorthandVariable,
    /// $var[Branchpoint: val]
    GraftedVariable { name: Ident<'a>, branch: Branch<'a> },
    /// $var@task
    TaskOutput { task: &'a str, output: &'a str },
    /// @task
    ShorthandTaskOutput { task: &'a str },
    /// $var@task[Branchpoint: val]
    GraftedTaskOutput {
        task: &'a str,
        output: &'a str,
        branch: Vec<(&'a str, &'a str)>,
    },
    /// @task[Branchpoint: val]
    ShorthandGraftedTaskOutput {
        task: &'a str,
        branch: Vec<(&'a str, &'a str)>,
    },
    /// (Branchpoint: val1=$rhs1 val2=$rhs2)
    Branchpoint {
        branchpoint: &'a str,
        vals: Vec<(&'a str, Self)>,
    },
    /// "foo-$bla-blee" or just 'foo'
    Interp { text: &'a str, vars: Vec<&'a str> },
}

// These methods are just to assist with writing more legible tests.
#[cfg(test)]
impl<'a> Rhs<'a> {
    pub fn literal(val: &'a str) -> Self {
        Self::Literal { val }
    }
    pub fn variable(name: &'a str) -> Self {
        Self::Variable { name }
    }
    // pub fn shorthand_variable() -> Self {
    //     Self::ShorthandVariable
    // }
    pub fn grafted_variable(name: &'a str, branch: Branch<'a>) -> Self {
        Self::GraftedVariable { name, branch }
    }
    pub fn task_output(output: Ident<'a>, task: Ident<'a>) -> Self {
        Self::TaskOutput { output, task }
    }
    pub fn shorthand_task_output(task: Ident<'a>) -> Self {
        Self::ShorthandTaskOutput { task }
    }
    pub fn grafted_task_output(output: Ident<'a>, task: Ident<'a>, branch: Branch<'a>) -> Self {
        Self::GraftedTaskOutput {
            output,
            task,
            branch,
        }
    }
    pub fn shorthand_grafted_task_output(task: Ident<'a>, branch: Branch<'a>) -> Self {
        Self::ShorthandGraftedTaskOutput { task, branch }
    }
    pub fn branchpoint(branchpoint: Ident<'a>, vals: Vec<(Ident<'a>, Self)>) -> Self {
        Self::Branchpoint { branchpoint, vals }
    }
}

/// One part of the header of a [`TasklikeBlock`].
/// Ducttape had an additional spec type: package (syntax: ': package_name').
#[derive(Debug, PartialEq, Eq)]
pub enum BlockSpec<'a> {
    Output {
        lhs: &'a str,
        rhs: Rhs<'a>,
    },
    Input {
        lhs: &'a str,
        rhs: Rhs<'a>,
    },
    Param {
        lhs: &'a str,
        rhs: Rhs<'a>,
        dot: bool,
    },
    Module {
        name: Ident<'a>,
    },
}

#[cfg(test)]
impl<'a> BlockSpec<'a> {
    pub fn output(lhs: Ident<'a>, rhs: Rhs<'a>) -> Self {
        Self::Output { lhs, rhs }
    }
    pub fn input(lhs: Ident<'a>, rhs: Rhs<'a>) -> Self {
        Self::Input { lhs, rhs }
    }
    pub fn param(lhs: Ident<'a>, rhs: Rhs<'a>) -> Self {
        Self::Param {
            lhs,
            rhs,
            dot: false,
        }
    }
    pub fn dot_param(lhs: Ident<'a>, rhs: Rhs<'a>) -> Self {
        Self::Param {
            lhs,
            rhs,
            dot: true,
        }
    }
}

/// Specific type of a [`TasklikeBlock`].
/// Ducttape had the following additional types:
/// package, action, versioner, submitter, function.
/// We would like to at least add an equivalent to submitter in the future.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Task,
}

/// A block which uses the task structure.
#[derive(Debug, PartialEq, Eq)]
pub struct TasklikeBlock<'a> {
    /// Block name
    pub name: &'a str,
    /// Specific type of block
    pub subtype: BlockType,
    /// Header components
    pub specs: Vec<BlockSpec<'a>>,
    /// Bash code contained within braces
    pub code: BashCode<'a>,
}

/// A block which consists of multiple nested [`TasklikeBlock`]s.
#[derive(Debug, PartialEq, Eq)]
pub struct GrouplikeBlock<'a> {
    /// Block name
    pub name: &'a str,
    /// Specific type of block
    pub subtype: BlockType,
    /// Header components
    pub specs: Vec<BlockSpec<'a>>,
    /// Sub-blocks
    pub blocks: Vec<TasklikeBlock<'a>>,
}

/// A block of bash code.
#[derive(Debug, PartialEq, Eq)]
pub struct BashCode<'a> {
    /// The literal text of the code.
    pub text: &'a str,
    /// Set of variable names referenced in the code.
    pub vars: crate::HashSet<Ident<'a>>,
}

/// Specification of branches for a single branchpoint.
#[derive(Debug, PartialEq, Eq)]
pub enum Branches<'a> {
    /// Specifies all branches (`*`).
    Glob,
    /// A specific set of branches (e.g. `branch1 branch2 branch3` etc.)
    Specified(Vec<&'a str>),
}

/// One part of a [`Plan`], consisting of a list of goal tasks and a list of branches.
#[derive(Debug, PartialEq, Eq)]
pub struct CrossProduct<'a> {
    /// Task names for the traversal to reach.
    pub goals: Vec<Ident<'a>>,
    /// List of (branchpoint name, branches) pairs used to form traversal.
    pub branches: Vec<(Ident<'a>, Branches<'a>)>,
}

/// A block of one or more [`CrossProduct`]s that specify a traversal through the workflow.
#[derive(Debug, PartialEq, Eq)]
pub struct Plan<'a> {
    /// Plan name
    pub name: &'a str,
    /// List of contained [`CrossProduct`]s
    pub cross_products: Vec<CrossProduct<'a>>,
}

/// One high-level item in the workflow.
#[derive(Debug, PartialEq, Eq)]
pub enum Item<'a> {
    // Versioner(GrouplikeBlock<'a>),
    /// A task definition.
    Task(TasklikeBlock<'a>),
    /// An import statement.
    Import(&'a str),
    // Package(TasklikeBlock<'a>),
    /// A block of config variables.
    GlobalConfig(Vec<(&'a str, Rhs<'a>)>),
    /// A [`Plan`].
    Plan(Plan<'a>),
    /// A module definition.
    Module(Ident<'a>, Rhs<'a>),
}
