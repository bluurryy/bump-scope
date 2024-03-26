def main [
    format?: string,
] {
    let bump_scope = $env.FILE_PWD | path dirname
    let target = "allocator"

    mut flags = [
        target/x86_64-unknown-linux-gnu/coverage/x86_64-unknown-linux-gnu/release/($target)
        --instr-profile=coverage/($target)/coverage.profdata
        --Xdemangler=rustfilt
        --ignore-filename-regex=cargo/registry
        
        $"($bump_scope)/src/allocator.rs"
    ]

    if $format == "html" {
        $flags ++= [
            --format=html
        ]

        let page_path = $"coverage/($target)/index.html"
        ^cargo cov -- show ...$flags | save -f $page_path
    } else {
        $flags ++= [
            --use-color
        ]

        print $flags
        cargo cov -- show ...$flags
    }
}