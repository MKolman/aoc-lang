use std::{collections::HashMap, fs};

use crate::aoc::compile_and_run;

macro_rules! interpret_tests {
    ($($name:ident,)*) => {
    $(
        #[test]
        fn $name() {
            run_single_example(stringify!($name));
        }
    )*
    }
}

#[test]
fn test_examples() {
    for (code_file, out_file) in collect_examples() {
        run_and_compare(&code_file, &out_file);
    }
}

fn run_single_example(test_case: &str) {
    run_and_compare(
        &format!("./examples/{}.aoc", test_case),
        &format!("./examples/{}.out", test_case),
    )
}

fn run_and_compare(code_file: &str, out_file: &str) {
    println!("TESTING {} AND {}", code_file, out_file);
    let code = fs::read_to_string(code_file).expect("Invalid code file");
    let want = fs::read_to_string(out_file).expect("Invalid out file");
    let mut output = Vec::new();
    compile_and_run(&code, &mut output);
    // Do writing here.
    assert_eq!(
        String::from_utf8_lossy(&output),
        want,
        "\n\tInvalid result for {} in {}",
        code_file,
        out_file,
    );
}

fn collect_examples() -> Vec<(String, String)> {
    let mut result: HashMap<String, (String, String)> = HashMap::new();
    for file in fs::read_dir("./examples").expect("Example folder doesn't exist.") {
        let path = file.expect("Cannot detect file").path();
        let fname = path.to_str().expect("Invalid path");
        match (fname.strip_suffix(".aoc"), fname.strip_suffix(".out")) {
            (Some(name), _) => result.entry(name.to_string()).or_default().0 = fname.to_string(),
            (_, Some(name)) => result.entry(name.to_string()).or_default().1 = fname.to_string(),
            _ => panic!("Invalid file extension for {}", fname),
        }
    }
    result.values().cloned().collect()
}

interpret_tests! {
    primes,
    sort,
    dfs,
}
