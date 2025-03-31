## PicolRust

Implementation of the [Picol](https://github.com/antirez/picol/tree/main) interpreter in (safe) Rust. 

## Instructions 

To run the interpreter, 
`cargo run -- <path-to-tcl-file>`

## Samples

### Square (Simple Procedures)
```Tcl
proc square {x} {
    * $x $x
}

puts [square 5]
```

### Factorial (Control Flow)

```Tcl
proc fact {x} {
    if {== $x 0} {
        return 1
    }
    return [* [fact [- $x 1]] $x]
}

puts [fact 5]
```

### Sum of First N numbers (Loops)

```Tcl
proc sum {n} {
    set x 0
    set s 0
    while {<= $x $n} {
        set s [+ $s $x]
        set x [+ $x 1]
    }
    return $s
}

puts [sum 5]
```
