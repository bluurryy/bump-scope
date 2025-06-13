use std::{fmt::Write, path::Path};

use glob_match::glob_match;
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
}

fn read_summary(path: &Path) -> Report {
    let summary = std::fs::read_to_string(path).expect("missing summary.json");
    let summary = serde_json::from_str::<BenchmarkSummary>(&summary).expect("failed to parse summary.json");

    let total = summary.callgrind_summary.unwrap().callgrind_run.total.summary;

    let mut ir = None;
    let mut bc = None;

    for (kind, diff) in &total.0 {
        match kind.as_str() {
            "Ir" => {
                ir = left(&diff.metrics);
            }
            "Bc" => {
                bc = left(&diff.metrics);
            }
            _ => (),
        }
    }

    Report {
        instructions: ir.unwrap_or(u64::MAX),
        branches: bc.unwrap_or(u64::MAX),
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
    "alloc_u32",
    "alloc_u32_aligned",
    "try_alloc_u32",
    "try_alloc_u32_aligned",
    "allocate",
    "grow_same_align",
    "grow_smaller_align",
    "grow_larger_align",
    "shrink_same_align",
    "shrink_smaller_align",
    "shrink_larger_align",
    "deallocate",
    "deallocate_non_last",
    "warm_up",
    "reset",
];

const LIBRARY_NAMES: &[&str] = &["bump_scope_up", "bump_scope_down", "bumpalo", "blink_alloc"];

const FOOTNOTES_GROUP: &[(&str, usize)] = &[("shrink_*", 2)];
const FOOTNOTES_LIBRARY: &[(&str, usize)] = &[("*_aligned/blink_alloc", 1)];

const SECTIONS: &[(&str, &[&str])] = &[
    ("alloc", &["*alloc_*"]),
    ("allocator_api", &["allocate*", "grow*", "shrink*", "deallocate*"]),
    ("misc", &["warm_up", "reset"]),
];

fn patch_readme(section: &str, table: &str) {
    let readme = std::fs::read_to_string("README.md").unwrap();

    let start_marker = format!("<!-- {section} table start -->");
    let end_marker = format!("<!-- {section} table end -->");

    let start_index = readme.find(&start_marker).unwrap() + start_marker.len();
    let end_index = readme[start_index..].find(&end_marker).unwrap() + start_index;

    let before = &readme[..start_index];
    let after = &readme[end_index..];

    let new_readme = format!("{before}\n\n{table}\n\n{after}");
    std::fs::write("README.md", new_readme).unwrap();
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
            let Report { instructions, branches } = read_summary(path.as_ref());

            let mut cell = if instructions == 0 && branches == 0 {
                "—".to_string()
            } else {
                format!("{instructions} / {branches}")
            };

            let group_and_library = format!("{group}/{library}");

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

// merge `try_`-prefix cases with non-prefixed if the result is the same
fn merge_try_prefixed(rows: &mut Vec<Vec<String>>) {
    for i in (0..rows.len()).rev() {
        if let Some(unprefixed_name) = rows[i][0].strip_prefix("try_")
            && let Some(unprefixed_i) = rows.iter().position(|row| row[0] == unprefixed_name)
            && rows[i][1..].iter().eq(&rows[unprefixed_i][1..])
        {
            rows[unprefixed_i][0] = format!("(try_){unprefixed_name}");
            rows.remove(i);
        }
    }
}

fn main() {
    let mut all_rows = rows();
    merge_try_prefixed(&mut all_rows);

    for (section, section_globs) in SECTIONS {
        let mut rows = vec![];

        for row in &all_rows {
            if globs_match(section_globs, &row[0]) {
                rows.push(row.clone());
            }
        }

        merge_try_prefixed(&mut rows);
        let table = markdown_tables::as_table(&rows.into_iter().map(Row).collect::<Vec<_>>());
        println!("{section}: {table}");
        patch_readme(section, &table);
    }
}
