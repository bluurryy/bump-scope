use core::fmt;
use std::{alloc::Layout, fmt::Write, ptr::NonNull};

use calliper::{Runner, Scenario, ScenarioConfig};

use allocator_api2::alloc::{AllocError, Allocator};
use indexmap::IndexMap;
use markdown_tables::MarkdownTableRow;

// We're using duck typing instead of a trait to be generic over bump allocators
// because I couldn't figure out how to make the current macro setup with `MIN_ALIGN` work with traits.
mod wrapper {
    pub(crate) mod bump_scope {
        use ::allocator_api2::alloc::Allocator;
        use ::bump_scope::{MinimumAlignment, SupportedMinimumAlignment};

        #[repr(transparent)]
        pub struct Bump<const MIN_ALIGN: usize>(bump_scope::Bump<bump_scope::alloc::Global, MIN_ALIGN>)
        where
            MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment;

        impl<const MIN_ALIGN: usize> Bump<MIN_ALIGN>
        where
            MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
        {
            #[inline(always)]
            pub(crate) fn new() -> Self {
                Self(::bump_scope::Bump::new())
            }

            #[inline(always)]
            pub(crate) fn with_capacity(capacity: usize) -> Self {
                Self(::bump_scope::Bump::with_size(capacity))
            }

            #[inline(always)]
            pub(crate) fn alloc<T>(&self, value: T) -> &T {
                ::bump_scope::BumpBox::leak(self.0.alloc(value))
            }

            #[inline(always)]
            pub(crate) fn try_alloc<T>(&self, value: T) -> Option<&T> {
                match self.0.try_alloc(value) {
                    Ok(value) => Some(bump_scope::BumpBox::leak(value)),
                    Err(_) => None,
                }
            }

            #[inline(always)]
            pub(crate) fn as_allocator(&self) -> impl Allocator {
                &self.0
            }

            #[inline(always)]
            pub(crate) fn reset(&mut self) {
                self.0.reset();
            }
        }
    }

    pub(crate) mod bumpalo {
        use ::allocator_api2::alloc::Allocator;

        #[repr(transparent)]
        pub struct Bump<const MIN_ALIGN: usize>(bumpalo::Bump<MIN_ALIGN>);

        impl<const MIN_ALIGN: usize> Bump<MIN_ALIGN> {
            #[inline(always)]
            pub(crate) fn new() -> Self {
                // NOTE: `with_min_align` is faster than `new`
                Self(::bumpalo::Bump::with_min_align())
            }

            #[inline(always)]
            pub(crate) fn with_capacity(capacity: usize) -> Self {
                Self(::bumpalo::Bump::with_min_align_and_capacity(capacity))
            }

            #[inline(always)]
            pub(crate) fn alloc<T>(&self, value: T) -> &T {
                self.0.alloc(value)
            }

            #[inline(always)]
            pub(crate) fn try_alloc<T>(&self, value: T) -> Option<&T> {
                match self.0.try_alloc(value) {
                    Ok(value) => Some(value),
                    Err(_) => None,
                }
            }

            #[inline(always)]
            pub(crate) fn as_allocator(&self) -> impl Allocator {
                &self.0
            }

            #[inline(always)]
            pub(crate) fn reset(&mut self) {
                self.0.reset();
            }
        }
    }

    pub(crate) mod blink_alloc {
        use core::alloc::Layout;

        use ::allocator_api2::alloc::Allocator;

        #[repr(transparent)]
        pub struct Bump<const MIN_ALIGN: usize>(blink_alloc::Blink);

        impl<const MIN_ALIGN: usize> Bump<MIN_ALIGN> {
            #[inline(always)]
            pub(crate) fn new() -> Self {
                Self(::blink_alloc::Blink::new())
            }

            #[inline(always)]
            pub(crate) fn with_capacity(capacity: usize) -> Self {
                let this = blink_alloc::Blink::with_chunk_size(capacity);
                // Blink does not allocate a chunk on creation.
                // We allocate here to make sure a chunk is allocated to make it fair.
                _ = this.allocator().allocate(Layout::new::<[u64; 2]>()).ok();
                Self(this)
            }

