use std::{
    env, fs,
    process::{Command, ExitCode},
};

fn main() -> ExitCode {
    // cd into this package's directory
    env::set_current_dir(env!("CARGO_MANIFEST_DIR")).expect("failed to set current dir");

    // Output asm using `cargo-show-asm`.
    let output = Command::new("cargo")
        .arg("asm")
        .arg("--lib")
        .arg("--everything")
        .arg("--simplify")
        .output()
        .expect("failed to run `cargo asm`");

    assert!(output.status.success());

    let asm = String::from_utf8(output.stdout).expect("asm is not utf8");

    // We're gonna be looking for the string "panic" and since our crate name
    // contains this string we remove the crate name from the asm.
    let asm = asm.replace("test_no_panic", "$crate");

    // Makes aligning error markers easier.
    let asm = asm.replace('\t', "    ");

    let sus_words = ["panic"];
    let mut sus = vec![];

    for (line, row) in asm.lines().zip(1..) {
        for sus_word in sus_words {
            if let Some(col) = line.find(sus_word) {
                sus.push((row, [col, col + sus_word.len()], line))
            }
        }
    }

    if sus.is_empty() {
        return ExitCode::SUCCESS;
    }

    let file = "out/everything.asm";

    for &(row, [start, end], line) in &sus {
        let start_pad = " ".repeat(start);
        let underline = "^".repeat(end - start);

        eprintln!(
            "\
 --> {file}:{row}:{start}
   |
   | {line}
   | {start_pad}{underline}
   |
"
        );
    }

    eprintln!("Found {} suspicious locations.", sus.len());

    fs::write(file, asm).expect("failed to write asm to file");

    ExitCode::FAILURE
}
