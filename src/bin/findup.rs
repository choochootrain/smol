use std::env;
use std::error::Error;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::result::Result;

extern crate exitcode;

#[derive(Debug)]
struct FindupError(exitcode::ExitCode, Option<String>);

impl FindupError {
    fn from_err<D>(code: exitcode::ExitCode, error: &Error, message: D) -> FindupError
    where
        D: fmt::Display,
    {
        FindupError(code, Some(format!("{}: {}", message, error)))
    }
}

impl From<io::Error> for FindupError {
    fn from(error: io::Error) -> Self {
        FindupError(exitcode::OSFILE, Some(format!("{}", error)))
    }
}

impl From<FindupError> for Result<(), FindupError> {
    fn from(error: FindupError) -> Self {
        Err(error)
    }
}

fn help(args: Vec<String>) -> FindupError {
    println!(
        "usage: {} NAME
    Find first file or directory named NAME in current or nearest ancestor's directory.",
        args[0]
    );

    FindupError(exitcode::USAGE, None)
}

fn findup(name: &String) -> Result<(), FindupError> {
    let root = Path::new("/");
    let path: &Path = Path::new(name);

    while !path.exists() {
        let current_dir: PathBuf = env::current_dir().map_err(|e| {
            FindupError::from_err(exitcode::OSFILE, &e, "Could not get current directory")
        })?;

        if current_dir == root {
            return FindupError(exitcode::OSFILE, None).into();
        }

        let parent_dir = current_dir.parent().ok_or(FindupError(
            exitcode::OSFILE,
            Some(format!("Could not get parent of {}", current_dir.display())),
        ))?;

        env::set_current_dir(parent_dir).map_err(|e| {
            FindupError::from_err(
                exitcode::OSFILE,
                &e,
                format!(
                    "Could not set current directory to {}",
                    parent_dir.display()
                ),
            )
        })?;
    }

    let canonical_path: PathBuf = path.canonicalize().map_err(|e| {
        FindupError::from_err(
            exitcode::OSFILE,
            &e,
            format!("Could not canonicalize path {}", path.display()),
        )
    })?;
    println!("{}", canonical_path.display());

    Ok(())
}

fn run(args: Vec<String>) -> Result<(), FindupError> {
    match args.len() {
        2 => {
            let name = &args[1];
            findup(name)
        }
        _ => help(args).into(),
    }
}

fn main() {
    match run(env::args().collect()) {
        Ok(_) => ::std::process::exit(exitcode::OK),
        Err(FindupError(code, Some(message))) => {
            println!("{}", message);
            ::std::process::exit(code);
        }
        Err(FindupError(code, _)) => ::std::process::exit(code),
    }
}