            #[inline(always)]
            pub(crate) fn alloc<T>(&self, value: T) -> &T {
                self.0.put_no_drop(value)
            }

            #[inline(always)]
            pub(crate) fn try_alloc<T>(&self, value: T) -> Option<&T> {
                match self.0.emplace_no_drop().try_value(value) {
                    Ok(value) => Some(value),
                    Err(_) => None,
                }
            }

            #[inline(always)]
            pub(crate) fn as_allocator(&self) -> impl Allocator {
                self.0.allocator()
            }

            #[inline(always)]
            pub(crate) fn reset(&mut self) {
                self.0.reset();
            }
        }
    }
}

macro_rules! scenario {
    ($name:ident, $library:ident) => {
        paste::paste! {
            crate::scenario([<wrapper_ $name _ $library>], stringify!([<$name _ $library>]))
        }
    };
}

macro_rules! library {
    (
        bench: $bench:ident,
        library: $library:ident,
        params: ($($param:ident: $param_ty:ty),*) $(-> $ret:ty)?,
        wrap: { $($wrap:tt)* }
        run: { $($run:tt)* }
        run_f: $run_f:ident,
    ) => {
        paste::paste! {
            pub mod $library {
                #[allow(unused_imports)]
                use crate::*;

                type Bump<const MIN_ALIGN: usize = 1> = crate::wrapper::$library::Bump<MIN_ALIGN>;

                #[inline(always)]
                fn generic_wrapper(
                    $run_f: fn($($param_ty),*),
                )  {
                    $($wrap)*
                }

                #[inline(always)]
                fn generic_run($($param: $param_ty),*) $(-> $ret)? {
                    $($run)*
                }


                #[inline(never)]
                #[unsafe(no_mangle)]
                pub fn [<wrapper_ $bench _ $library>]() {
                    generic_wrapper(|$($param: $param_ty),*| {
                        _ = std::hint::black_box([<$bench _ $library>]($(std::hint::black_box($param)),*));
                    });
                }

                #[inline(never)]
                #[unsafe(no_mangle)]
                pub fn [<$bench _ $library>]($($param: $param_ty),*) $(-> $ret)? {
                    generic_run($($param),*)
                }
            }
        }
    };
}

pub(crate) use library;

macro_rules! bench_impls {
    (
        $(
            $name:ident {
                wrap($run_f:ident) {
                    $($wrap:tt)*
                }
                run($($param:ident: $param_ty:ty),*) $(-> $ret:ty)? {
                    $($run:tt)*
                }
            }
        )*
    ) => {
        paste::paste! {
            pub mod bench_impls {
                $(
                    pub mod $name {
                        #[allow(unused_imports)]
                        use crate::*;

                        crate::library! {
                            bench: $name,
                            library: bump_scope,
                            params: ($($param: $param_ty),*) $(-> $ret)?,
                            wrap: { $($wrap)* }
                            run: { $($run)* }
                            run_f: $run_f,
                        }

                        crate::library! {
                            bench: $name,
                            library: bumpalo,
                            params: ($($param: $param_ty),*) $(-> $ret)?,
                            wrap: { $($wrap)* }
                            run: { $($run)* }
                            run_f: $run_f,
                        }

                        crate::library! {
                            bench: $name,
                            library: blink_alloc,
                            params: ($($param: $param_ty),*) $(-> $ret)?,
                            wrap: { $($wrap)* }
                            run: { $($run)* }
                            run_f: $run_f,
                        }
                    }
                )*

                pub fn scenarios() -> Vec<calliper::Scenario> {
                    $(
                        use $name::bump_scope::*;
                        use $name::bumpalo::*;
                        use $name::blink_alloc::*;
                    )*

                    vec![
                        $(
                            scenario!($name, bump_scope),
                            scenario!($name, bumpalo),
                            scenario!($name, blink_alloc),
                        )*
                    ]
                }
            }
        }
    };
}

