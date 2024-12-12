# This was written for nushell version 0.100.0 and cargo-show-asm version 0.2.30

let MARKER_UNDERSCORE = "XXXXX"

def map-to-index [] {
    uniq
    | enumerate
    | group-by item
    | update cells { get 0 | get index }
    | get 0
}

def parse-label-with-function-index [name: string] {
    let content = $in

    $content
    | parse --regex ($name + '(?<f>[0-9]+)_(?<i>[0-9]+)')
    | group-by f?
    | update cells { get i | each { into int } | map-to-index }
    | get 0
}

def replace-label-with-function-index [function_map, map, prefix: string] {
    mut content = $in

    for $old in ($map | transpose function indices) {
        let old_function = $old.function
        let new_function = $function_map | get $old_function | into string

        for $item in ($old.indices | transpose old_index new_index) {
            let old_index = $item.old_index
            let new_index = $item.new_index | into string
            let old_re = '\.' + $prefix + $old_function + "_" + $old_index + '\b'
            let new_re = "." + $prefix + $new_function + $MARKER_UNDERSCORE + $new_index
            $content = ($content | str replace -a --regex $old_re $new_re)
        }
    }

    $content
}

def replace-label [map, prefix: string] {
    mut content = $in

    for $item in ($map | transpose old_index new_index) {
        let old_index = $item.old_index
        let new_index = $item.new_index | into string
        let old_re = '\.' + $prefix + '_' + $old_index + '\b'
        let new_re = "." + $prefix + $MARKER_UNDERSCORE + $new_index
        $content = ($content | str replace -a --regex $old_re $new_re)
    }

    $content
}

def simplify [] {
    let content = $in

    let lbbs = $content | parse-label-with-function-index '(?m)^\.LBB'
    let ljtis = $content | parse-label-with-function-index '\.LJTI'
    let lcpis = $content | parse-label-with-function-index '\.LCPI'

    let function_map = [$lbbs, $ljtis, $lcpis]
    | columns
    | flatten
    | map-to-index

    let unnameds = $content
    | parse --regex '\.L__unnamed_(?<i>[0-9]+)'
    | get i
    | map-to-index

    return (
        $content
        | replace-label-with-function-index $function_map $lbbs "LBB"
        | replace-label-with-function-index $function_map $ljtis "LJTI"
        | replace-label-with-function-index $function_map $lcpis "LCPI"
        | replace-label $unnameds "L__unnamed"
        | str replace -a $MARKER_UNDERSCORE "_"
    )
}

def report [file_name: string, symbol:string] {
    print -e $"($symbol) ($file_name)"
}

def asm-save [name: string, target: string, extra_args: list<string>] {
    let file_stem = ($name | str replace -a "::" "/")
    let file_name = $"($file_stem).asm"
    let file_path = ([out $target $file_name] | path join)
    let out_dir = ($file_path | path dirname)

    let result = do {
        ^cargo asm --simplify $name 0 ...$extra_args
    } | complete

    if $result.exit_code != 0 {
        if "You asked to display item #0 (zero based), but there's only 0 matching items" in $result.stdout {
        report $file_stem "?"
        mkdir $out_dir
        "" | save -f $file_path
        return
        }

        print -e $result.stderr
        print $result.stdout
        error make { msg: "cargo erred" }
    }

    let old_content = try { $file_path | open --raw } catch { "" }
    let new_content = ($result.stdout | simplify)

    if $new_content == $old_content {
        report $file_stem "="
        return
    }

    report $file_stem "!"
    mkdir $out_dir
    $new_content | save -f $file_path
}

def update-diffable [] {
    for $file in (ls out/x86-64/**/*.asm) {
        let old_content = $file.name | open --raw
        let new_content = $old_content | simplify

        let new_content = if ($new_content | str ends-with "\n") {
            $new_content
        } else {
            $new_content + "\n"
        }

        let new_content = $new_content | str replace -a 'L__unnamed__' "L__unnamed_"

        $new_content | save -f $file.name
    }
}

