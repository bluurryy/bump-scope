#![cfg_attr(feature = "nightly-allocator-api", feature(allocator_api))]
use bump_scope::{
    allocator_api2::alloc::{AllocError, Global, Layout},
    Bump, BumpBox, MinimumAlignment, SupportedMinimumAlignment,
};

trait Bumper {
    fn with_capacity(layout: Layout) -> Self;
    fn alloc<T>(&self, value: T) -> &mut T;
    fn alloc_with<T>(&self, f: impl FnOnce() -> T) -> &mut T;
    fn alloc_try_with<T, E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<&mut T, E>;
    fn try_alloc<T>(&self, value: T) -> Result<&mut T, AllocError>;
    fn try_alloc_with<T>(&self, f: impl FnOnce() -> T) -> Result<&mut T, AllocError>;
    fn try_alloc_try_with<T, E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<Result<&mut T, E>, AllocError>;
}

impl<const MIN_ALIGN: usize, const UP: bool> Bumper for Bump<Global, MIN_ALIGN, UP>
where
    MinimumAlignment<MIN_ALIGN>: SupportedMinimumAlignment,
{
    fn with_capacity(layout: Layout) -> Self {
        Bump::with_capacity(layout)
    }

    fn alloc<T>(&self, value: T) -> &mut T {
        BumpBox::leak(Bump::alloc(self, value))
    }

    fn alloc_with<T>(&self, f: impl FnOnce() -> T) -> &mut T {
        BumpBox::leak(Bump::alloc_with(self, f))
    }

    fn alloc_try_with<T, E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<&mut T, E> {
        Bump::alloc_try_with(self, f).map(BumpBox::leak)
    }

    fn try_alloc<T>(&self, value: T) -> Result<&mut T, AllocError> {
        Bump::try_alloc(self, value).map(BumpBox::leak)
    }

    fn try_alloc_with<T>(&self, f: impl FnOnce() -> T) -> Result<&mut T, AllocError> {
        Bump::try_alloc_with(self, f).map(BumpBox::leak)
    }

    fn try_alloc_try_with<T, E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<Result<&mut T, E>, AllocError> {
        Bump::try_alloc_try_with(self, f).map(|r| r.map(BumpBox::leak))
    }
}

impl Bumper for bumpalo::Bump {
    fn with_capacity(layout: Layout) -> Self {
        bumpalo::Bump::with_capacity(layout.size())
    }

    fn alloc<T>(&self, value: T) -> &mut T {
        bumpalo::Bump::alloc(self, value)
    }

    fn alloc_with<T>(&self, f: impl FnOnce() -> T) -> &mut T {
        bumpalo::Bump::alloc_with(self, f)
    }

    fn alloc_try_with<T, E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<&mut T, E> {
        bumpalo::Bump::alloc_try_with(self, f)
    }

    fn try_alloc<T>(&self, value: T) -> Result<&mut T, AllocError> {
        bumpalo::Bump::try_alloc(self, value).map_err(|_| AllocError)
    }

    fn try_alloc_with<T>(&self, f: impl FnOnce() -> T) -> Result<&mut T, AllocError> {
        bumpalo::Bump::try_alloc_with(self, f).map_err(|_| AllocError)
    }

    fn try_alloc_try_with<T, E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<Result<&mut T, E>, AllocError> {
        match bumpalo::Bump::try_alloc_try_with(self, f) {
            Ok(ok) => Ok(Ok(ok)),
            Err(err) => match err {
                bumpalo::AllocOrInitError::Alloc(_) => Err(AllocError),
                bumpalo::AllocOrInitError::Init(err) => Ok(Err(err)),
            },
        }
    }
}

use criterion::*;

type Small = u8;
type Big = [usize; 32];

fn alloc<B: Bumper, T: Default>(n: usize) {
    let bump = B::with_capacity(Layout::array::<T>(n).unwrap());

    for _ in 0..n {
        let bump = black_box(&bump);
        let value: &mut T = bump.alloc(black_box(Default::default()));
        black_box(value);
    }
}