bench_impls! {
    alloc_u8 {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            run(&bump, 42);
        }
        run(bump: &Bump, value: u8) -> &u8 {
            bump.alloc(value)
        }
    }

    alloc_u32 {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            run(&bump, 42);
        }
        run(bump: &Bump, value: u32) -> &u32 {
            bump.alloc(value)
        }
    }

    alloc_u32_aligned {
        wrap(run) {
            let bump = Bump::<4>::with_capacity(1024);
            run(&bump, 42);
        }
        run(bump: &Bump::<4>, value: u32) -> &u32 {
            bump.alloc(value)
        }
    }

    try_alloc_u32 {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            run(&bump, 42);
        }
        run(bump: &Bump, value: u32) -> Option<&u32> {
            bump.try_alloc(value)
        }
    }

    try_alloc_u32_aligned {
        wrap(run) {
            let bump = Bump::<4>::with_capacity(1024);
            run(&bump, 42);
        }
        run(bump: &Bump::<4>, value: u32) -> Option<&u32> {
            bump.try_alloc(value)
        }
    }

    allocate_u32 {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            run(&bump);
        }
        run(bump: &Bump) -> Result<NonNull<[u8]>, AllocError> {
            bump.as_allocator().allocate(Layout::new::<u32>())
        }
    }

    allocate {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            run(&bump, Layout::new::<u32>());
        }
        run(bump: &Bump, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
            bump.as_allocator().allocate(layout)
        }
    }

    grow_same_align {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            let ptr = bump.as_allocator().allocate(Layout::new::<u32>()).unwrap().cast::<u8>();
            run(&bump, ptr);
        }
        run(bump: &Bump, ptr: NonNull<u8>) ->  Result<NonNull<[u8]>, AllocError> {
            unsafe { bump.as_allocator().grow(ptr, Layout::new::<u32>(), Layout::new::<[u32; 2]>()) }
        }
    }

    grow_smaller_align {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            let ptr = bump.as_allocator().allocate(Layout::new::<u32>()).unwrap().cast::<u8>();
            run(&bump, ptr);
        }
        run(bump: &Bump, ptr: NonNull<u8>) ->  Result<NonNull<[u8]>, AllocError> {
            unsafe { bump.as_allocator().grow(ptr, Layout::new::<u32>(), Layout::new::<[u16; 4]>()) }
        }
    }

    grow_larger_align {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            let ptr = bump.as_allocator().allocate(Layout::new::<u32>()).unwrap().cast::<u8>();
            run(&bump, ptr);
        }
        run(bump: &Bump, ptr: NonNull<u8>) ->  Result<NonNull<[u8]>, AllocError> {
            unsafe { bump.as_allocator().grow(ptr, Layout::new::<u32>(), Layout::new::<u64>()) }
        }
    }

    shrink_same_align {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            let ptr = bump.as_allocator().allocate(Layout::new::<[u32; 2]>()).unwrap().cast::<u8>();
            run(&bump, ptr);
        }
        run(bump: &Bump, ptr: NonNull<u8>) ->  Result<NonNull<[u8]>, AllocError> {
            unsafe { bump.as_allocator().shrink(ptr, Layout::new::<[u32; 2]>(), Layout::new::<u32>()) }
        }
    }

    shrink_smaller_align {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            let ptr = bump.as_allocator().allocate(Layout::new::<u32>()).unwrap().cast::<u8>();
            run(&bump, ptr);
        }
        run(bump: &Bump, ptr: NonNull<u8>) ->  Result<NonNull<[u8]>, AllocError> {
            unsafe { bump.as_allocator().shrink(ptr, Layout::new::<u32>(), Layout::new::<u16>()) }
        }
    }

    shrink_larger_align {
        wrap(run) {
            let bump = Bump::with_capacity(1024);
            let ptr = bump.as_allocator().allocate(Layout::new::<[u16; 4]>()).unwrap().cast::<u8>();
            run(&bump, ptr);
        }
        run(bump: &Bump, ptr: NonNull<u8>) ->  Result<NonNull<[u8]>, AllocError> {
            unsafe { bump.as_allocator().shrink(ptr, Layout::new::<[u16; 4]>(), Layout::new::<u32>()) }
        }
    }

    warm_up {
        wrap(run) {
            run();
        }
        run() -> Bump {
            let bump = Bump::new();
            bump.alloc(0u32);
            bump
        }
    }

    reset {
        wrap(run) {
            let mut bump = Bump::with_capacity(1024);
            bump.as_allocator().allocate(Layout::new::<u32>()).unwrap();
            run(&mut bump);
        }
        run(bump: &mut Bump) {
            bump.reset();
        }
    }
}

