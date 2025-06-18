use std::{collections::HashMap, env, ffi::OsString, fmt::Write, path::Path, process::Command};

use fast_glob::glob_match;
use markdown_tables::MarkdownTableRow;

use crate::schema::{BenchmarkSummary, EitherOrBothForUint64};

mod schema;

const BENCH_NAME: &str = "bench";
const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

fn left(metric: &EitherOrBothForUint64) -> Option<u64> {
    match *metric {
        EitherOrBothForUint64::Left(left) => Some(left),
        EitherOrBothForUint64::Both(left, _) => Some(left),
        EitherOrBothForUint64::Right(_) => None,
    }
}

#[derive(Debug)]
struct Report {
    instructions: u64,
    branches: u64,
    branch_predictor_misses: u64,
}

fn read_summary(path: &Path) -> Report {
    let summary = std::fs::read_to_string(path).expect("missing summary.json");
    let summary = serde_json::from_str::<BenchmarkSummary>(&summary).expect("failed to parse summary.json");

    let total = summary.callgrind_summary.unwrap().callgrind_run.total.summary;

    let mut ir = None;
    let mut bc = None;
    let mut bcm = None;

    for (kind, diff) in &total.0 {
        match kind.as_str() {
            "Ir" => {
                ir = left(&diff.metrics);
            }
            "Bc" => {
                bc = left(&diff.metrics);
            }
            "Bcm" => {
                bcm = left(&diff.metrics);
            }
            _ => (),
        }
    }

    Report {
        instructions: ir.unwrap_or(u64::MAX),
        branches: bc.unwrap_or(u64::MAX),
        branch_predictor_misses: bcm.unwrap_or(u64::MAX),
    }
}

struct Row(Vec<String>);

impl MarkdownTableRow for Row {
    fn column_names() -> Vec<&'static str> {
        vec!["name", "bump-scope (up)", "bump-scope (down)", "bumpalo", "blink-alloc"]
    }

    fn column_values(&self) -> Vec<String> {
        self.0.clone()
    }
}

const GROUP_NAMES: &[&str] = &[
    "alloc_u8",
    //
    "alloc_u32",
    "alloc_u32_aligned",
    "try_alloc_u32",
    "try_alloc_u32_aligned",
    //
    "alloc_big_struct",
    "alloc_big_struct_aligned",
    "try_alloc_big_struct",
    "try_alloc_big_struct_aligned",
    //
    "alloc_slice_copy",
    "alloc_slice_copy_aligned",
    "try_alloc_slice_copy",
    "try_alloc_slice_copy_aligned",
    //
    "allocate",
    "grow_same_align",
    "grow_smaller_align",
    "grow_larger_align",
    "shrink_same_align",
    "shrink_smaller_align",
    "shrink_larger_align",
    "deallocate",
    "deallocate_non_last",
    //
    "black_box_allocate",
    "black_box_grow_same_align",
    "black_box_grow_smaller_align",
    "black_box_grow_larger_align",
    "black_box_shrink_same_align",
    "black_box_shrink_smaller_align",
    "black_box_shrink_larger_align",
    "black_box_deallocate",
    "black_box_deallocate_non_last",
    //
    "warm_up",
    "reset",
];

const LIBRARY_NAMES: &[&str] = &["bump_scope_up", "bump_scope_down", "bumpalo", "blink_alloc"];

const INVALID: &[&str] = &[
    // These particular cases generally result in a `0` instruction count anyway due to function deduplication i assume.
    // However the `alloc_u32_aligned` for some reason didn't which messed with the `try_` prefix merging.
    // The results of these cases is not interesting anyway because it's the as for the non-`_aligned` cases.
    "*_aligned/blink_alloc",
];

const FOOTNOTES_GROUP: &[(&str, usize)] = &[("*shrink*", 2)];
const FOOTNOTES_LIBRARY: &[(&str, usize)] = &[("*_aligned/blink_alloc", 1)];

const TABLE_SECTIONS: &[(&str, &[&str])] = &[
    ("alloc", &["alloc_*"]),
    ("try alloc", &["try_alloc_*"]),
    ("allocator_api", &["allocate*", "grow*", "shrink*", "deallocate*"]),
    (
        "black_box_allocator_api",
        &[
            "black_box_allocate*",
            "black_box_grow*",
            "black_box_shrink*",
            "black_box_deallocate*",
        ],
    ),
    ("misc", &["warm_up", "reset"]),
];

