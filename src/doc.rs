macro_rules! use_mut_instead {
    ($what:ident) => {
        concat!(
            "\n\n",
            "If you have a `&mut self` you can use [`",
            stringify!($what),
            "`](Self::",
            stringify!($what),
            ") instead for better performance.",
        )
    };
}

pub(crate) use use_mut_instead;

macro_rules! mut_alloc_function {
    ($compared_to:ident, $the_what:literal) => {
        concat!(
            "\n\n",
            "This function is designed as a performance improvement over [`",
            stringify!($compared_to),
            "`](Self::",
            stringify!($compared_to),
            "). By taking `self` as `&mut`, it can use the entire remaining chunk space ",
            "as the capacity for its ",
            $the_what,
            ". As a result, the ",
            $the_what,
            " rarely needs to grow.",
        )
    };
}

pub(crate) use mut_alloc_function;
