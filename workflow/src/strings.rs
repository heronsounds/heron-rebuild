use anyhow::Result;

use intern::{GetStr, InternStr, LooseInterner, PackedInterner, TypedInterner};
use syntax::ast;

use crate::value::create_value;
use crate::{
    AbstractTaskId, BaselineBranches, BranchpointId, Error, IdentId, LiteralId, ModuleId, RunStrId,
    Value,
};

/// Stores all the interned strings associated with a Workflow.
#[derive(Debug)]
pub struct WorkflowStrings {
    /// Names of branchpoints
    // TODO this could just be a vec of IdentIds and we store the names there!
    pub branchpoints: TypedInterner<BranchpointId, PackedInterner<u8, u8>>,
    /// Names of tasks
    pub tasks: TypedInterner<AbstractTaskId, PackedInterner<u8, u16>>,
    /// Names of other idents (variables, branches, etc.)
    pub idents: TypedInterner<IdentId, PackedInterner<u16, u16>>,
    /// Names of modules
    pub modules: TypedInterner<ModuleId, PackedInterner<u8, u8>>,
    /// Literal strings (code blocks, variable values)
    pub literals: TypedInterner<LiteralId, LooseInterner<u8, u16>>,
    /// Keep track of which branch is baseline for each branchpoint
    pub baselines: BaselineBranches,
    /// Strings used while running workflow: full file paths, debug strings etc.
    // this was observed to be 18/994, later 54/3436.
    pub run: TypedInterner<RunStrId, PackedInterner<u32, usize>>,
}

impl Default for WorkflowStrings {
    fn default() -> Self {
        let mut idents = PackedInterner::with_capacity_and_avg_len(64, 1024);
        // seed idents with an empty value, so we can use 0 as a special val:
        // TODO if we just start using branch masks instead of ident ids, we won't need this.
        idents.intern("").expect("Ident interner seeding should never fail");

        Self {
            branchpoints: TypedInterner::new(PackedInterner::with_capacity_and_str_len(8, 32)),
            tasks: TypedInterner::new(PackedInterner::with_capacity_and_str_len(16, 256)),
            idents: TypedInterner::new(idents),
            literals: TypedInterner::new(LooseInterner::with_capacity_and_str_len(64, 4096)),
            modules: TypedInterner::new(PackedInterner::with_capacity_and_str_len(8, 16)),
            baselines: BaselineBranches::with_capacity(8),
            // we'll re-alloc these later when we need them:
            run: TypedInterner::new(PackedInterner::with_capacity_and_str_len(0, 0)),
        }
    }
}

impl WorkflowStrings {
    /// Allocate space for new strings created during traversal:
    pub fn alloc_for_traversal(&mut self) {
        // placeholder
    }

    /// Since we don't allocate any space for runtime strings up front,
    /// call this fn to get ready to actually run the workflow.
    pub fn alloc_for_run(&mut self) {
        self.run = TypedInterner::new(PackedInterner::with_capacity_and_str_len(64, 4096));
    }

    /// Create a value from its ast representation.
    #[inline]
    pub fn create_value(&mut self, lhs: ast::Ident, rhs: ast::Rhs) -> Result<Value> {
        create_value(self, lhs, rhs)
    }

    /// Used while loading branchpoints.txt to make sure our branchpoints are
    /// ordered consistently, and baselines stay consistent between runs.
    pub fn pre_load_baseline(&mut self, branchpoint: &str, branchval: &str) -> Result<()> {
        let k = self.branchpoints.intern(branchpoint)?;
        let v = self.idents.intern(branchval)?;
        self.baselines.add(k, v);
        Ok(())
    }

    pub fn add_branchpoint(&mut self, branchpoint: &str) -> Result<BranchpointId> {
        self.branchpoints.intern(branchpoint)
    }

    pub fn add_branch(
        &mut self,
        _branchpoint: BranchpointId,
        branch_name: &str,
    ) -> Result<IdentId> {
        self.idents.intern(branch_name)
    }

    pub fn log_sizes(&self) {
        self.log_sizes_for("Branchpoints", &self.branchpoints);
        self.log_sizes_for("Tasks", &self.tasks);
        self.log_sizes_for("Idents", &self.idents);
        self.log_sizes_for("Modules", &self.modules);
        self.log_sizes_for("Literals", &self.literals);
    }

    fn log_sizes_for<T: GetStr>(&self, name: &str, interner: &T) {
        log::debug!(
            "{} {}, str len {}",
            interner.len(),
            name,
            interner.str_len()
        );
    }
}

// string interpolation /////////////////////
impl WorkflowStrings {
    /// Realize an interpolated string into `strbuf`.
    pub fn make_interpolated(
        &self,
        orig: LiteralId,
        // NB these must be in order of where they appear in the string!
        vars: &[(IdentId, LiteralId)],
        strbuf: &mut String,
    ) -> Result<()> {
        let orig_str = self.literals.get(orig)?;
        strbuf.push_str(orig_str);

        let mut var_str = String::with_capacity(16);
        var_str.push('$');

        // keep moving scan start fwd so we don't accidentally mess up
        // work we already did...
        let mut scan_start = 0;
        for (ident, val) in vars {
            var_str.truncate(1);
            let ident_str = self.idents.get(*ident)?;
            var_str.push_str(ident_str);

            let val_str = self.literals.get(*val)?;

            if let Some(offset) = strbuf[scan_start..].find(&var_str) {
                let start = scan_start + offset;
                let end = start + var_str.len();
                strbuf.replace_range(start..end, val_str);
                scan_start = start + val_str.len();
            } else {
                return Err(Error::Interp(var_str, strbuf.clone()).into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_interpolate() -> Result<()> {
        let mut strings = WorkflowStrings::default();
        let orig_id = strings.literals.intern("$v1 and $v2 $v1-$v2.$v2 etc");
        let v1 = strings.idents.intern("v1")?;
        let v2 = strings.idents.intern("v2")?;
        let v1_val = strings.literals.intern("value for var one")?;
        let v2_val = strings.literals.intern("$$xyz$$")?;

        let mut buf = String::with_capacity(32);
        buf.push_str("prefix.");

        let vars = &[
            (v1, v1_val),
            (v2, v2_val),
            (v1, v1_val),
            (v2, v2_val),
            (v2, v2_val),
        ];

        strings.make_interpolated(orig_id, vars, &mut buf)?;

        assert_eq!(
            &buf,
            "prefix.value for var one and $$xyz$$ value for var one-$$xyz$$.$$xyz$$ etc"
        );

        // now try a bad one:
        let v3 = strings.idents.intern("v3_wont_be_found")?;
        let res = strings.make_interpolated(orig_id, &[(v3, v1_val)], &mut buf);
        assert!(res.is_err());

        Ok(())
    }
}
