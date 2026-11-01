use std::{
    env, fs,
    io::{self, Write},
    process::{Command, ExitCode},
};

fn main() -> ExitCode {
    let filter = env::args().nth(1);

    // cd into this package's directory
    env::set_current_dir(env!("CARGO_MANIFEST_DIR")).expect("failed to set current dir");

    let mut names = function_names();

    if let Some(filter) = filter.as_deref() {
        names.retain(|name| name.contains(filter));
    }

    let count = names.len();
    progress_bar(0, count, "");

    for (i, name) in names.into_iter().enumerate() {
        for dir in ["upwards", "downwards"] {
            if let Err(err) = check(dir, &name) {
                eprintln!();
                eprintln!("{err}");
                return ExitCode::FAILURE;
            };

            progress_bar(i + 1, count, &format!("{dir}::{name}"));
        }
    }

    progress_bar(count, count, "done!");
    ExitCode::SUCCESS
}

fn progress_bar(done: usize, total: usize, message: &str) {
    let width = 70;
    let filled = done * width / total;
    eprint!("\r\x1b[2K[{}{}] {message}", "█".repeat(filled), " ".repeat(width - filled));
    io::stderr().flush().unwrap();
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

fn check(dir: &str, name: &str) -> Result<(), String> {
    let lib = "test_no_panic";
    let allowlist = [
        // used for default constructing the base allocator for const constructibles,
        // only gets called when the default constructor were to panic
        "panic_cannot_unwind",
    ];
    let blocklist = ["panic"];

    let asm = compile_asm(&format!("test_no_panic::{dir}::{name}"))?;
    let mut lines = vec![];
    let mut blocked = false;

    for line in asm.lines() {
        let line = line.replace(lib, "$crate");
        let stripped_line = anstream::adapter::strip_str(&line).to_string();

        let is_allowed =
            allowlist.iter().any(|s| stripped_line.contains(s)) || !blocklist.iter().any(|s| stripped_line.contains(s));

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
        const FUNCTION: anstyle::Style = anstyle::Style::new()
            .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightGreen)))
            .bold();

        let output = lines.join("\n");
        let error = format_error("panic in");
        Err(format!("{output}\n\n{error} {FUNCTION}{dir}::{name}{FUNCTION:#}"))
    } else {
        Ok(())
    }
}

fn compile_asm(name: &str) -> Result<String, String> {
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
        let error = format_error(&format!("failed to generate asm for {name}"));
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{error}\n\n{stderr}"));
    }

    match String::from_utf8(output.stdout) {
        Ok(string) => Ok(string),
        Err(err) => Err(format_error(&format!("asm output is not utf8: {err}"))),
    }
}

fn format_error(message: &str) -> String {
    const LABEL: anstyle::Style = anstyle::Style::new()
        .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightRed)))
        .bold();

    const MESSAGE: anstyle::Style = anstyle::Style::new().bold();

    format!("{LABEL}ERROR:{LABEL:#} {MESSAGE}{message}{MESSAGE:#}")
}
