# Written for nushell version 0.105 and rustdoc json format version 46.

let json = open target/doc/bump_scope.json
let package_names = open --raw Cargo.lock | from toml | get package.name

def remove-null []: list -> list {
    each {}
}

def children [id: int]: nothing -> list<int> {
    let item = $json.index | get ($id | into string)

    # `each` is like `filter_map`. It is used here to
    match $item.inner {
        {module: $module} => $module.items,
        {use: $use} => ([$use.id] | remove-null),
        {union: $union} => ($union.fields ++ $union.impls),
        {struct: $struct} => ($struct.impls ++ match $struct.kind {
            "unit" => []
            {tuple: $tuple} => ($tuple | remove-null)
            {plain: $plain} => $plain.fields,
        }),
        {enum: $enum} => ($enum.variants ++ $enum.impls),
        {variant: $variant} => (match $variant.kind {
            "unit" => {}
            {tuple: $tuple} => ($tuple | remove-null)
            {struct: $struct} => $struct.fields,
        }),
        {trait: $trait} => ($trait.items ++ $trait.implementations),
        {impl: $impl} => $impl.items,
        {primitive: $primitive} => $primitive.impls,
        _ => []
    }
}

def keys []: record -> list<string> {
    transpose key value | get key
}

def values []: record -> list {
    transpose key value | get value
}

def table-into-record [key_column: cell-path, value_column: cell-path]: table -> record {
    reduce --fold {} { |it, acc| $acc | upsert ($it | get $key_column) ($it | get $value_column) }
}

let parents = $json.index | keys | into int | reduce --fold {} { |parent, acc| (children $parent | reduce --fold $acc { |child, acc| $acc | upsert $"($child)" $parent } ) }

def package-name [crate: string]: nothing -> string {
    $package_names | where { ($in | str replace --all '-' '_' ) == $crate } | get 0
}

def panic [msg: string] {
    error make { msg: $msg }
}

def todo [why?: string] {
    mut msg = "not yet implemented"
    if $why != null { $msg += ": " + $why }
    error make { msg: $msg }
}

def item [id: int]: nothing -> record {
    let id_s = $id | into string
    let item = $json.index | get -i $id_s
    
    if $item != null {
        let name = $item.name
        let kind = $item.inner | keys | get 0
        let parent_id = $parents | get -i $id_s
        let parent = if $parent_id == null { null } else { {id: $parent_id} }
        return {name: $name, kind: $kind, parent: $parent}
    }

    let item_summary = $json.paths | get -i $id_s

    if $item_summary != null {
        let name = $item_summary.path | last
        let kind = $item_summary.kind
        let parent_path = $item_summary.path | take (($item_summary.path | length) - 1)
        let parent = if $parent_path == [] { null } else { {path: $parent_path} }
        return {name: $name, kind: $kind, parent: $parent}
    }

    panic $"can't resolve item ($id)"
}

def item-path [id: int]: nothing -> list<record> {
    mut id = $id
    mut item_path = []

    loop {
        let item = item $id

        $item_path ++= [($item | select name kind)]

        match $item.parent {
            {id: $parent_id} => {
                $id = $parent_id
            }
            {path: $parent_path} => {
                mut $path = $parent_path

                loop {
                    let found = $json.paths | values | where { |it| $it.path == $path } | get -i 0
                    let name = $path | last
                    let kind = $found | get -i kind | default module

                    $item_path ++= [{name: $name, kind: $kind}]
                    $path = $path | take (($path | length) - 1)

                    if $path == [] {
                        break
                    }
                }

                break
            }
            null => { break }
            _ => { todo $"($item)" }
        }
    }

    $item_path
}

def replace-range [range: range, values: list]: list -> list {
    let lhs = $in | slice ..(($range | first) - 1)
    let rhs = $in | slice (($range | last) + 1)..
    $lhs ++ $values ++ $rhs
}

def fuse-impl-function-to-method []: list -> list {
    let path = $in
    let index = $path | get kind | window 2 | enumerate | where item == [impl function] | get -i 0.index
    if $index == null { return $path }
    let name = $path | get ($index + 1) | get name
    $path | replace-range $index..($index + 1) [{name: $name, kind: method}]
}