fn replace_section(readme: &str, section_name: &str, new_content: &str) -> String {
    let start_marker = format!("<!-- {section_name} start -->");
    let end_marker = format!("<!-- {section_name} end -->");

    let start_index = readme.find(&start_marker).unwrap() + start_marker.len();
    let end_index = readme[start_index..].find(&end_marker).unwrap() + start_index;

    let before = &readme[..start_index];
    let after = &readme[end_index..];

    format!("{before}{new_content}{after}")
}

fn rows() -> Vec<Vec<String>> {
    let mut rows = vec![];

    for &group in GROUP_NAMES {
        let mut group_label = group.to_string();

        for (glob, i) in FOOTNOTES_GROUP {
            if glob_match(glob, group) {
                group_label.write_fmt(format_args!(" [^{i}]")).unwrap();
            }
        }

        let mut row = vec![group_label];

        for &library in LIBRARY_NAMES {
            let path = format!("target/iai/{PACKAGE_NAME}/{BENCH_NAME}/{group}/{library}/summary.json");
            let Report {
                instructions,
                branches,
                branch_predictor_misses,
            } = read_summary(path.as_ref());

            let group_and_library = format!("{group}/{library}");

            let mut cell = if (instructions == 0 && branches == 0) || globs_match(INVALID, &group_and_library) {
                "—".to_string()
            } else {
                format!("{instructions} / {branches} / {branch_predictor_misses}")
            };

            for (glob, i) in FOOTNOTES_LIBRARY {
                if glob_match(glob, &group_and_library) {
                    cell.write_fmt(format_args!(" [^{i}]")).unwrap();
                }
            }

            row.push(cell);
        }

        rows.push(row);
    }

    rows
}

fn globs_match(globs: &[&str], path: &str) -> bool {
    for glob in globs {
        if glob_match(glob, path) {
            return true;
        }
    }

    false
}

#[expect(dead_code)]
// merge `try_`-prefix cases with non-prefixed if the result is the same
fn merge_try_prefixed(rows: &mut Vec<Vec<String>>) {
    #![allow(clippy::collapsible_if)]
    for i in (0..rows.len()).rev() {
        if let Some(unprefixed_name) = rows[i][0].strip_prefix("try_") {
            if let Some(unprefixed_i) = rows.iter().position(|row| row[0] == unprefixed_name) {
                if rows[i][1..].iter().eq(&rows[unprefixed_i][1..]) {
                    rows[unprefixed_i][0] = format!("(try_) {unprefixed_name}");
                    rows.remove(i);
                }
            }
        }
    }
}

// code from https://github.com/djc/rustc-version-rs
fn rustc_version() -> HashMap<String, String> {
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"));

    let mut cmd = if let Some(wrapper) = env::var_os("RUSTC_WRAPPER").filter(|w| !w.is_empty()) {
        let mut cmd = Command::new(wrapper);
        cmd.arg(rustc);
        cmd
    } else {
        Command::new(rustc)
    };

    let out = cmd.arg("-vV").output().expect("failed to execute `rustc -vV`");

    if !out.status.success() {
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        panic!("`rustc -vV` failed, stdout={stdout} stderr={stderr}");
    }

    let verbose_version_string = str::from_utf8(&out.stdout).expect("`rustc -vV` did not return utf8");

    let mut map = HashMap::new();

    for (i, line) in verbose_version_string.lines().enumerate() {
        if i == 0 {
            map.insert("short".to_string(), line.to_string());
            continue;
        }

        if let Some((key, value)) = line.split_once(": ") {
            map.insert(key.to_string(), value.to_string());
        }
    }

    map
}

fn main() {
    let mut readme = std::fs::read_to_string("README.md").unwrap();

    let all_rows = rows();

    for (section, section_globs) in TABLE_SECTIONS {
        let mut rows = vec![];

        for row in &all_rows {
            if globs_match(section_globs, &row[0]) {
                rows.push(row.clone());
            }
        }

        // merge_try_prefixed(&mut rows);
        let table = markdown_tables::as_table(&rows.into_iter().map(Row).collect::<Vec<_>>());
        readme = replace_section(&readme, &format!("{section} table"), &format!("\n\n{table}\n"));
    }

    // update compiler info
    {
        let version = rustc_version();
        let rustc = &version["short"];
        let host = &version["host"];
        let llvm = version.get("LLVM version");

        let mut s = String::new();
        write!(s, "`{rustc}` on `{host}`").unwrap();

        if let Some(llvm) = llvm {
            write!(s, " using `LLVM version {llvm}`").unwrap();
        }

        readme = replace_section(&readme, "version", &s);
    }

    std::fs::write("README.md", readme).unwrap();
}
