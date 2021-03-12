use std::fs::File;
use std::io::{stderr, stdout, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread;

use anyhow::{Context, Result};
use colored::Colorize;

use crate::fs::Fs;

/// Run a subprocess, storing stdout and stderr in the given `artifacts_dir`.
/// Based on:
/// <https://stackoverflow.com/questions/66060139/how-to-tee-stdout-stderr-from-a-subprocess-in-rust>
pub fn run_cmd(
    cmd: &mut Command,
    artifacts_dir: &str,
    fs: &mut Fs,
    pathbuf: &mut PathBuf,
    verbose: bool,
) -> Result<bool> {
    if verbose {
        eprintln!("{}", "Creating stdout and stderr files...".magenta());
    }

    let (out_file, err_file) = make_log_files(fs, artifacts_dir, pathbuf)?;

    if verbose {
        eprintln!("{}", "Running command...".magenta());
    }
    let mut cmd = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|_| {
            panic!(
                "failed to execute child process {:?} {:?}",
                cmd.get_program(),
                cmd.get_args(),
            )
        });

    let child_out = cmd.stdout.take().expect("Cannot attach to child stdout");
    let child_err = cmd.stderr.take().expect("Cannot attach to child stderr");

    let thread_out = thread::spawn(move || {
        communicate(child_out, out_file, stdout()).expect("error communicating with child stdout")
    });
    let thread_err = thread::spawn(move || {
        communicate(child_err, err_file, stderr()).expect("error communicating with child stderr")
    });

    thread_out.join().expect("Error joining stdout thread");
    thread_err.join().expect("Error joining stderr thread");

    let status = cmd.wait().expect("failed to wait on child process");

    if verbose {
        eprintln!("\n{} with {status}.", "Process finished".green());
    }
    Ok(status.success())
}

fn communicate<R: Read, W: Write>(
    mut stream: R,
    mut file: File,
    mut output: W,
) -> std::io::Result<()> {
    let mut buf = [0u8; 1024];
    loop {
        let num_read = stream.read(&mut buf)?;
        if num_read == 0 {
            break;
        }

        let buf = &buf[..num_read];
        file.write_all(buf)?;
        output.write_all(buf)?;
    }

    Ok(())
}

fn make_log_files(fs: &mut Fs, artifacts_dir: &str, pathbuf: &mut PathBuf) -> Result<(File, File)> {
    let out_file = fs
        .create_file(fs.stdout(artifacts_dir, pathbuf))
        .context("creating stdout.txt file")?;

    let err_file = fs
        .create_file(fs.stderr(artifacts_dir, pathbuf))
        .context("creating stderr.txt file")?;

    Ok((out_file, err_file))
}
