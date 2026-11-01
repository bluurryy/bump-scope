use std::{
    env, fs,
    process::{Command, ExitCode},
};

fn main() -> ExitCode {
    // cd into this package's directory
    env::set_current_dir(env!("CARGO_MANIFEST_DIR")).expect("failed to set current dir");

    let lib = "test_no_panic";
    let allowlist = [
        // used for default constructing the base allocator for const constructibles,
        // only gets called when the default constructor were to panic
        "panic_cannot_unwind",
    ];
    let blocklist = ["panic"];

    for name in function_names() {
        for dir in ["upwards", "downwards"] {
            let Ok(asm) = compile_asm(&format!("{lib}::{dir}::{name}")) else {
                return ExitCode::FAILURE;
            };

            let mut lines = vec![];
            let mut blocked = false;

            for line in asm.lines() {
                let line = line.replace(lib, "$crate");
                let stripped_line = anstream::adapter::strip_str(&line).to_string();

                let is_allowed = allowlist.iter().any(|s| stripped_line.contains(s))
                    || !blocklist.iter().any(|s| stripped_line.contains(s));

                if is_allowed {
                    lines.push(line);
                } else {
                    const STYLE: anstyle::Style = anstyle::Style::new()
                        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightRed)))
                        .bold();

                    lines.push(format!("{STYLE}{stripped_line}{STYLE:#}"));
                    blocked = true;
                }
            }

            if blocked {
                let output = lines.join("\n");
                eprintln!("{output}");
                eprintln!();
                print_error(&format!("panic in {dir}::{name}"));
                return ExitCode::FAILURE;
            }

            eprintln!("{dir}::{name}");
        }
    }

    ExitCode::SUCCESS
}

fn function_names() -> Vec<String> {
    let source = fs::read_to_string("src/lib.rs").unwrap();
    let start = source.find("up_and_down! {").unwrap();
    let region = &source[start..];
    let mut function_names = Vec::new();

    for line in region.lines() {
        if let Some(i) = line.find("fn ") {
            let start = i + 3;
            let end = line[start..]
                .char_indices()
                .find_map(|(i, c)| matches!(c, '(' | '<').then_some(i))
                .unwrap();
            let name = &line[start..start + end];
            function_names.push(name.to_string());
        }
    }

    function_names
}

fn compile_asm(name: &str) -> Result<String, ()> {
    // Output asm using `cargo-show-asm`.
    let output = Command::new("cargo")
        .arg("+nightly")
        .arg("asm")
        .arg("--quiet")
        .arg("--lib")
        .arg("--simplify")
        .arg("--color")
        .arg("--context=1000")
        .arg(name)
        .arg("0")
        .output()
        .expect("can't execute `cargo`");

    if !output.status.success() {
        print_error(&format!("failed to generate asm for {name}"));
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        return Err(());
    }

    match String::from_utf8(output.stdout) {
        Ok(string) => Ok(string),
        Err(err) => {
            print_error(&format!("asm output is not utf8: {err}"));
            Err(())
        }
    }
}

fn print_error(message: &str) {
    const LABEL: anstyle::Style = anstyle::Style::new()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightRed)))
        .bold();

    const MESSAGE: anstyle::Style = anstyle::Style::new().bold();

    eprintln!("{LABEL}ERROR:{LABEL:#} {MESSAGE}{message}{MESSAGE:#}")
}