def --wrapped main [
  target: string # Target directory under './out'.
  --filter: string # Only check functions whose path contains this string.
  --help (-h) # Display the help message for this command
  --update-diffable # Update the existing asm output with more diffable labels
  ...args # Arguments for `cargo asm` (cargo-show-asm)
]: nothing -> nothing {
    if $update_diffable {
        if $target != "x86-64" {
            error make { msg: "target not supported for `--update_diffable`" }
        }

        update-diffable
        return
    }

    mut names = []

    for name in [allocate, deallocate, grow, shrink] {
        $names ++= $"($name)::up"
        $names ++= $"($name)::down"
        $names ++= $"($name)::bumpalo"
    }

    for try in ["", try_] {
        $names ++= $"alloc_layout::($try)up"
        $names ++= $"alloc_layout::($try)down"
        $names ++= $"alloc_layout::($try)bumpalo"
    }

    for ty in [zst, u8, u32, vec3, 12_u32, big, str, u32_slice, u32_slice_clone] {
        for prefix in ["", try_] {
            $names ++= $"alloc_($ty)::($prefix)up"
            $names ++= $"alloc_($ty)::($prefix)up_a"
            $names ++= $"alloc_($ty)::($prefix)down"
            $names ++= $"alloc_($ty)::($prefix)down_a"
            $names ++= $"alloc_($ty)::($prefix)bumpalo"
        }
    }

    for ty in [u32, big_ok] {
        for prefix in ["", try_] {
            $names ++= $"alloc_try_($ty)::($prefix)up"
            $names ++= $"alloc_try_($ty)::($prefix)up_mut"
            $names ++= $"alloc_try_($ty)::($prefix)down"
            $names ++= $"alloc_try_($ty)::($prefix)down_mut"
            $names ++= $"alloc_try_($ty)::($prefix)bumpalo"
        }
    }

    for dir in [up, down, down_big] {
        $names ++= $"alloc_overaligned_but_size_matches::($dir)"
    }

    # for prefix in ["", try_] {
    #     $names ++= $"alloc_with_drop::($prefix)up"
    #     $names ++= $"alloc_with_drop::($prefix)up_a"
    #     $names ++= $"alloc_with_drop::($prefix)down"
    #     $names ++= $"alloc_with_drop::($prefix)down_a"
    # }

    for ty in [u32] {
        for dir in [up, down] {
            for try in ["", try_] {
                $names ++= $"bump_vec_($ty)::($dir)::($try)with_capacity"
                $names ++= $"bump_vec_($ty)::($dir)::($try)push"
            }
        }
    }

    for ty in [u32, u32_bump_vec] {
        let prefixes = if $ty == "u32" {
            ["", exact_, mut_, mut_rev_]
        } else {
            ["", rev_]
        }

        let tries = if $ty == "u32" {
            ["", try_]
        } else {
            [""]
        }

        for try in $tries {
            for prefix in $prefixes {
                $names ++= $"alloc_iter_($ty)::($try)($prefix)up"
                $names ++= $"alloc_iter_($ty)::($try)($prefix)up_a"
                $names ++= $"alloc_iter_($ty)::($try)($prefix)down"
                $names ++= $"alloc_iter_($ty)::($try)($prefix)down_a"
            }
        }

        if $ty == "u32" {
            $names ++= $"alloc_iter_($ty)::bumpalo"
        }
    }

    for try in ["", try_] {
        for $mut in ["", mut_] {
            $names ++= $"alloc_fmt::($try)($mut)up"
            $names ++= $"alloc_fmt::($try)($mut)up_a"
            $names ++= $"alloc_fmt::($try)($mut)down"
            $names ++= $"alloc_fmt::($try)($mut)down_a"
        }
    }

    for try in ["", try_] {
        for which in [same, grow, shrink] {
            $names ++= $"vec_map::($try)($which)"
        }
    }

    if $filter != null {
        $names = ($names | filter { str contains $filter })
    }

    for $name in $names {
        asm-save $name $target $args
    }
}
