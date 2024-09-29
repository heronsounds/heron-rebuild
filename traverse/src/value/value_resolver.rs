use anyhow::Result;

use intern::GetStr;
use util::Bitmask;
use workflow::{BaseValue, BranchSpec, DirectValue, IdentId, Value, Workflow, NULL_IDENT};

use super::{BranchMasks, Error, RealValueLike};

/// Just a convenience to keep Bfs impls from growing too large.
#[derive(Debug)]
pub struct ValueResolver;

impl ValueResolver {
    /// Resolve the given `Value` for use in a task realized by `branch`.
    pub fn resolve<T, B>(
        &self,
        value: &Value,
        branch: &BranchSpec,
        wf: &Workflow,
    ) -> Result<(T, BranchMasks<B>)>
    where
        T: RealValueLike,
        B: Bitmask,
    {
        match value {
            Value::Direct(v) => self.resolve_direct(v, branch, wf),
            Value::Branched(vals) => {
                for (val_branch, val) in vals {
                    if val_branch.is_compatible(branch) {
                        let (mut real_val, mut masks) =
                            self.resolve_direct::<T, B>(val, branch, wf)?;
                        masks.add |= val_branch.as_mask::<B>()?;
                        real_val.update_branch(val_branch);
                        return Ok((real_val, masks));
                    }
                }
                Err(Error::BranchNotFound.into())
            }
        }
    }

    fn resolve_direct<T, B>(
        &self,
        value: &DirectValue,
        branch: &BranchSpec,
        wf: &Workflow,
    ) -> Result<(T, BranchMasks<B>)>
    where
        T: RealValueLike,
        B: Bitmask,
    {
        match value {
            DirectValue::Simple(v) => self.resolve_base(v, branch, wf),
            DirectValue::Graft(v, graft_branch) => {
                let mut new_branch = branch.clone();
                new_branch.insert_all(graft_branch);
                let (real_val, mut masks) = self.resolve_base::<T, B>(v, &new_branch, wf)?;
                for (k, v) in graft_branch.iter().enumerate() {
                    if *v != NULL_IDENT {
                        masks.rm.set(k);
                    }
                }
                Ok((real_val, masks))
            }
        }
    }

    fn resolve_base<T, B>(
        &self,
        value: &BaseValue,
        branch: &BranchSpec,
        wf: &Workflow,
    ) -> Result<(T, BranchMasks<B>)>
    where
        T: RealValueLike,
        B: Bitmask,
    {
        use BaseValue::*;
        match value {
            Literal(v) => Ok((T::literal(*v)?, BranchMasks::default())),
            Task(abstract_task, v) => Ok((
                T::task(*abstract_task, *v, branch.clone())?,
                BranchMasks::default(),
            )),
            Config(v) => self.get_config_val_and_resolve(*v, branch, wf),
            Interp(v, vars) => {
                let mut outer_masks = BranchMasks::default();
                let mut var_literals = Vec::with_capacity(vars.len());
                for var in vars {
                    let (val, masks) = self.get_config_val_and_resolve::<T, B>(*var, branch, wf)?;
                    // so... we can't chain interp vars? hm.
                    // could simplify this by just sticking a value id in there instead.
                    // except, where does the value go? we can't store it anywhere from here.
                    // we can't even match on it anymore, since it's hidden by a type param... geez.
                    let var_lit_id = val.get_literal_id()?;
                    var_literals.push((*var, var_lit_id));
                    outer_masks.or_eq(&masks);
                }
                Ok((T::interp(*v, var_literals)?, outer_masks))
            }
        }
    }

    fn get_config_val_and_resolve<T, B>(
        &self,
        ident: IdentId,
        branch: &BranchSpec,
        wf: &Workflow,
    ) -> Result<(T, BranchMasks<B>)>
    where
        T: RealValueLike,
        B: Bitmask,
    {
        let val_id = wf.get_config_value(ident).ok_or_else(|| {
            let ident = wf
                .strings
                .idents
                .get(ident)
                .expect("Ident id should be interned at this point.");
            Error::UndefinedConfigValue(ident.to_owned())
        })?;
        let val = wf.get_value(val_id)?;
        self.resolve(val, branch, wf)
    }
}