fn alloc_with<B: Bumper, T: Default>(n: usize) {
    let bump = B::with_capacity(Layout::array::<T>(n).unwrap());

    for _ in 0..n {
        let bump = black_box(&bump);
        let val: &mut T = bump.alloc_with(|| black_box(Default::default()));
        black_box(val);
    }
}

fn alloc_try_with_ok<B: Bumper, T: Default, E>(n: usize) {
    let bump = B::with_capacity(Layout::array::<T>(n).unwrap());

    for _ in 0..n {
        let bump = black_box(&bump);
        let value: Result<&mut T, E> = bump.alloc_try_with(|| black_box(Ok(Default::default())));
        let _ = black_box(value);
    }
}

fn alloc_try_with_err<B: Bumper, T, E: Default>(n: usize) {
    let bump = B::with_capacity(Layout::array::<T>(n).unwrap());

    for _ in 0..n {
        let bump = black_box(&bump);
        let val: Result<&mut T, E> = bump.alloc_try_with(|| black_box(Err(Default::default())));
        let _ = black_box(val);
    }
}

fn try_alloc<B: Bumper, T: Default>(n: usize) {
    let bump = B::with_capacity(Layout::array::<T>(n).unwrap());

    for _ in 0..n {
        let bump = black_box(&bump);
        let val: Result<&mut T, _> = bump.try_alloc(black_box(Default::default()));
        let _ = black_box(val);
    }
}

fn try_alloc_with<B: Bumper, T: Default>(n: usize) {
    let bump = B::with_capacity(Layout::array::<T>(n).unwrap());

    for _ in 0..n {
        let bump = black_box(&bump);
        let val: Result<&mut T, _> = bump.try_alloc_with(|| black_box(Default::default()));
        let _ = black_box(val);
    }
}

fn try_alloc_try_with_ok<B: Bumper, T: Default, E>(n: usize) {
    let bump = B::with_capacity(Layout::array::<Result<T, E>>(n).unwrap());

    for _ in 0..n {
        let bump = black_box(&bump);
        let value: Result<Result<&mut T, E>, AllocError> = bump.try_alloc_try_with(|| black_box(Ok(Default::default())));
        let _ = black_box(value);
    }
}

fn try_alloc_try_with_err<B: Bumper, T, E: Default>(n: usize) {
    // Only enough capacity for one, since the allocation is undone.
    let bump = B::with_capacity(Layout::array::<Result<T, E>>(n).unwrap());

    for _ in 0..n {
        let bump = black_box(&bump);
        let value: Result<Result<&mut T, E>, AllocError> = bump.try_alloc_try_with(|| black_box(Err(Default::default())));
        let _ = black_box(value);
    }
}

const ALLOCATIONS: usize = 5_000;

fn func(f: impl Fn(usize)) -> impl Fn(&mut Bencher) {
    move |b| b.iter(|| f(ALLOCATIONS))
}

#[rustfmt::skip]
fn bench_alloc(c: &mut Criterion) {
    let mut group = c.benchmark_group("alloc");
    group.throughput(Throughput::Elements(ALLOCATIONS as u64));
    group.bench_function("u8_bumpalo", func(alloc::<bumpalo::Bump, u8>));
    group.bench_function("u32_bumpalo", func(alloc::<bumpalo::Bump, u32>));
    group.bench_function("u8_up", func(alloc::<Bump<Global, 1, true>, u8>));
    group.bench_function("u8_down", func(alloc::<Bump<Global, 1, false>, u8>));
    group.bench_function("u32_up", func(alloc::<Bump<Global, 1, true>, u32>));
    group.bench_function("u32_down", func(alloc::<Bump<Global, 1, false>, u32>));
    group.bench_function("u32_aligned_up", func(alloc::<Bump<Global, 4, true>, u32>));
    group.bench_function("u32_aligned_down", func(alloc::<Bump<Global, 4, false>, u32>));
    group.bench_function("u32_overaligned_up", func(alloc::<Bump<Global, 16, true>, u32>));
    group.bench_function("u32_overaligned_down", func(alloc::<Bump<Global, 16, false>, u32>));
}

