//! Utility functions for dealing with the branchpoints.txt file.

use std::path::Path;

use anyhow::Result;

use intern::GetStr;
use workflow::{Workflow, BRANCH_KV_DELIM};

use crate::ui::Ui;

use super::{Error, Fs};

impl Fs {
    /// Load the contents of `branchpoints_file` into `wf`.
    pub fn load_branches(
        &self,
        branchpoints_file: &Path,
        wf: &mut Workflow,
        strbuf: &mut String,
        ui: &Ui,
    ) -> Result<()> {
        read_branchpoints_file(self, branchpoints_file, strbuf, wf, ui)
    }

    /// Write info from `wf` into `branchpoints_file`.
    pub fn write_branches(
        &self,
        branchpoints_file: &Path,
        wf: &Workflow,
        strbuf: &mut String,
    ) -> Result<()> {
        write_branchpoints_file(self, branchpoints_file, strbuf, wf)
    }
}

/// Load branchpoints from branchpoints.txt into the workflow.
fn read_branchpoints_file(
    fs: &Fs,
    branchpoints_file: &Path,
    strbuf: &mut String,
    wf: &mut Workflow,
    ui: &Ui,
) -> Result<()> {
    ui.verbose_progress("Reading branchpoints.txt file");
    if fs.exists(branchpoints_file) {
        fs.read_to_buf(branchpoints_file, strbuf)?;
        for kv in strbuf.split_whitespace() {
            if let Some((k, v)) = kv.split_once(BRANCH_KV_DELIM) {
                wf.strings.pre_load_baseline(k, v);
            } else {
                return Err(Error::InvalidBranchpointsFile.into());
            }
        }
        ui.done();
    } else {
        ui.verbose_msg("\nNo branchpoints.txt file. Continuing.");
    }
    Ok(())
}

/// Write branchpoints from the workflow to branchpoints.txt.
fn write_branchpoints_file(
    fs: &Fs,
    branchpoints_file: &Path,
    strbuf: &mut String,
    wf: &Workflow,
) -> Result<()> {
    if fs.exists(branchpoints_file) {
        // TODO save a backup in case the app crashes here...
        fs.delete_file(branchpoints_file)?;
    }
    strbuf.clear();
    for (k, v) in wf.strings.baselines.iter() {
        let branchpt = wf.strings.branchpoints.get(k.into());
        let branchval = wf.strings.idents.get(*v);
        strbuf.push_str(branchpt);
        strbuf.push(BRANCH_KV_DELIM);
        strbuf.push_str(branchval);
        strbuf.push('\n');
    }
    fs.write_file(branchpoints_file, strbuf)?;
    Ok(())
}
