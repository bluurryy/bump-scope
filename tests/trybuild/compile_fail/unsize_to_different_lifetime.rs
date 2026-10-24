use bump_scope::{unsize_bump_box, BoxU};

fn evil_unsize(boxed: BoxU<[i32; 3]>) -> BoxU<'static, [i32]> {
    unsize_bump_box!(boxed)
}

fn main() {}