fn scenario(f: fn(), name: &str) -> Scenario {
    Scenario::new(f).name(name).config(ScenarioConfig::default().filters([name]))
}

#[derive(Clone)]
struct BenchResult {
    instructions: u64,
    branches: u64,
    #[expect(dead_code)]
    branch_misses: u64,
    footnotes: Vec<usize>,
}

impl BenchResult {
    fn new(report: &calliper::Report) -> Self {
        let parsed = report.parse();

        BenchResult {
            instructions: parsed.instruction_reads.unwrap(),
            branches: parsed.branches.unwrap_or(0),
            branch_misses: parsed.branch_misses.unwrap_or(0),
            footnotes: vec![],
        }
    }
}

impl fmt::Display for BenchResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            instructions, branches, ..
        } = self;

        f.write_fmt(format_args!("{instructions} / {branches}"))?;

        for footnote in &self.footnotes {
            f.write_fmt(format_args!(" [^{footnote}]"))?;
        }

        Ok(())
    }
}

const LIBRARY_NAMES: &[&str] = &["bump_scope", "bumpalo", "blink_alloc"];

fn split_name(name: &str) -> [&str; 2] {
    for library_name in LIBRARY_NAMES {
        if name.ends_with(library_name) {
            let mid = name.len() - library_name.len();
            return [&name[..mid - 1], &name[mid..]];
        }
    }

    panic!("bench function did not end with a library name suffix")
}

struct Report {
    map: IndexMap<String, IndexMap<String, BenchResult>>,
    footnotes: Vec<String>,
}

impl Report {
    fn to_markdown(&self) -> String {
        struct Row(Vec<String>);

        impl MarkdownTableRow for Row {
            fn column_names() -> Vec<&'static str> {
                vec!["name", "bump-scope", "bumpalo", "blink-alloc"]
            }

            fn column_values(&self) -> Vec<String> {
                self.0.clone()
            }
        }

        let mut rows = vec![];

        for (name, libraries) in &self.map {
            let mut row = vec![name.to_string()];

            for &library_name in LIBRARY_NAMES {
                row.push(libraries[&library_name.to_string()].to_string());
            }

            rows.push(Row(row));
        }

        let mut markdown = markdown_tables::as_table(&rows);

        if !self.footnotes.is_empty() {
            markdown.push('\n');

            for (i, footnote) in self.footnotes.iter().enumerate() {
                markdown.write_fmt(format_args!("[^{i}]: {footnote}")).unwrap();
            }
        }

        markdown
    }

    fn register_footnote(&mut self, footnote: &str) -> usize {
        let index = self.footnotes.len();
        self.footnotes.push(footnote.into());
        index
    }
}

fn run_benches() -> Report {
    let runner = Runner::default().config(ScenarioConfig::default().branch_sim(true));
    let scenarios = bench_impls::scenarios();

    let reports = runner
        .run(&scenarios)
        .expect("runner failed")
        .expect("runner didn't return anything");

    for report in &reports {
        let parsed = report.parse();
        eprintln!("\n{parsed}");
    }

    let mut map = IndexMap::<String, IndexMap<String, BenchResult>>::new();

    for report in &reports {
        let [name, library] = split_name(report.scenario.get_name()).map(String::from);
        let result = BenchResult::new(report);
        map.entry(name).or_default().insert(library, result);
    }

    Report {
        map,
        footnotes: Default::default(),
    }
}

fn main() {
    let mut report = run_benches();

    let blink_alloc_does_not_support_min_align =
        report.register_footnote("`blink_alloc` does not support setting a minimum alignment");

    for (bench, libraries) in &mut report.map {
        if bench.ends_with("_aligned") {
            libraries["blink_alloc"]
                .footnotes
                .push(blink_alloc_does_not_support_min_align);
        }
    }

    let table = report.to_markdown();
    println!("{table}");
}
