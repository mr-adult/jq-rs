# JQ

This is a WIP rust-based rewrite of the popular [JQ command-line JSON processor](https://github.com/jqlang/jq). It has some simple goals:

1. bring most of JQ's functionality to native Rust
1. expose JQ's query language via proc-macros to compile into native rust code and execute as fast or faster than JQ's bytecode-based system
1. avoid file system access to allow compilation to WASM and environments where there may not be a file system.

`jq` is like `sed` for JSON data - you can use it to slice and filter and map and transform structured data with the same ease that sed, awk, grep and friends let you play with text.

In the current state, the `jq` query language has not been implemented. Instead, you can get the `jq` functionality by using the iterators.

The current feature parity status of this crate is as follows:
- Identity: `.`
    - [x] implemented via iterators
    - [ ] implemented via proc-macro
- 

|Feature |Implemented?|Available in proc-macro form?|
|--------|------------|-----------------------------|
|Identity| - [x]      | - [ ]                       |
