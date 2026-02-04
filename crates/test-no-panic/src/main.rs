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
        .arg("--color")
        .output()
        .expect("failed to run `cargo asm`");

    if !output.status.success() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        return ExitCode::FAILURE;
    }

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
            let stripped_line = anstream::adapter::strip_str(line).to_string();

            if let Some(col) = stripped_line.find(sus_word) {
                sus.push((row, [col, col + sus_word.len()], line))
            }
        }
    }

    if sus.is_empty() {
        return ExitCode::SUCCESS;
    }

    let file = "out/everything.asm";

    for &(row, [start, end], line) in &sus {
        let col = start + 1;
        let start_pad = " ".repeat(start);
        let underline = "^".repeat(end - start);
        let style = anstyle::Style::new()
            .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightRed)))
            .bold();

        eprintln!(
            "\
 --> {file}:{row}:{col}
   |
   | {line}
   | {start_pad}{style}{underline}{style:#}
   |
"
        );
    }

    eprintln!("Found {} suspicious locations.", sus.len());

    let stripped_asm = anstream::adapter::strip_str(&asm).to_string();
    fs::write(file, stripped_asm).expect("failed to write asm to file");

    ExitCode::FAILURE
}
