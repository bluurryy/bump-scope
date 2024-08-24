use super::usize::max;
use core::alloc::{Layout, LayoutError};

const LAYOUT_ERROR: LayoutError = match Layout::from_size_align(0, 0) {
    Ok(_) => unreachable!(),
    Err(error) => error,
};

pub(crate) const fn extend(this: Layout, next: Layout) -> Result<(Layout, usize), LayoutError> {
    let new_align = max(this.align(), next.align());

    let pad = padding_needed_for(this, next.align());

    let offset = match this.size().checked_add(pad) {
        Some(offset) => offset,
        None => return Err(LAYOUT_ERROR),
    };

    let new_size = match offset.checked_add(next.size()) {
        Some(new_size) => new_size,
        None => return Err(LAYOUT_ERROR),
    };

    // The safe constructor is called here to enforce the isize size limit.
    let layout = match Layout::from_size_align(new_size, new_align) {
        Ok(layout) => layout,
        Err(error) => return Err(error),
    };

    Ok((layout, offset))
}

pub(crate) const fn padding_needed_for(this: Layout, align: usize) -> usize {
    let len = this.size();
    let len_rounded_up = len.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
    len_rounded_up.wrapping_sub(len)
}
