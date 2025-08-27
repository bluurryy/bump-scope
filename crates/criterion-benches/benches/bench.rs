#![allow(clippy::mut_from_ref)]

use core::alloc::Layout;

use bump_scope::{
    BumpBox, MinimumAlignment, SupportedMinimumAlignment,
    alloc::{AllocError, Global},
};

type Bump<const MIN_ALIGN: usize, const UP: bool> = bump_scope::Bump<Global, MIN_ALIGN, UP, true, true>;

trait Bumper {
    fn with_capacity(layout: Layout) -> Self;
    #[allow(dead_code)]
    fn alloc<T>(&self, value: T) -> &mut T;
    fn alloc_with<T>(&self, f: impl FnOnce() -> T) -> &mut T;
    fn alloc_try_with<T, E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<&mut T, E>;
    #[allow(dead_code)]
    fn try_alloc<T>(&self, value: T) -> Result<&mut T, AllocError>;
    fn try_alloc_with<T>(&self, f: impl FnOnce() -> T) -> Result<&mut T, AllocError>;
    fn try_alloc_try_with<T, E>(&self, f: impl FnOnce() -> Result<T, E>) -> Result<Result<&mut T, E>, AllocError>;
}

impl<const MIN_ALIGN: usize, const UP: bool> Bumper for Bump<MIN_ALIGN, UP>
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

impl<const MIN_ALIGN: usize> Bumper for bumpalo::Bump<MIN_ALIGN> {
    fn with_capacity(layout: Layout) -> Self {
        bumpalo::Bump::with_min_align_and_capacity(layout.size())
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
use std::hint::black_box;

type Small = u8;
type Big = [usize; 32];

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
fn try_alloc_try_with_ok<B: Bumper, T: Default, E>(n: usize) {
    let bump = B::with_capacity(Layout::array::<Result<T, E>>(n).unwrap());

    for _ in 0..n {
        let bump = black_box(&bump);
        let value: Result<Result<&mut T, E>, AllocError> = bump.try_alloc_try_with(|| black_box(Ok(Default::default())));
        let _ = black_box(value);
    }
}

#[allow(dead_code)]
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
fn bench_alloc_u8(c: &mut Criterion) {
    let mut group = c.benchmark_group("alloc_u8");
    group.throughput(Throughput::Elements(ALLOCATIONS as u64));
    group.bench_function("bumpalo", func(alloc_with::<bumpalo::Bump, u8>));
    group.bench_function("up", func(alloc_with::<Bump<1, true>, u8>));
    group.bench_function("down", func(alloc_with::<Bump<1, false>, u8>));
}

fn bench_alloc_u32(c: &mut Criterion) {
    bench_alloc::<u32>(c, "alloc_u32")
}

fn bench_alloc_u32_try(c: &mut Criterion) {
    bench_alloc_try::<u32>(c, "alloc_u32_try")
}

fn bench_alloc_12_u32(c: &mut Criterion) {
    bench_alloc::<[u32; 12]>(c, "alloc_12_u32")
}

fn bench_alloc_12_u32_try(c: &mut Criterion) {
    bench_alloc_try::<[u32; 12]>(c, "alloc_12_u32_try")
}

#[rustfmt::skip]
fn bench_alloc<T: Default>(c: &mut Criterion, name: &str) {
    let mut group = c.benchmark_group(name);
    group.throughput(Throughput::Elements(ALLOCATIONS as u64));
    group.bench_function("bumpalo", func(alloc_with::<bumpalo::Bump, T>));
    group.bench_function("bumpalo_aligned", func(alloc_with::<bumpalo::Bump::<4>, T>));
    group.bench_function("bumpalo_overaligned", func(alloc_with::<bumpalo::Bump::<16>, T>));
    group.bench_function("up", func(alloc_with::<Bump<1, true>, T>));
    group.bench_function("up_aligned", func(alloc_with::<Bump<4, true>, T>));
    group.bench_function("up_overaligned", func(alloc_with::<Bump<16, true>, T>));
    group.bench_function("down", func(alloc_with::<Bump<1, false>, T>));
    group.bench_function("down_aligned", func(alloc_with::<Bump<4, false>, T>));
    group.bench_function("down_overaligned", func(alloc_with::<Bump<16, false>, T>));
}

#[rustfmt::skip]
fn bench_alloc_try<T: Default>(c: &mut Criterion, name: &str) {
    let mut group = c.benchmark_group(name);
    group.throughput(Throughput::Elements(ALLOCATIONS as u64));
    group.bench_function("bumpalo", func(try_alloc_with::<bumpalo::Bump, T>));
    group.bench_function("bumpalo_aligned", func(try_alloc_with::<bumpalo::Bump<4>, T>));
    group.bench_function("bumpalo_overaligned", func(try_alloc_with::<bumpalo::Bump<16>, T>));
    group.bench_function("up", func(try_alloc_with::<Bump<1, true>, T>));
    group.bench_function("up_aligned", func(try_alloc_with::<Bump<4, true>, T>));
    group.bench_function("up_overaligned", func(try_alloc_with::<Bump<16, true>, T>));
    group.bench_function("down", func(try_alloc_with::<Bump<1, false>, T>));
    group.bench_function("down_aligned", func(try_alloc_with::<Bump<4, false>, T>));
    group.bench_function("down_overaligned", func(try_alloc_with::<Bump<16, false>, T>));
}

#[rustfmt::skip]
fn bench_alloc_try_with_ok_small_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("alloc_try_with_ok_small_small");
    group.throughput(Throughput::Elements(ALLOCATIONS as u64));
    group.bench_function("up", func(alloc_try_with_ok::<Bump<1, true>, Small, Small>));
    group.bench_function("down", func(alloc_try_with_ok::<Bump<1, false>, Small, Small>));
    group.bench_function("bumpalo", func(alloc_try_with_ok::<bumpalo::Bump, Small, Small>));
}

#[rustfmt::skip]
fn bench_alloc_try_with_ok_small_big(c: &mut Criterion) {
    let mut group = c.benchmark_group("alloc_try_with_ok_small_big");
    group.throughput(Throughput::Elements(ALLOCATIONS as u64));
    group.bench_function("up", func(alloc_try_with_ok::<Bump<1, true>, Small, Big>));
    group.bench_function("down", func(alloc_try_with_ok::<Bump<1, false>, Small, Big>));
    group.bench_function("bumpalo", func(alloc_try_with_ok::<bumpalo::Bump, Small, Big>));
}

#[rustfmt::skip]
fn bench_alloc_try_with_ok_big_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("alloc_try_with_ok_big_small");
    group.throughput(Throughput::Elements(ALLOCATIONS as u64));
    group.bench_function("up", func(alloc_try_with_ok::<Bump<1, true>, Big, Small>));
    group.bench_function("down", func(alloc_try_with_ok::<Bump<1, false>, Big, Small>));
    group.bench_function("bumpalo", func(alloc_try_with_ok::<bumpalo::Bump, Big, Small>));
}

