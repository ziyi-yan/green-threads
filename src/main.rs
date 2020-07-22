//! green-threads is a toy implementation on user-space threads in non-preemptive multitasking.
//! This implementation is mostly guided by cfsamson's tutorial:
//! https://cfsamson.gitbook.io/green-threads-explained-in-200-lines-of-rust/green-threads.
#![deny(missing_docs)]
#![feature(llvm_asm)]
#![feature(naked_functions)]
#![feature(thread_local)]

use std::collections::VecDeque;
use std::mem;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon;

const DEFAULT_STACK_SIZE: usize = 1024 * 1 * 2;
static mut RUNTIME: usize = 0;

#[thread_local]
#[no_mangle]
static WORKER_ID: AtomicUsize = AtomicUsize::new(!1);

/// Runtime schedule and switch threads.
pub struct Runtime {
    current: usize,
    machines: Vec<Machine>,
}

/// This is the real thing running in the cores
pub struct Machine {
    queue: VecDeque<Task>,
    current: Task,
}

/// ThreadContext contains the registers marked as "callee-saved" (preserved across calls)
/// in the specification of x86-64 architecture. They contain all the information
/// we need to resume a thread.
#[derive(Debug, Default)]
#[repr(C)]
struct ThreadContext {
    rsp: u64,
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    rbx: u64,
    rbp: u64,
    rdi: u64,
}

struct Task {
    stack: Vec<u8>,
    ctx: ThreadContext,
}

impl Runtime {
    /// initialize runtime with machines same numbers as cpu cores
    pub fn new() -> Self {
        let mut machines = Vec::new();
        let cpus = num_cpus::get();
        for _ in 0..cpus {
            machines.push(Machine::new());
        }
        Runtime {
            current: 0,
            machines,
        }
    }
    /// store the pointer to runtime
    pub fn init(&self) {
        unsafe {
            let r_ptr: *const Runtime = self;
            RUNTIME = r_ptr as usize;
        }
    }
    /// spawn a coroutine, spread them equally
    pub fn spawn(&mut self, r: fn()) {
        self.machines[self.current].spawn(r);
        self.current += 1;
        if self.current == self.machines.len() {
            self.current = 0;
        }
    }
    /// run all machines in their own thread
    pub fn run(&mut self) {
        rayon::scope(|s| {
            let mut i = 0;
            for m in self.machines.iter_mut() {
                s.spawn(move |_| {
                    WORKER_ID.store(i, Ordering::Relaxed);
                    while m.t_yield() {}
                });
                i += 1;
            }
        })
    }
    fn t_return(&mut self) {
        let id = WORKER_ID.load(Ordering::Relaxed);
        self.machines[id].t_return();
    }
    fn t_yield(&mut self) {
        let id = WORKER_ID.load(Ordering::Relaxed);
        self.machines[id].t_yield();
    }
}

impl Task {
    fn new() -> Self {
        Task {
            stack: vec![0_u8; DEFAULT_STACK_SIZE],
            ctx: ThreadContext::default(),
        }
    }
}

fn cb(foo: fn()) {
    foo();
    guard();
}

impl Machine {
    /// Initialize with a base thread.
    fn new() -> Self {
        let base_r = Task::new();

        Machine {
            queue: VecDeque::new(),
            current: base_r,
        }
    }

    // force call t_return
    fn t_return(&mut self) {
        if self.queue.len() == 0 {
            return;
        }
        let mut next = self.queue.pop_front().unwrap();
        mem::swap(&mut next, &mut self.current);

        unsafe {
            switch(&mut next.ctx, &self.current.ctx);
        }
    }

    fn t_yield(&mut self) -> bool {
        if self.queue.len() == 0 {
            return false;
        }
        let mut next = self.queue.pop_front().unwrap();
        mem::swap(&mut next, &mut self.current);
        self.queue.push_back(next);

        unsafe {
            let last = self.queue.len() - 1;
            switch(&mut self.queue[last].ctx, &self.current.ctx);
        }
        // Prevents compiler from optimizing our code away on Windows.
        self.queue.len() > 0
    }

    /// spawn a function to be executed by runtime
    fn spawn(&mut self, f: fn()) {
        let mut available = Task::new();
        let s_ptr = available.stack.as_mut_ptr();
        self.queue.push_back(available);
        let last_index = self.queue.len() - 1;
        let last = &mut self.queue[last_index];

        let size = last.stack.len();

        unsafe {
            ptr::write(s_ptr.offset((size - 0x10) as isize) as *mut u64, cb as u64);

            last.ctx.rdi = f as u64;
            last.ctx.rsp = s_ptr.offset((size - 0x10) as isize) as u64;
        }
    }
}

fn guard() {
    unsafe {
        let rt_ptr = RUNTIME as *mut Runtime;
        (*rt_ptr).t_return()
    };
}

/// yield_thread is a helper function that lets us call yield from an arbitrary place in our code.
pub fn yield_thread() {
    unsafe {
        let rt_ptr = RUNTIME as *mut Runtime;
        (*rt_ptr).t_yield();
    };
}

#[naked]
#[inline(never)]
unsafe fn switch(old: *mut ThreadContext, new: *const ThreadContext) {
    llvm_asm!("
        mov     %rsp, 0x00($0)
        mov     %r15, 0x08($0)
        mov     %r14, 0x10($0)
        mov     %r13, 0x18($0)
        mov     %r12, 0x20($0)
        mov     %rbx, 0x28($0)
        mov     %rbp, 0x30($0)
        mov     %rdi, 0x38($0)

        mov     0x00($1), %rsp
        mov     0x08($1), %r15
        mov     0x10($1), %r14
        mov     0x18($1), %r13
        mov     0x20($1), %r12
        mov     0x28($1), %rbx
        mov     0x30($1), %rbp
        mov     0x38($1), %rdi
        ret
        "
    :
    :"r"(old), "r"(new)
    :
    : "volatile", "alignstack"
    );
}

fn main() {
    let mut runtime = Runtime::new();
    runtime.init();
    runtime.spawn(|| {
        let id = 1;
        for i in 0..10 {
            println!("thread: {} counter: {}", id, i);
            yield_thread();
        }
        println!("THREAD 1 FINISHED");
    });
    runtime.spawn(|| {
        let id = 2;
        for i in 0..15 {
            println!("thread: {} counter: {}", id, i);
            yield_thread();
        }
        println!("THREAD 2 FINISHED");
    });

    runtime.spawn(|| {});
    runtime.spawn(|| return);

    for _ in 0..10 {
        runtime.spawn(|| {
            yield_thread();
            println!("THREAD mass FINISHED");
        });
    }
    runtime.run();
}
