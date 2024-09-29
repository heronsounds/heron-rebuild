/// Utility for building the contents of a `task.sh` script file.
/// Note that it modifies a String reference held internally;
/// read that String to get the script's contents.
#[derive(Debug)]
pub struct TaskScriptBuilder<'a> {
    strbuf: &'a mut String,
}

impl<'a> TaskScriptBuilder<'a> {
    pub fn new(strbuf: &'a mut String) -> Self {
        Self { strbuf }
    }
}

impl TaskScriptBuilder<'_> {
    /// shebang line and bash option
    pub fn write_prefix(&mut self) {
        self.strbuf.clear();
        self.strbuf.push_str("#!/usr/bin/env bash\nset -xeuo pipefail\n\n");
    }

    /// a single variable assignment
    pub fn write_assignment_line(&mut self, var_name: &str, var_val: &str) {
        self.strbuf.push_str(var_name);
        self.strbuf.push('=');
        if var_val.is_empty() {
            self.strbuf.push_str("\"\"");
        } else {
            self.strbuf.push_str(var_val);
        }
        self.strbuf.push('\n');
    }

    /// cd to module directory, execute code, copy outputs back to realization dir, and exit.
    pub fn write_module_task_suffix(
        &mut self,
        code: &str,
        module_dir: &str,
        src: &[&str],
        tgt: &[&str],
    ) {
        self.write_cd_to_module(module_dir);
        self.write_code(code);
        self.write_copy_module_files(src, tgt);
        self.write_exit();
    }

    /// execute code and exit.
    pub fn write_normal_task_suffix(&mut self, code: &str) {
        self.write_code(code);
        self.write_exit();
    }

    fn write_cd_to_module(&mut self, module_dir: &str) {
        self.strbuf.push_str(
            "\n# This is a module task, so we cd to the module directory before running it:\n",
        );
        self.strbuf.push_str("cd ");
        self.strbuf.push_str(module_dir);
        self.strbuf.push('\n');
    }

    fn write_code(&mut self, code: &str) {
        self.strbuf.push_str(code);
    }

    fn write_copy_module_files(&mut self, src: &[&str], tgt: &[&str]) {
        debug_assert!(src.len() == tgt.len());
        self.strbuf
            .push_str("\n# Copy all outputs in module directory back to artifacts directory:\n");
        for i in 0..src.len() {
            self.strbuf.push_str("cp -r ");
            self.strbuf.push_str(src[i]);
            self.strbuf.push(' ');
            self.strbuf.push_str(tgt[i]);
            self.strbuf.push('\n');
        }
    }

    fn write_exit(&mut self) {
        self.strbuf.push_str("\nexit 0\n");
    }
}
