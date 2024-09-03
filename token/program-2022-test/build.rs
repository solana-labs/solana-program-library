extern crate walkdir;

use {
    std::{env, path::Path, process::Command},
    walkdir::WalkDir,
};

fn rerun_if_changed(directory: &Path) {
    let src = directory.join("src");
    let files_in_src: Vec<_> = WalkDir::new(src)
        .into_iter()
        .map(|entry| entry.unwrap())
        .filter(|entry| {
            if !entry.file_type().is_file() {
                return false;
            }
            true
        })
        .map(|f| f.path().to_str().unwrap().to_owned())
        .collect();

    for file in files_in_src {
        if !Path::new(&file).is_file() {
            panic!("{} is not a file", file);
        }
        println!("cargo:rerun-if-changed={}", file);
    }
    let toml = directory.join("Cargo.toml").to_str().unwrap().to_owned();
    println!("cargo:rerun-if-changed={}", toml);
}

fn main() {
    let cwd = env::current_dir().expect("Unable to get current working directory");

    let spl_token_2022_dir = cwd
        .parent()
        .expect("Unable to get parent directory of current working dir")
        .join("program-2022");
    rerun_if_changed(&spl_token_2022_dir);

    let instruction_padding_dir = cwd
        .parent()
        .expect("Unable to get parent directory of current working dir")
        .parent()
        .expect("Unable to get grandparent directory of current working dir")
        .join("instruction-padding")
        .join("program");
    rerun_if_changed(&instruction_padding_dir);

    println!("cargo:rerun-if-changed=build.rs");

    for program_dir in [spl_token_2022_dir, instruction_padding_dir] {
        let program_toml = program_dir.join("Cargo.toml");
        let program_toml = format!("{}", program_toml.display());
        let args = vec!["build-sbf", "--manifest-path", &program_toml];
        let output = Command::new("cargo")
            .args(&args)
            .output()
            .expect("Error running cargo build-sbf");
        if let Ok(output_str) = std::str::from_utf8(&output.stdout) {
            let subs = output_str.split('\n');
            for sub in subs {
                println!("cargo:warning=(not a warning) {}", sub);
            }
        }
    }
}