#[rustfmt::skip]
fn bench_alloc_with(c: &mut Criterion) {
    let mut group = c.benchmark_group("alloc-with");
    group.throughput(Throughput::Elements(ALLOCATIONS as u64));
    group.bench_function("u8_bumpalo", func(alloc_with::<bumpalo::Bump, u8>));
    group.bench_function("u32_bumpalo", func(alloc_with::<bumpalo::Bump, u32>));
    group.bench_function("u8_up", func(alloc_with::<Bump<Global, 1, true>, u8>));
    group.bench_function("u8_down", func(alloc_with::<Bump<Global, 1, false>, u8>));
    group.bench_function("u32_up", func(alloc_with::<Bump<Global, 1, true>, u32>));
    group.bench_function("u32_down", func(alloc_with::<Bump<Global, 1, false>, u32>));
    group.bench_function("u32_aligned_up", func(alloc_with::<Bump<Global, 4, true>, u32>));
    group.bench_function("u32_aligned_down", func(alloc_with::<Bump<Global, 4, false>, u32>));
    group.bench_function("u32_overaligned_up", func(alloc_with::<Bump<Global, 16, true>, u32>));
    group.bench_function("u32_overaligned_down", func(alloc_with::<Bump<Global, 16, false>, u32>));
}

#[rustfmt::skip]
fn bench_alloc_try_with_ok(c: &mut Criterion) {
    let mut group = c.benchmark_group("alloc-try-with-ok");
    group.throughput(Throughput::Elements(ALLOCATIONS as u64));
    group.bench_function("small_small__up", func(alloc_try_with_ok::<Bump<Global, 1, true>, Small, Small>));
    group.bench_function("small_small__down", func(alloc_try_with_ok::<Bump<Global, 1, false>, Small, Small>));
    group.bench_function("small_small__bumpalo", func(alloc_try_with_ok::<bumpalo::Bump, Small, Small>));
    group.bench_function("small_big__up", func(alloc_try_with_ok::<Bump<Global, 1, true>, Small, Big>));
    group.bench_function("small_big__down", func(alloc_try_with_ok::<Bump<Global, 1, false>, Small, Big>));
    group.bench_function("small_big__bumpalo", func(alloc_try_with_ok::<bumpalo::Bump, Small, Big>));
    group.bench_function("big_small__up", func(alloc_try_with_ok::<Bump<Global, 1, true>, Big, Small>));
    group.bench_function("big_small__down", func(alloc_try_with_ok::<Bump<Global, 1, false>, Big, Small>));
    group.bench_function("big_small__bumpalo", func(alloc_try_with_ok::<bumpalo::Bump, Big, Small>));
    group.bench_function("big_big__up", func(alloc_try_with_ok::<Bump<Global, 1, true>, Big, Big>));
    group.bench_function("big_big__down", func(alloc_try_with_ok::<Bump<Global, 1, false>, Big, Big>));
    group.bench_function("big_big__bumpalo", func(alloc_try_with_ok::<bumpalo::Bump, Big, Big>));
}

