use std::{collections::HashMap, env, ffi::OsString, fmt::Write, path::Path, process::Command};

use cargo_metadata::{Metadata, MetadataCommand};
use fast_glob::glob_match;
use gungraun_summary::{
    either_or_both::EitherOrBoth,
    indexmap::IndexMap,
    v7::{EventKind, Metric, MetricResult, Tool, ToolMetricResults},
};
use markdown_tables::MarkdownTableRow;

const BENCH_NAME: &str = "bench";
const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Debug)]
struct Report {
    instructions: u64,
    branches: u64,
}

fn read_summary(path: &Path) -> Report {
    let summary = gungraun_summary::v7::parse(path).expect("can't parse gungraun summary");

    let metrics = summary
        .profiles
        .0
        .into_iter()
        .find(|p| p.tool == Tool::Callgrind)
        .expect("callgrind summary missing")
        .data
        .total
        .metrics;

    let metrics = match metrics {
        ToolMetricResults::Callgrind(metrics) => metrics.0,
        _ => panic!("callgrind summary has no callgrind metrics"),
    };

    Report {
        instructions: get_metric_int_or_max(&metrics, EventKind::Ir),
        branches: get_metric_int_or_max(&metrics, EventKind::Bc),
    }
}

fn get_metric_int_or_max(metrics: &IndexMap<EventKind, MetricResult>, event_kind: EventKind) -> u64 {
    get_metric_int(metrics, event_kind).unwrap_or(u64::MAX)
}

fn get_metric_int(metrics: &IndexMap<EventKind, MetricResult>, event_kind: EventKind) -> Option<u64> {
    let metric = metrics.get(&event_kind)?;

    let new_metric = match &metric.values {
        EitherOrBoth::Left(left) => left,
        EitherOrBoth::Both(left, _) => left,
        EitherOrBoth::Right(_) => return None,
    };

    match new_metric {
        Metric::Int(int) => Some(*int),
        Metric::Float(_) => None,
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
    "alloc_u8_overaligned",
    "try_alloc_u8",
    "try_alloc_u8_overaligned",
    //
    "alloc_u32",
    "alloc_u32_aligned",
    "alloc_u32_overaligned",
    "try_alloc_u32",
    "try_alloc_u32_aligned",
    "try_alloc_u32_overaligned",
    //
    "alloc_big_struct",
    "alloc_big_struct_aligned",
    "alloc_big_struct_overaligned",
    "try_alloc_big_struct",
    "try_alloc_big_struct_aligned",
    "try_alloc_big_struct_overaligned",
    //
    "alloc_u8_slice",
    "alloc_u8_slice_overaligned",
    "try_alloc_u8_slice",
    "try_alloc_u8_slice_overaligned",
    //
    "alloc_u32_slice",
    "alloc_u32_slice_aligned",
    "alloc_u32_slice_overaligned",
    "try_alloc_u32_slice",
    "try_alloc_u32_slice_aligned",
    "try_alloc_u32_slice_overaligned",
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
    // `blink-alloc` does not support setting a minimum alignment.
    "*aligned/blink_alloc",
];

const FOOTNOTES_FOR_GROUP: &[(&str, usize)] = &[("*shrink*", 2)];
const FOOTNOTES_FOR_LIBRARY: &[(&str, usize)] = &[("*aligned/blink_alloc", 1)];

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

        for (glob, i) in FOOTNOTES_FOR_GROUP {
            if glob_match(glob, group) {
                group_label.write_fmt(format_args!(" [^{i}]")).unwrap();
            }
        }

        let mut row = vec![group_label];

        for &library in LIBRARY_NAMES {
            let path = format!("target/gungraun/{PACKAGE_NAME}/{BENCH_NAME}/{group}/{group}_{library}/summary.json");
            let Report { instructions, branches } = read_summary(path.as_ref());

            let group_and_library = format!("{group}/{library}");

            let mut cell = if (instructions == 0 && branches == 0) || globs_match(INVALID, &group_and_library) {
                "—".to_string()
            } else {
                format!("{instructions} \\| {branches}")
            };

            for (glob, i) in FOOTNOTES_FOR_LIBRARY {
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

fn package_version(metadata: &Metadata, name: &str) -> String {
    metadata
        .packages
        .iter()
        .find(|p| p.name.as_str() == name)
        .unwrap_or_else(|| panic!("can't find package with name '{name}'"))
        .version
        .to_string()
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

        let table = markdown_tables::as_table(&rows.into_iter().map(Row).collect::<Vec<_>>());
        readme = replace_section(&readme, &format!("{section} table"), &format!("\n\n{table}\n"));
    }

    // update compiler info
    {
        let version = rustc_version();
        let rustc = &version["short"];
        let platform = env!("TARGET");
        let llvm = version.get("LLVM version");

        let mut s = String::new();
        write!(s, "`{rustc}` on `{platform}`").unwrap();

        if let Some(llvm) = llvm {
            write!(s, " using `LLVM version {llvm}`").unwrap();
        }

        readme = replace_section(&readme, "version", &s);
    }

    // update library info
    {
        let metadata = MetadataCommand::new().exec().expect("failed to execute `cargo metadata`");
        let bump_scope_version = package_version(&metadata, "bump-scope");
        let bumpalo_version = package_version(&metadata, "bumpalo");
        let blink_alloc_version = package_version(&metadata, "blink-alloc");

        readme = replace_section(&readme, "bump-scope-version", &bump_scope_version);
        readme = replace_section(&readme, "bumpalo-version", &bumpalo_version);
        readme = replace_section(&readme, "blink-alloc-version", &blink_alloc_version);
    }

    std::fs::write("README.md", readme).unwrap();
}
