# Green Threads

The 200-line implementation of green threads from [this tutorial](https://cfsamson.gitbook.io/green-threads-explained-in-200-lines-of-rust) with comments about the details I learned.

# TODO
- [ ] [Growable stacks](https://blog.cloudflare.com/how-stacks-are-handled-in-go/)
- [X] [x86-64 psABI stack layout and calling convention](https://github.com/hjl-tools/x86-psABI/wiki/X86-psABI)
    - Q: how %rsp register works in stack construction in `spawn()`
        - when `ret`
        - when to move to next byte
    - A: Princeton COS217 [slide](https://www.cs.princeton.edu/courses/archive/spr16/cos217/lectures/15_AssemblyFunctions.pdf) on "Assembly Langauge: Function Calls"
- [X] why `#[inline(never)]` for `switch()`?
    - It's for stack alignment.

      In the perspective of one task, `call switch` sub `%rsp` for storing the `%rip` when suspended.
      When it's resumed, `ret` add `%rsp` back to the original aligned address.