def item-url [id: number]: nothing -> string {
    item-path $id
    | reverse
    | fuse-impl-function-to-method
    | enumerate
    | each { |it| 
        let name = $it.item.name
        let kind = if $it.index == 0 { "crate" } else { $it.item.kind }

        match $kind {
            "module" => $"($name)/",
            "struct" => $"struct.($name).html",
            "enum" => $"enum.($name).html",
            "union" => $"union.($name).html",
            "macro" => $"macro.($name).html",
            "function" => $"fn.($name).html"
            "method" => $"#method.($name)"
            "trait" => $"trait.($name).html",
            "crate" => {
                if $name in [core, alloc, std] {
                    $"https://doc.rust-lang.org/($name)/"
                } else {
                    let package = package-name $name
                    $"https://docs.rs/($package)/latest/($name)/"
                }
            }
            "impl" | "use" => ""
            _ => {
                todo $"path segment for '($kind)' \(($name)\)"
            }
        }
    }
    | str join
}

def replace-section [section_name: string, new_content: string]: string -> string {
    let readme = $in

    let start_marker = $"<!-- ($section_name) start -->"
    let end_marker = $"<!-- ($section_name) end -->"

    let start_index = ($readme | str index-of $start_marker) + ($start_marker | str length)
    let end_index = $readme | str index-of $end_marker --range $start_index..

    let before = $readme | str substring ..<$start_index
    let after = $readme | str substring $end_index..

    $before ++ $new_content ++ $after
}

# quasi-polyfill of `str replace --regex --all` with a closure parameter.
def replace [regex: string, get_replacement: closure]: string -> string {
    parse --regex ('(?<__before>[\s\S]*?)(?<__matched>' ++ $regex ++  '|$)')
    | each { |it|
        if $it.__matched == "" { return $it.__before }
        let captures = $it | reject __before __matched | insert all $it.__matched
        let replacement = (do $get_replacement $captures)
        $"($it.__before)($replacement)"
    }
    | str join
}

let links = $json.index 
| get ($json.root | into string) 
| get links
| transpose link id
| insert url { |it| try { item-url $it.id } catch { null } } 
| table-into-record link url

let docs = $json.index 
| get ($json.root | into string)
| get docs
# Separate code sections from the rest to process them independently.
| parse --regex '(?<prose>[\s\S]*?)(?<outer_code>```(?<code>[\s\S]*?)```|$)'
| each { |it| 
    let new_prose = $it.prose 
    # Replace rustdoc links with the url.
    | replace '\[(?<text>[^\]]*)\](?:\((?<inline>[^\)]*)\)|\[(?<reference>[^\]]*)\])?' { |it| 
        if $it.reference != "" {
            # `[foo][bar]`
            # We ignore those for now.
            return $it.all
        }

        let link = if $it.inline != "" {
            # `[foo](bar)`
            $it.inline
        } else {
            # `[foo]`
            $it.text
        }

        if $link not-in ($links | keys) {
            # Link is not something rustdoc related. Return it as is.
            return $it.all
        }
    
        let url = $links | get $link
        
        if $url == null {
            # A rustdoc link that could not be resolved. 
            # Lets remove the link and just retain the text.
            print -e $"Could not resolve doc link for `($link)`."
            return $it.text
        }
        
        # The url does not yet contain the hash part of the link so lets append it.
        let hash = $link | split row -n 2 '#' | skip 1 | each { prepend '#' | str join } | str join

        $"[($it.text)]\(($url ++ $hash)\)"
    }
    | str join
    # Add another hash to the markdown headers given that the readme wants to reserve h1 for the title.
    | str replace --all --regex '(?m:^#)' '##'

    if $it.outer_code == "" {
        return $new_prose
    }

    # Remove hidden (`#` prefixed) lines from doc tests.
    let new_code = $it.code | str replace --all --regex '(?m:^ *#.*\n)' ''
    $'($new_prose)```rust($new_code)```'
}
| str join

open README.md
| replace-section "crate docs" $"\n\n($docs)\n\n"
| save -f README.md