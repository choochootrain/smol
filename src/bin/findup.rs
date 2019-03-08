extern crate exitcode;

use std::env;
use std::path::{Path, PathBuf};
use std::result::Result;

use smol::errors::{SmolError};


fn help(args: Vec<String>) -> SmolError {
    println!(
        "usage: {} NAME
    Find first file or directory named NAME in current or nearest ancestor's directory.",
        args[0]
    );

    SmolError(exitcode::USAGE, None)
}

fn findup(name: &String) -> Result<(), SmolError> {
    let root = Path::new("/");
    let path: &Path = Path::new(name);

    while !path.exists() {
        let current_dir: PathBuf = env::current_dir().map_err(|e| {
            SmolError::from_err(exitcode::OSFILE, &e, "Could not get current directory")
        })?;

        if current_dir == root {
            return SmolError(exitcode::OSFILE, None).into();
        }

        let parent_dir = current_dir.parent().ok_or(SmolError(
            exitcode::OSFILE,
            Some(format!("Could not get parent of {}", current_dir.display())),
        ))?;

        env::set_current_dir(parent_dir).map_err(|e| {
            SmolError::from_err(
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
        SmolError::from_err(
            exitcode::OSFILE,
            &e,
            format!("Could not canonicalize path {}", path.display()),
        )
    })?;
    println!("{}", canonical_path.display());

    Ok(())
}

fn run(args: Vec<String>) -> Result<(), SmolError> {
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
        Err(SmolError(code, Some(message))) => {
            println!("{}", message);
            ::std::process::exit(code);
        }
        Err(SmolError(code, _)) => ::std::process::exit(code),
    }
}
