use std::io::Read;
use std::time::Instant;

use crate::bytecode::{Bytecode, Instruction, Opcode};
use crate::heap::{Heap, Word};

pub const STACK_SIZE: usize = 1 << 14;
pub const HEAP_SIZE: usize = 1 << 20;

/// The VM struct
pub struct VM {
    pub bytecode: Bytecode,
    /// Fixed-size stack of words
    stack: [Word; STACK_SIZE],
    /// Stack pointer. Points to the next free slot in the stack.
    stack_ptr: usize,
    /// Instruction pointer
    ip: usize,
    /// The heap
    heap: Heap,
}

impl VM {
    /// Create a new `VM` with the given bytecode
    pub fn new(bytecode: Bytecode) -> Self {
        VM {
            bytecode,
            stack: [Word::from_int(0); STACK_SIZE], // Initialize stack with zeroes
            stack_ptr: 0,
            ip: 0,
            heap: Heap::new(HEAP_SIZE), // The heap
        }
    }

    fn print_state(&mut self) {
        print!("Stack: ");
        for i in 0..self.stack_ptr {
            print!("| {:?} ", self.stack[i]);
        }
        println!("|");

        let opcode: Option<Opcode> = Opcode::from_u8(self.bytecode.instructions[self.ip]);

        println!("IP 0x{:X}: {:?}", self.ip, opcode);
    }

    fn next_byte(&mut self) -> u8 {
        let byte = self.bytecode.instructions[self.ip];
        self.ip += 1;
        byte
    }

    fn next_instr(&mut self) -> Instruction {
        match Opcode::from_u8(self.next_byte()).expect("Invalid bytecode") {
            Opcode::Halt => Instruction::Halt,
            Opcode::Jump => {
                let addr = [self.next_byte(), self.next_byte()];
                Instruction::Jump(u16::from_le_bytes(addr))
            }
            Opcode::Jnz => {
                let addr = [self.next_byte(), self.next_byte()];
                Instruction::Jnz(u16::from_le_bytes(addr))
            }
            Opcode::Jumpi => Instruction::Jumpi,
            Opcode::Dup => Instruction::Dup(self.next_byte()),
            Opcode::Swap => Instruction::Swap(self.next_byte()),
            Opcode::Drop => Instruction::Drop,
            Opcode::Push4 => {
                let arg = [
                    self.next_byte(),
                    self.next_byte(),
                    self.next_byte(),
                    self.next_byte(),
                ];
                Instruction::Push4(i32::from_le_bytes(arg))
            }
            Opcode::Push2 => {
                let arg = [self.next_byte(), self.next_byte()];
                Instruction::Push2(i16::from_le_bytes(arg))
            }
            Opcode::Push1 => Instruction::Push1(self.next_byte() as i8),
            Opcode::Add => Instruction::Add,
            Opcode::Sub => Instruction::Sub,
            Opcode::Mul => Instruction::Mul,
            Opcode::Div => Instruction::Div,
            Opcode::Mod => Instruction::Mod,
            Opcode::Eq => Instruction::Eq,
            Opcode::Ne => Instruction::Ne,
            Opcode::Lt => Instruction::Lt,
            Opcode::Gt => Instruction::Gt,
            Opcode::Le => Instruction::Le,
            Opcode::Ge => Instruction::Ge,
            Opcode::Not => Instruction::Not,
            Opcode::And => Instruction::And,
            Opcode::Or => Instruction::Or,
            Opcode::Input => Instruction::Input,
            Opcode::Output => Instruction::Output,
            Opcode::Alloc => Instruction::Alloc,
            Opcode::Load => {
                let addr = [
                    self.next_byte(),
                    self.next_byte(),
                    self.next_byte(),
                    self.next_byte(),
                ];
                Instruction::Load(u32::from_le_bytes(addr))
            }
            Opcode::Clock => Instruction::Clock,
        }
    }

    fn peek_word(&self, depth: usize) -> Word {
        self.stack[self.stack_ptr - 1 - depth]
    }

    fn peek_word_mut(&mut self, depth: usize) -> &mut Word {
        &mut self.stack[self.stack_ptr - 1 - depth]
    }

    fn pop_word(&mut self) -> Word {
        let word = self.peek_word(0);
        self.stack_ptr -= 1;
        word
    }

    fn push_word(&mut self, w: Word) {
        self.stack[self.stack_ptr] = w;
        self.stack_ptr += 1;
    }

