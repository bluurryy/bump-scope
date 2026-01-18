use bump_scope::{unsize_bump_box, BumpBox};

fn evil_unsize(boxed: BumpBox<[i32; 3]>) -> BumpBox<'static, [i32]> {
    unsize_bump_box!(boxed)
}

fn main() {}
