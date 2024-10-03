//! Ids for use in typed collections.

// we use this IdentId to mark that a branchpoint should use the baseline branch,
// since branch names are stored in the idents table and branches are specified
// with IdentIds.
// In the future, we could try using NonZeroU* and maybe some Options to
// accomplish the same thing; we'd still need to make sure that a branch
// name never ends up with id 0 though.
pub const NULL_IDENT: IdentId = IdentId(0);

macro_rules! id {
    ($name:ident, $ty:ty) => {
        #[derive(Debug, Default, Clone, Copy, Hash, PartialEq, Eq)]
        pub struct $name($ty);

        impl From<$name> for usize {
            fn from(id: $name) -> usize {
                id.0 as usize
            }
        }

        impl From<usize> for $name {
            fn from(val: usize) -> $name {
                Self(val as $ty)
            }
        }

        impl From<$name> for $ty {
            fn from(id: $name) -> $ty {
                id.0
            }
        }

        impl From<$ty> for $name {
            fn from(val: $ty) -> $name {
                Self(val)
            }
        }
    };
}

id!(ModuleId, u8);
id!(BranchpointId, u8);
id!(IdentId, u16);
id!(LiteralId, u8);
id!(AbstractTaskId, u8);
id!(AbstractValueId, u16);

id!(RealTaskId, u16);
id!(RealValueId, u16);

id!(RunStrId, u32);