#[rustfmt::skip]
fn bench_alloc_try_with_err(c: &mut Criterion) {
    let mut group = c.benchmark_group("alloc-try-with-err");
    group.throughput(Throughput::Elements(ALLOCATIONS as u64));
    group.bench_function("small_small__up", func(alloc_try_with_err::<Bump<Global, 1, true>, Small, Small>));
    group.bench_function("small_small__down", func(alloc_try_with_err::<Bump<Global, 1, false>, Small, Small>));
    group.bench_function("small_small__bumpalo", func(alloc_try_with_err::<bumpalo::Bump, Small, Small>));
    group.bench_function("small_big__up", func(alloc_try_with_err::<Bump<Global, 1, true>, Small, Big>));
    group.bench_function("small_big__down", func(alloc_try_with_err::<Bump<Global, 1, false>, Small, Big>));
    group.bench_function("small_big__bumpalo", func(alloc_try_with_err::<bumpalo::Bump, Small, Big>));
    group.bench_function("big_small__up", func(alloc_try_with_err::<Bump<Global, 1, true>, Big, Small>));
    group.bench_function("big_small__down", func(alloc_try_with_err::<Bump<Global, 1, false>, Big, Small>));
    group.bench_function("big_small__bumpalo", func(alloc_try_with_err::<bumpalo::Bump, Big, Small>));
    group.bench_function("big_big__up", func(alloc_try_with_err::<Bump<Global, 1, true>, Big, Big>));
    group.bench_function("big_big__down", func(alloc_try_with_err::<Bump<Global, 1, false>, Big, Big>));
    group.bench_function("big_big__bumpalo", func(alloc_try_with_err::<bumpalo::Bump, Big, Big>));
}

#[rustfmt::skip]
fn bench_try_alloc(c: &mut Criterion) {
    let mut group = c.benchmark_group("try-alloc");
    group.throughput(Throughput::Elements(ALLOCATIONS as u64));
    group.bench_function("u8_bumpalo", func(try_alloc::<bumpalo::Bump, u8>));
    group.bench_function("u32_bumpalo", func(try_alloc::<bumpalo::Bump, u32>));
    group.bench_function("u8_up", func(try_alloc::<Bump<Global, 1, true>, u8>));
    group.bench_function("u8_down", func(try_alloc::<Bump<Global, 1, false>, u8>));
    group.bench_function("u32_up", func(try_alloc::<Bump<Global, 1, true>, u32>));
    group.bench_function("u32_down", func(try_alloc::<Bump<Global, 1, false>, u32>));
    group.bench_function("u32_aligned_up", func(try_alloc::<Bump<Global, 4, true>, u32>));
    group.bench_function("u32_aligned_down", func(try_alloc::<Bump<Global, 4, false>, u32>));
    group.bench_function("u32_overaligned_up", func(try_alloc::<Bump<Global, 16, true>, u32>));
    group.bench_function("u32_overaligned_down", func(try_alloc::<Bump<Global, 16, false>, u32>));
}

#[rustfmt::skip]
fn bench_try_alloc_with(c: &mut Criterion) {
    let mut group = c.benchmark_group("try-alloc-with");
    group.throughput(Throughput::Elements(ALLOCATIONS as u64));
    group.bench_function("u8_bumpalo", func(try_alloc_with::<bumpalo::Bump, u8>));
    group.bench_function("u32_bumpalo", func(try_alloc_with::<bumpalo::Bump, u32>));
    group.bench_function("u8_up", func(try_alloc_with::<Bump<Global, 1, true>, u8>));
    group.bench_function("u8_down", func(try_alloc_with::<Bump<Global, 1, false>, u8>));
    group.bench_function("u32_up", func(try_alloc_with::<Bump<Global, 1, true>, u32>));
    group.bench_function("u32_down", func(try_alloc_with::<Bump<Global, 1, false>, u32>));
    group.bench_function("u32_aligned_up", func(try_alloc_with::<Bump<Global, 4, true>, u32>));
    group.bench_function("u32_aligned_down", func(try_alloc_with::<Bump<Global, 4, false>, u32>));
    group.bench_function("u32_overaligned_up", func(try_alloc_with::<Bump<Global, 16, true>, u32>));
    group.bench_function("u32_overaligned_down", func(try_alloc_with::<Bump<Global, 16, false>, u32>));
}

