use std::env;
use std::path::{Path, PathBuf};

fn help(args: Vec<String>) {
    println!("usage: {} NAME
    Find first file or directory named NAME in current or nearest ancestor's directory.", args[0]);
    ::std::process::exit(1);
}

fn findup(name: &String) {
    let root = Path::new("/");
    let path: &Path = Path::new(name);

    while !path.exists() {
        let current_dir: PathBuf = env::current_dir()
            .expect("Could not get current directory");

        if current_dir == root {
            ::std::process::exit(1);
        }

        let parent_dir = current_dir
            .parent()
            .expect(&format!("Could not get parent of {}", current_dir.display()));

        env::set_current_dir(parent_dir)
            .expect(&format!("Could not set current directory to {}", parent_dir.display()));
    }

    let canonical_path: PathBuf = path
            .canonicalize()
            .expect(&format!("Could not canonicalize path {}", path.display()));
    println!("{}", canonical_path.display());
    ::std::process::exit(0);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.len() {
        2 => {
            let name = &args[1];
            findup(&name);
        },
        _ => help(args)
    }
}
