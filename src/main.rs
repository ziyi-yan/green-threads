//! green-threads is a toy implementation on user-space threads in non-preemptive multitasking.
//! This implementation is mostly guided by cfsamson's tutorial:
//! https://cfsamson.gitbook.io/green-threads-explained-in-200-lines-of-rust/green-threads.
#![deny(missing_docs)]
#![feature(llvm_asm)]

/// SSIZE is a small stack size in bytes for easy debugging.
const SSIZE: isize = 48;

/// #[repr(C)] tells compiler to use a stable ABI in C way for struct ThreadContext.
#[derive(Debug, Default)]
#[repr(C)]
struct ThreadContext {
    rsp: u64, // the stack pointer
}

fn hello() -> ! {
    println!("I LOVE WAKING UP ON A NEW STACK!");

    loop {}
}

/// gt_switch switch over to new stack.
///
/// Register %rsp description from page 33 in https://github.com/hjl-tools/x86-psABI/wiki/x86-64-psABI-1.0.pdf:
///     The stack pointer holds the address of the byte with lowest address which is part of the stack.
///     It is guaranteed to be 16-byte aligned at process entry.
unsafe fn gt_switch(new: *const ThreadContext) {
    llvm_asm!(
        "
        mov 0x00($0), %rsp
        ret
        "
        :
        : "r"(new)
        :
        : "alignstack" // it woll work without this now, will need it later
    );
}

fn main() {
    let mut ctx = ThreadContext::default();
    let mut stack = vec![0_u8; SSIZE as usize];
    unsafe {
        let stack_bottom = stack.as_mut_ptr().offset(SSIZE);
        println!("stack_bottom: {:?}", stack_bottom as usize);
        let sb_aligned = (stack_bottom as usize & !15) as *mut u8; // align backwards to 16 bytes
        println!("sb_aligned: {:?}", sb_aligned as usize);
        std::ptr::write(sb_aligned.offset(-16) as *mut u64, hello as u64); // another -16 bytes offset for alignment reason
        ctx.rsp = sb_aligned.offset(-16) as u64;

        // print the stack,
        // remember the stack starts on the top and the address should align to the top in 16 bytes.
        print_stack(stack.as_ptr());

        gt_switch(&mut ctx);
    }
}

// print stack from the top to bottom.
unsafe fn print_stack(stack_ptr: *const u8) {
    for i in (0..SSIZE).rev() {
        println!(
            "{}: mem: {}, val: {}",
            i,
            stack_ptr.offset(i) as usize,
            *stack_ptr.offset(i)
        );
    }
}
