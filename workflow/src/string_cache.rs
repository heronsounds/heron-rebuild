use std::cell::{Ref, RefCell};

use anyhow::Result;

use util::{HashMap, Hasher};

use crate::WorkflowStrings;

pub trait StringMaker<T> {
    fn make_string(&self, val: &T, wf: &WorkflowStrings, buf: &mut String) -> Result<()>;
}

#[derive(Debug)]
pub struct StringCache<T, M, Idx = u16> {
    strings: RefCell<String>,
    idxs: RefCell<HashMap<T, (Idx, Idx)>>,
    maker: M,
}

impl<T, M, Idx> StringCache<T, M, Idx> {
    pub fn with_capacity_and_str_len(maker: M, cap: usize, str_len: usize) -> Self {
        Self {
            strings: RefCell::new(String::with_capacity(str_len)),
            idxs: RefCell::new(HashMap::with_capacity_and_hasher(cap, Hasher::default())),
            maker,
        }
    }
}

impl<T, M, Idx> StringCache<T, M, Idx>
where
    T: Clone + Eq + std::hash::Hash,
    M: StringMaker<T>,
    Idx: Copy + TryFrom<usize> + Into<usize>,
    Idx::Error: std::error::Error + Send + Sync + 'static,
{
    pub fn get_or_insert(&self, val: &T, wf: &WorkflowStrings) -> Result<Ref<str>> {
        if let Some((start, end)) = self.idxs.borrow().get(val).copied() {
            return Ok(self.get_substr(start.into(), end.into()));
        }
        let (start, end) = self.push_new_str(val, wf)?;
        self.idxs.borrow_mut().insert(val.clone(), (start.try_into()?, end.try_into()?));
        Ok(self.get_substr(start, end))
    }

    fn push_new_str(&self, val: &T, wf: &WorkflowStrings) -> Result<(usize, usize)> {
        let mut s = self.strings.borrow_mut();
        let start = s.len();
        self.maker.make_string(val, wf, &mut s)?;
        Ok((start, s.len()))
    }

    fn get_substr(&self, start: usize, end: usize) -> Ref<str> {
        let s = self.strings.borrow();
        Ref::map(s, |s| &s[start..end])
    }
}
