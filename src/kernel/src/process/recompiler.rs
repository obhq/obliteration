use iced_x86::code_asm::{CodeAssembler, CodeLabel};
use iced_x86::{Code, Decoder, DecoderOptions, Instruction};
use std::collections::VecDeque;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::mem::transmute;

pub(super) struct Recompiler<'input> {
    input: &'input [u8],
    assembler: CodeAssembler,
    jobs: VecDeque<(usize, CodeLabel)>,
    output_size: usize,
}

impl<'input> Recompiler<'input> {
    /// `input` is a mapped SELF.
    pub fn new(input: &'input [u8]) -> Self {
        Self {
            input,
            assembler: CodeAssembler::new(64).unwrap(),
            jobs: VecDeque::new(),
            output_size: 0,
        }
    }

    pub fn run(mut self, starts: &[usize]) -> Result<(NativeCode, Vec<*const u8>), RunError> {
        // Recompile all start offset.
        let mut start_addrs: Vec<*const u8> = Vec::new();

        for &start in starts {
            self.recompile(start)?;
        }

        // TODO: Recompile all of references recursively.
        Ok((NativeCode {}, start_addrs))
    }

    fn recompile(&mut self, offset: usize) -> Result<(), RunError> {
        // Setup decoder.
        let input = self.input;
        let base: u64 = unsafe { transmute(input.as_ptr()) };
        let decoder = Decoder::with_ip(
            64,
            &input[offset..],
            base + offset as u64,
            DecoderOptions::AMD,
        );

        // Re-assemble offset until return.
        for i in decoder {
            // Check if instruction valid.
            let offset = (i.ip() - base) as usize;

            if i.is_invalid() {
                return Err(RunError::InvalidInstruction(offset));
            }

            // Transform instruction.
            self.output_size += match i.code() {
                Code::Call_rel32_64 => self.transform_call_rel32(i),
                _ => {
                    return Err(RunError::UnknownInstruction(
                        offset,
                        (&input[offset..(offset + i.len())]).into(),
                        i,
                    ))
                }
            };
        }

        Ok(())
    }

    fn transform_call_rel32(&mut self, i: Instruction) -> usize {
        let label = self.assembler.create_label();
        let offset = self.offset(i.near_branch64());

        self.assembler.call(label).unwrap();
        self.jobs.push_back((offset, label));

        15
    }

    fn offset(&self, addr: u64) -> usize {
        (addr as usize) - unsafe { transmute::<*const u8, usize>(self.input.as_ptr()) }
    }
}

pub struct NativeCode {}

#[derive(Debug)]
pub enum RunError {
    InvalidInstruction(usize),
    UnknownInstruction(usize, Vec<u8>, Instruction),
}

impl Error for RunError {}

impl Display for RunError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            Self::InvalidInstruction(o) => {
                write!(f, "invalid instruction at {:#018x}", o)
            }
            Self::UnknownInstruction(o, r, i) => {
                write!(f, "unknown instruction '{}' ({:x?}) at {:#018x}", i, r, o)
            }
        }
    }
}
