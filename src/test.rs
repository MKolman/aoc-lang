use std::{collections::HashMap, fs};

use crate::runner::interpret;

#[test]
fn test_examples() {
    for (code_file, out_file) in collect_examples() {
        let code = fs::read_to_string(&code_file).expect("Invalid code file");
        let want = fs::read_to_string(&out_file).expect("Invalid out file");
        assert_eq!(
            interpret(&code),
            Ok(want),
            "\n\tInvalid result for {} in {}",
            code_file,
            out_file,
        );
    }
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
