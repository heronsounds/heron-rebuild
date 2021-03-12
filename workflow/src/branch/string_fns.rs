//! Utility fns for dealing with branch strings (e.g. "Branchpoint.branch+X.y").

use anyhow::Result;

use intern::{GetStr, InternStr};

use crate::Workflow;
use crate::{BranchpointId, IdentId, BRANCH_DELIM, BRANCH_KV_DELIM};

use super::{BranchSpec, Error};

const BASELINE_STR: &str = "Baseline.baseline";
const BASELINE_STR_PLUS: &str = "Baseline.baseline+";

// TODO make a zero-size struct to hold these fns, add it to Workflow.

/// Branch string with all branches specified, even if they are baseline.
/// If there are no branches at all, uses "Baseline.baseline".
pub fn make_full_string(branch: &BranchSpec, wf: &Workflow, buf: &mut String) {
    let mut first = true;
    for (k, _) in wf.strings.baselines.iter() {
        if k >= branch.len() {
            break;
        }
        let k: BranchpointId = k.into();
        if let Some(v) = branch.get_specified(k) {
            if first {
                first = false;
            } else {
                buf.push(BRANCH_DELIM);
            }
            push_branch_pair(k, v, wf, buf);
        }
    }
    if buf.is_empty() {
        buf.push_str(BASELINE_STR);
    }
}

/// Branch string with only non-baseline branches specified.
/// If there are no branches, or if any branch is baseline,
/// Starts with "Baseline.baseline".
/// These strings will always stay valid between runs, as long
/// as the branch ordering doesn't change (specified in branchpoints.txt).
pub fn make_compact_string(branch: &BranchSpec, wf: &Workflow, buf: &mut String) {
    let mut first = true;
    let mut needs_baseline = false;
    for (k, baseline_v) in wf.strings.baselines.iter() {
        if k >= branch.len() {
            break;
        }
        let k: BranchpointId = k.into();
        if let Some(v) = branch.get_specified(k) {
            if v == *baseline_v {
                needs_baseline = true;
            } else {
                if first {
                    first = false;
                } else {
                    buf.push(BRANCH_DELIM);
                }
                push_branch_pair(k, v, wf, buf);
            }
        }
    }

    if buf.is_empty() {
        buf.insert_str(0, BASELINE_STR);
    } else if needs_baseline {
        buf.insert_str(0, BASELINE_STR_PLUS);
    }
}

fn push_branch_pair(k: BranchpointId, v: IdentId, wf: &Workflow, buf: &mut String) {
    buf.push_str(wf.strings.branchpoints.get(k));
    buf.push(BRANCH_KV_DELIM);
    buf.push_str(wf.strings.idents.get(v));
}

/// Parse a string of the kind created by `make_compact_string` into a `BranchSpec`.
pub fn parse_compact_branch_str(wf: &mut Workflow, s: &str) -> Result<BranchSpec> {
    let mut branch = BranchSpec::default();
    for kv in s.split(BRANCH_DELIM) {
        if kv != BASELINE_STR {
            if let Some((k, v)) = kv.split_once(BRANCH_KV_DELIM) {
                let k = wf.strings.branchpoints.intern(k);
                let v = wf.strings.idents.intern(v);
                branch.insert(k, v);
            } else {
                return Err(Error::InvalidBranchString(kv.to_owned()).into());
            }
        }
    }
    for (k, v) in wf.strings.baselines.iter() {
        let id: BranchpointId = k.into();
        if branch.is_unspecified(id) {
            branch.insert(id, *v);
        }
    }
    Ok(branch)
}
