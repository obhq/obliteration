use crate::module::Memory;
use iced_x86::{Code, Decoder, DecoderOptions, OpKind};
use std::collections::{HashMap, VecDeque};
use thiserror::Error;

/// Contains state of module disassemble.
pub(super) struct Disassembler<'a> {
    module: &'a Memory,
    functions: HashMap<usize, Function>, // Key is the offset in the mapped memory.
}

impl<'a> Disassembler<'a> {
    pub fn new(module: &'a Memory) -> Self {
        Self {
            module,
            functions: HashMap::new(),
        }
    }

    /// `offset` is an offset of the target **function** in the mapped memory.
    pub fn disassemble(&mut self, offset: usize) -> Result<(), DisassembleError> {
        let mut jobs = VecDeque::from([offset]); // Function offset.

        while let Some(job) = jobs.pop_front() {
            // Check if the offset is already disassembled.
            if self.functions.contains_key(&job) {
                continue;
            }

            // Disassemble.
            let func = self.disassemble_single(job)?;

            jobs.extend(&func.calls);

            if self.functions.insert(job, func).is_some() {
                panic!("Function {job} is already disassembled.");
            }
        }

        Ok(())
    }

    pub fn fixup(&mut self) {
        // TODO: Fixup all disassembled function.
    }

    pub fn get(&self, offset: usize) -> Option<&Function> {
        self.functions.get(&offset)
    }

    fn disassemble_single(&mut self, offset: usize) -> Result<Function, DisassembleError> {
        // Setup the decoder.
        let module = self.module.as_ref();
        let base = module.as_ptr() as u64;
        let decoder = Decoder::with_ip(
            64,
            &module[offset..],
            base + (offset as u64),
            DecoderOptions::AMD,
        );

        // Decode the whole function.
        let mut func = Function {
            params: Vec::new(),
            returns: Vec::new(),
            instructions: Vec::new(),
            calls: Vec::new(),
            refs: Vec::new(),
        };

        for i in decoder {
            // If the instruction is not valid that mean it is (likely) the end of function.
            if i.is_invalid() {
                break;
            }

            // Parse the instruction.
            let offset = (i.ip() - base) as usize;

            match i.code() {
                Code::Xor_rm64_r64 => self.disassemble_xor(&i, &mut func),
                _ => {
                    let opcode = &module[offset..(offset + i.len())];

                    return Err(DisassembleError::UnknownInstruction(
                        offset,
                        opcode.into(),
                        i,
                    ));
                }
            }
        }

        Ok(func)
    }

    fn disassemble_xor(&self, i: &iced_x86::Instruction, f: &mut Function) {
        let i = if i.op0_kind() == OpKind::Memory {
            if i.has_lock_prefix() {
                panic!("XOR with LOCK prefix is not supported yet.");
            } else {
                panic!("XOR with the first operand is a memory is not supported yet.");
            }
        } else {
            let dst: Operand = i.op0_register().into();
            let src: Operand = match i.op1_kind() {
                OpKind::Register => i.op1_register().into(),
                _ => panic!(
                    "XOR with the second operand is {:?} is not supported yet.",
                    i.op1_kind()
                ),
            };

            Instruction::Xor(dst, src)
        };

        f.instructions.push(i);
    }
}

/// Represents a disassembled function.
pub(super) struct Function {
    params: Vec<Param>,
    returns: Vec<iced_x86::Register>,
    instructions: Vec<Instruction>,
    calls: Vec<usize>,
    refs: Vec<usize>,
}

impl Function {
    pub fn instructions(&self) -> &[Instruction] {
        self.instructions.as_ref()
    }

    /// Gets a slice of the offset this function call to.
    pub fn calls(&self) -> &[usize] {
        self.calls.as_ref()
    }

    /// Gets a slice of the offset whose calling this function.
    pub fn refs(&self) -> &[usize] {
        self.refs.as_ref()
    }
}

/// Represents a function parameter.
pub(super) enum Param {}

/// Represents a CPU instruction.
pub(super) enum Instruction {
    Xor(Operand, Operand),
}

/// Represents the operand of the instruction.
pub(super) enum Operand {
    Rbp(usize),
}

impl From<iced_x86::Register> for Operand {
    fn from(value: iced_x86::Register) -> Self {
        use iced_x86::Register;

        match value {
            Register::RBP => Self::Rbp(64),
            _ => panic!("Register {value:?} is not supported yet."),
        }
    }
}

/// Represents an error for [`Disassembler::disassemble()`].
#[derive(Debug, Error)]
pub enum DisassembleError {
    #[error("unknown instruction '{2}' ({1:02x?}) at {0:#018x}")]
    UnknownInstruction(usize, Vec<u8>, iced_x86::Instruction),
}