#[rustfmt::skip]
fn bench_alloc_try_with_ok_big_big(c: &mut Criterion) {
    let mut group = c.benchmark_group("alloc_try_with_ok_big_big");
    group.throughput(Throughput::Elements(ALLOCATIONS as u64));
    group.bench_function("up", func(alloc_try_with_ok::<Bump<1, true>, Big, Big>));
    group.bench_function("down", func(alloc_try_with_ok::<Bump<1, false>, Big, Big>));
    group.bench_function("bumpalo", func(alloc_try_with_ok::<bumpalo::Bump, Big, Big>));
}

#[rustfmt::skip]
fn bench_alloc_try_with_err_small_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("alloc_try_with_err_small_small");
    group.throughput(Throughput::Elements(ALLOCATIONS as u64));
    group.bench_function("up", func(alloc_try_with_err::<Bump<1, true>, Small, Small>));
    group.bench_function("down", func(alloc_try_with_err::<Bump<1, false>, Small, Small>));
    group.bench_function("bumpalo", func(alloc_try_with_err::<bumpalo::Bump, Small, Small>));
}
#[rustfmt::skip]
fn bench_alloc_try_with_err_small_big(c: &mut Criterion) {
    let mut group = c.benchmark_group("alloc_try_with_err_small_big");
    group.throughput(Throughput::Elements(ALLOCATIONS as u64));
    group.bench_function("up", func(alloc_try_with_err::<Bump<1, true>, Small, Big>));
    group.bench_function("down", func(alloc_try_with_err::<Bump<1, false>, Small, Big>));
    group.bench_function("bumpalo", func(alloc_try_with_err::<bumpalo::Bump, Small, Big>));
}

#[rustfmt::skip]
fn bench_alloc_try_with_err_big_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("alloc_try_with_err_big_small");
    group.throughput(Throughput::Elements(ALLOCATIONS as u64));
    group.bench_function("up", func(alloc_try_with_err::<Bump<1, true>, Big, Small>));
    group.bench_function("down", func(alloc_try_with_err::<Bump<1, false>, Big, Small>));
    group.bench_function("bumpalo", func(alloc_try_with_err::<bumpalo::Bump, Big, Small>));
}

#[rustfmt::skip]
fn bench_alloc_try_with_err_big_big(c: &mut Criterion) {
    let mut group = c.benchmark_group("alloc_try_with_err_big_big");
    group.throughput(Throughput::Elements(ALLOCATIONS as u64));
    group.bench_function("up", func(alloc_try_with_err::<Bump<1, true>, Big, Big>));
    group.bench_function("down", func(alloc_try_with_err::<Bump<1, false>, Big, Big>));
    group.bench_function("bumpalo", func(alloc_try_with_err::<bumpalo::Bump, Big, Big>));
}

criterion_group!(
    benches,
    bench_alloc_u8,
    bench_alloc_u32,
    bench_alloc_u32_try,
    bench_alloc_12_u32,
    bench_alloc_12_u32_try,
    bench_alloc_try_with_ok_small_small,
    bench_alloc_try_with_ok_small_big,
    bench_alloc_try_with_ok_big_small,
    bench_alloc_try_with_ok_big_big,
    bench_alloc_try_with_err_small_small,
    bench_alloc_try_with_err_small_big,
    bench_alloc_try_with_err_big_small,
    bench_alloc_try_with_err_big_big,
);

criterion_main!(benches);
