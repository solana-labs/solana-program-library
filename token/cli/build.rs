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

fn build_bpf(program_directory: &Path) {
    let toml_file = program_directory.join("Cargo.toml");
    let toml_file = format!("{}", toml_file.display());
    let args = vec!["build-sbf", "--manifest-path", &toml_file];
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

fn main() {
    let is_debug = env::var("DEBUG").map(|v| v == "true").unwrap_or(false);
    let build_dependent_programs = env::var("BUILD_DEPENDENT_PROGRAMS")
        .map(|v| v != "false" && v != "0")
        .unwrap_or(false);
    if is_debug && build_dependent_programs {
        let cwd = env::current_dir().expect("Unable to get current working directory");
        let spl_token_2022_dir = cwd
            .parent()
            .expect("Unable to get parent directory of current working dir")
            .join("program-2022");
        rerun_if_changed(&spl_token_2022_dir);
        let spl_token_dir = cwd
            .parent()
            .expect("Unable to get parent directory of current working dir")
            .join("program");
        rerun_if_changed(&spl_token_dir);
        let spl_associated_token_account_dir = cwd
            .parent()
            .expect("Unable to get parent directory of current working dir")
            .parent()
            .expect("Unable to get parent directory of current working dir")
            .join("associated-token-account")
            .join("program");
        rerun_if_changed(&spl_associated_token_account_dir);

        build_bpf(&spl_token_dir);
        build_bpf(&spl_token_2022_dir);
        build_bpf(&spl_associated_token_account_dir);
    }
    println!("cargo:rerun-if-changed=build.rs");
}
