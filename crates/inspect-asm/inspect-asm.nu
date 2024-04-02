# This was written for nushell version 0.91.0 and cargo-show-asm version 0.2.30

def group-by-maybe-empty [] {
  let list = $in

  if ($list | length) == 0 {
    []
  } else {
    $list | group-by
  }  
}

def simplify [output: string] {
  # Remove empty lines
  mut output = ($output | lines | filter { ($in | str length) != 0 } | str join "\n")

  # We replace all `.LBB123_45` and such with `.LBB_45`.
  let indexes = $output 
  | parse -r '\.LBB(?<index>[0-9]+)' 
  | get index
  | group-by-maybe-empty
  | transpose value
  | get value

  if ($indexes | length) > 1 {
    error make { msg: "expected .LBB to always start with the same number inside a function" }
  }

  if ($indexes | length) > 0 {
    let index = $indexes.0
    $output = ($output | str replace -a $'.LBB($index)' ".LBB")
  }

  # We replace all `.L__unnamed_11` with indices starting at `0`
  let indexes = $output 
  | parse -r '\.L__unnamed_(?<index>[0-9]+)' 
  | get index 
  | group-by
  | transpose index
  | get index
  | enumerate
  
  for entry in $indexes {
    let old = $entry.item
    let new = $entry.index

    $output = ($output | str replace -a $'.L__unnamed_($old)' $".L__unnamed__($new)")
  }

  $output
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
  let new_content = (simplify $result.stdout)

  if $new_content == $old_content {
    report $file_stem "="
    return
  }
  
  report $file_stem "!"
  mkdir $out_dir
  $new_content | save -f $file_path
}

def --wrapped main [
  target: string # Target directory under './out'.
  --filter: string # Only check functions whose path contains this string.
  --help (-h) # Display the help message for this command
  ...args # Arguments for `cargo asm` (cargo-show-asm)
]: nothing -> nothing {
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

  for ty in [zst u8, u32, vec3, big, str, u32_slice, u32_slice_clone] {
    for prefix in ["", try_] {
      $names ++= $"alloc_($ty)::($prefix)up"
      $names ++= $"alloc_($ty)::($prefix)up_a"
      $names ++= $"alloc_($ty)::($prefix)down"
      $names ++= $"alloc_($ty)::($prefix)down_a"
      $names ++= $"alloc_($ty)::($prefix)bumpalo"
    }
  }

  # for prefix in ["", try_] {
  #   $names ++= $"alloc_with_drop::($prefix)up"
  #   $names ++= $"alloc_with_drop::($prefix)up_a"
  #   $names ++= $"alloc_with_drop::($prefix)down"
  #   $names ++= $"alloc_with_drop::($prefix)down_a"
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

  if $filter != null {
    $names = ($names | filter { str contains $filter })
  }

  for $name in $names {
    asm-save $name $target $args
  }
}