# Green Threads

The 200-line implementation of green threads from [this tutorial](https://cfsamson.gitbook.io/green-threads-explained-in-200-lines-of-rust) with comments about the details I learned.

# TODO
- [ ] [Growable stacks](https://blog.cloudflare.com/how-stacks-are-handled-in-go/)
- [ ] [x86-64 psABI stack layout and calling convention](https://github.com/hjl-tools/x86-psABI/wiki/X86-psABI)
    - how %rsp register works in stack construction in `spawn()`
        - when `ret`
        - when to move to next byte
