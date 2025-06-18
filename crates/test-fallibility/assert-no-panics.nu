# This was written for nushell version 0.100.0
# and `cargo-show-asm` version 0.2.43

if not ("out" | path exists) { 
    mkdir out 
}

if not ("out/.gitignore" | path exists) { 
    "*" | save -f out/.gitignore 
}

^cargo asm --everything --simplify
| save -f out/everything.asm

let allowed_panics = [
    
]

let panics = open --raw out/everything.asm 
| find panic 
| filter { ($in | ansi strip) not-in $allowed_panics }; 

let message = if ($panics | length) == 0 { 
    "OK" 
} else { 
    $panics | wrap panics 
}

print -e $message

exit ($panics | length)