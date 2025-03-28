## PicolRust

Implementation of the [Picol](https://github.com/antirez/picol/tree/main) interpreter in Rust. 

## Instructions 

To run the interpreter, 
`cargo run -- <path-to-tcl-file>`

## Samples

### Square
```Tcl
proc square {x} {
    * $x $x
}

puts [square 5]
```
