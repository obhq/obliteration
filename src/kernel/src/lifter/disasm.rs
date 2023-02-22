use crate::module::Memory;
use iced_x86::{Code, Decoder, DecoderOptions};
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
        None
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
                Code::Xor_rm64_r64 => func.instructions.push(Instruction::Other(i)),
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
    Other(iced_x86::Instruction),
}

/// Represents an error for [`Disassembler::disassemble()`].
#[derive(Debug, Error)]
pub enum DisassembleError {
    #[error("unknown instruction '{2}' ({1:02x?}) at {0:#018x}")]
    UnknownInstruction(usize, Vec<u8>, iced_x86::Instruction),
}
