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

### Factorial

```Tcl
proc fact {x} {
    if {== $x 0} {
        return 1
    }
    return [* [fact [- $x 1]] $x]
}

puts [fact 5]
```