#[rustfmt::skip]
fn bench_try_alloc_try_with_ok(c: &mut Criterion) {
    let mut group = c.benchmark_group("alloc-try-with-ok");
    group.throughput(Throughput::Elements(ALLOCATIONS as u64));
    group.bench_function("small_small__up", func(try_alloc_try_with_ok::<Bump<Global, 1, true>, Small, Small>));
    group.bench_function("small_small__down", func(try_alloc_try_with_ok::<Bump<Global, 1, false>, Small, Small>));
    group.bench_function("small_small__bumpalo", func(try_alloc_try_with_ok::<bumpalo::Bump, Small, Small>));
    group.bench_function("small_big__up", func(try_alloc_try_with_ok::<Bump<Global, 1, true>, Small, Big>));
    group.bench_function("small_big__down", func(try_alloc_try_with_ok::<Bump<Global, 1, false>, Small, Big>));
    group.bench_function("small_big__bumpalo", func(try_alloc_try_with_ok::<bumpalo::Bump, Small, Big>));
    group.bench_function("big_small__up", func(try_alloc_try_with_ok::<Bump<Global, 1, true>, Big, Small>));
    group.bench_function("big_small__down", func(try_alloc_try_with_ok::<Bump<Global, 1, false>, Big, Small>));
    group.bench_function("big_small__bumpalo", func(try_alloc_try_with_ok::<bumpalo::Bump, Big, Small>));
    group.bench_function("big_big__up", func(try_alloc_try_with_ok::<Bump<Global, 1, true>, Big, Big>));
    group.bench_function("big_big__down", func(try_alloc_try_with_ok::<Bump<Global, 1, false>, Big, Big>));
    group.bench_function("big_big__bumpalo", func(try_alloc_try_with_ok::<bumpalo::Bump, Big, Big>));
}

#[rustfmt::skip]
fn bench_try_alloc_try_with_err(c: &mut Criterion) {
    let mut group = c.benchmark_group("alloc-try-with-err");
    group.throughput(Throughput::Elements(ALLOCATIONS as u64));
    group.bench_function("small_small__up", func(try_alloc_try_with_err::<Bump<Global, 1, true>, Small, Small>));
    group.bench_function("small_small__down", func(try_alloc_try_with_err::<Bump<Global, 1, false>, Small, Small>));
    group.bench_function("small_small__bumpalo", func(try_alloc_try_with_err::<bumpalo::Bump, Small, Small>));
    group.bench_function("small_big__up", func(try_alloc_try_with_err::<Bump<Global, 1, true>, Small, Big>));
    group.bench_function("small_big__down", func(try_alloc_try_with_err::<Bump<Global, 1, false>, Small, Big>));
    group.bench_function("small_big__bumpalo", func(try_alloc_try_with_err::<bumpalo::Bump, Small, Big>));
    group.bench_function("big_small__up", func(try_alloc_try_with_err::<Bump<Global, 1, true>, Big, Small>));
    group.bench_function("big_small__down", func(try_alloc_try_with_err::<Bump<Global, 1, false>, Big, Small>));
    group.bench_function("big_small__bumpalo", func(try_alloc_try_with_err::<bumpalo::Bump, Big, Small>));
    group.bench_function("big_big__up", func(try_alloc_try_with_err::<Bump<Global, 1, true>, Big, Big>));
    group.bench_function("big_big__down", func(try_alloc_try_with_err::<Bump<Global, 1, false>, Big, Big>));
    group.bench_function("big_big__bumpalo", func(try_alloc_try_with_err::<bumpalo::Bump, Big, Big>));
}

criterion_group!(
    benches,
    bench_alloc,
    bench_alloc_with,
    bench_alloc_try_with_ok,
    bench_alloc_try_with_err,
    bench_try_alloc,
    bench_try_alloc_with,
    bench_try_alloc_try_with_ok,
    bench_try_alloc_try_with_err,
);

criterion_main!(benches);
