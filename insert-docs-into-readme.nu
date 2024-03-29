# This was written for nushell version 0.91.0

def each-even [closure] {
  enumerate | each { |e| 
    if $e.index mod 2 == 0 { 
      $e.item | do $closure
    } else {
      $e.item
    }
  }
}

def each-odd [closure] {
  enumerate | each { |e| 
    if $e.index mod 2 == 0 { 
      $e.item
    } else {
      $e.item | do $closure
    }
  }
}

let start_marker = '[//]: # (START_OF_CRATE_DOCS)'
let end_marker = '[//]: # (END_OF_CRATE_DOCS)'

let content = open README.md

let start_index = $content | str index-of $start_marker
let end_index = $content | str index-of $end_marker

let before = $content | str substring ..$start_index
let after = $content | str substring ($end_index + ($end_marker | str length))..

let docs = open src/lib.rs
| parse --regex '//!(?<doc>.*)' 
| get doc 
| each { str replace --regex '^ ' '' } 
| str join "\n"

# fix code blocks, smaller headings
let docs = $docs 
| split row '```'
| each-even { str replace --all --regex '(?m:^# )' '## ' }
| each-odd { str replace --all --regex '(?m:^ *#.*\n)' '' | prepend 'rust' | str join }
| str join '```'

# remove implicit links
let docs = $docs
| str replace --all --regex '\[(?<label>`[^\]]+`)\](?!\(|:)' '$label'

# remove explicit links that are not section or web links
let docs = $docs 
| str replace --all --regex '\[(?<label>[^\]]+)\](\((?!#|http)[^\)]+\))' '$label'

let new_content = [$before, $start_marker, "\n\n", $docs, "\n\n", $end_marker, $after] | str join

$new_content | save -f README.md