    pub fn run(&mut self) {
        let stdin = std::io::stdin();
        let mut stdin = stdin.lock();
        let start = Instant::now();
        loop {
            match self.next_instr() {
                Instruction::Halt => break,
                Instruction::Jump(addr) => {
                    self.ip = addr as usize;
                }
                Instruction::Jnz(addr) => {
                    let arg = self.pop_word();
                    assert!(!arg.is_pointer());
                    if arg.to_int() != 0 {
                        self.ip = addr as usize;
                    }
                }
                Instruction::Jumpi => {
                    let addr = self.pop_word();
                    assert!(!addr.is_pointer());
                    self.ip = addr.to_int() as usize;
                }
                Instruction::Dup(depth) => {
                    let arg = self.peek_word(depth as usize);
                    self.push_word(arg);
                }
                Instruction::Swap(depth) => {
                    assert!(depth > 0);
                    let mut top = self.pop_word();
                    std::mem::swap(&mut top, self.peek_word_mut((depth - 1) as usize));
                    self.push_word(top);
                }
                Instruction::Drop => {
                    self.pop_word();
                }
                Instruction::Push4(arg) => {
                    self.push_word(Word::from_int(arg));
                }
                Instruction::Push2(arg) => {
                    self.push_word(Word::from_int(arg as i32));
                }
                Instruction::Push1(arg) => {
                    self.push_word(Word::from_int(arg as i32));
                }
                instr @ (Instruction::Add
                | Instruction::Sub
                | Instruction::Mul
                | Instruction::Div
                | Instruction::Mod
                | Instruction::Eq
                | Instruction::Ne
                | Instruction::Lt
                | Instruction::Gt
                | Instruction::Le
                | Instruction::Ge
                | Instruction::And
                | Instruction::Or) => {
                    let b = self.pop_word();
                    assert!(!b.is_pointer());
                    let b = b.to_int();
                    let a = self.pop_word();
                    assert!(!a.is_pointer());
                    let a = a.to_int();
                    let result = match instr {
                        Instruction::Add => a + b,
                        Instruction::Sub => a - b,
                        Instruction::Mul => a * b,
                        Instruction::Div => a / b,
                        Instruction::Mod => a % b,
                        Instruction::Eq => (a == b) as i32,
                        Instruction::Ne => (a != b) as i32,
                        Instruction::Lt => (a < b) as i32,
                        Instruction::Gt => (a > b) as i32,
                        Instruction::Le => (a <= b) as i32,
                        Instruction::Ge => (a >= b) as i32,
                        Instruction::And => a & b,
                        Instruction::Or => a | b,
                        _ => unreachable!(),
                    };
                    self.push_word(Word::from_int(result));
                }
                Instruction::Not => {
                    let arg = self.pop_word();
                    assert!(!arg.is_pointer());
                    self.push_word(Word::from_int((arg.to_int() == 0) as i32));
                }
                Instruction::Input => {
                    let mut char: u8 = 0;
                    stdin.read_exact(std::slice::from_mut(&mut char)).unwrap();
                    self.push_word(Word::from_int(char as i32))
                }
                Instruction::Output => {
                    let arg = self.pop_word();
                    let char = char::from_u32(u32::try_from(arg.to_int()).unwrap()).unwrap();
                    print!("{char}");
                }
                Instruction::Alloc => {
                    let tag = self.pop_word();
                    assert!(!tag.is_pointer());
                    let tag = u8::try_from(tag.to_int()).expect("invalid tag");

                    let size = self.pop_word();
                    assert!(!size.is_pointer());
                    let size = usize::try_from(size.to_int()).expect("invalid size");

                    assert!(size <= self.stack_ptr);
                    let words = &self.stack[self.stack_ptr - size..self.stack_ptr];
                    let pointer = match self.heap.alloc(size, tag, words) {
                        Some(pointer) => pointer,
                        None => {
                            // Attempt a gc cycle
                            let rootset = self.stack[..self.stack_ptr]
                                .iter_mut()
                                .filter(|w| w.is_pointer());
                            self.heap.gc(rootset);
                            // Try to allocate again. This time failure is fatal.
                            let words = &self.stack[self.stack_ptr - size..self.stack_ptr];
                            self.heap
                                .alloc(size, tag, words)
                                .expect("allocation failed")
                        }
                    };
                    self.stack_ptr -= size;
                    self.push_word(Word::from_pointer(pointer));
                }
                Instruction::Load(offset) => {
                    let word = self.pop_word();
                    assert!(word.is_pointer());
                    self.push_word(self.heap.heap[word.to_pointer() + offset as usize]);
                }
                Instruction::Clock => {
                    println!("{:.4}", start.elapsed().as_secs_f64());
                }
            }
        }
    }
}
