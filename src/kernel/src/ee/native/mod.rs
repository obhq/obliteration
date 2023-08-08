use super::{EntryArg, ExecutionEngine};
use crate::fs::{VPath, VPathBuf};
use crate::memory::{Protections, VPages};
use crate::rtld::{CodeWorkspaceError, Memory, Module, RuntimeLinker, UnprotectSegmentError};
use crate::syscalls::{Input, Output, Syscalls};
use byteorder::{ByteOrder, LE};
use iced_x86::code_asm::{
    dword_ptr, eax, edi, qword_ptr, r10, r11, r11d, r12, r12d, r8, r9, rax, rbp, rbx, rcx, rdi,
    rdx, rsi, rsp, CodeAssembler,
};
use std::arch::asm;
use std::error::Error;
use std::mem::size_of;
use std::ptr::null_mut;
use std::sync::RwLock;
use thiserror::Error;

/// An implementation of [`ExecutionEngine`] for running the PS4 binary natively.
pub struct NativeEngine<'a, 'b: 'a> {
    rtld: &'a RwLock<RuntimeLinker<'b>>,
    syscalls: &'a Syscalls<'a, 'b>,
    stack: TlsKey, // A pointer to Stack struct.
}

impl<'a, 'b: 'a> NativeEngine<'a, 'b> {
    pub fn new(
        rtld: &'a RwLock<RuntimeLinker<'b>>,
        syscalls: &'a Syscalls<'a, 'b>,
    ) -> Result<Self, NativeEngineError> {
        let stack = match Self::allocate_stack_key() {
            Ok(v) => v,
            Err(e) => return Err(NativeEngineError::AllocateStackTlsFailed(e)),
        };

        Ok(Self {
            rtld,
            syscalls,
            stack,
        })
    }

    /// # SAFETY
    /// No other threads may read or write any module memory.
    pub unsafe fn patch_mods(&mut self) -> Result<Vec<(VPathBuf, usize)>, PatchModsError> {
        let ld = self.rtld.read().unwrap();
        let mut counts: Vec<(VPathBuf, usize)> = Vec::new();

        for module in ld.list() {
            let count = self.patch_mod(module)?;
            let path = module.path();

            counts.push((path.to_owned(), count));
        }

        Ok(counts)
    }

    fn syscalls(&self) -> *const Syscalls<'a, 'b> {
        self.syscalls
    }

    /// # Safety
    /// The caller is responsible for making sure no other alias to the returned [`Stack`].
    #[cfg(unix)]
    unsafe extern "sysv64" fn stack(&self) -> *mut Stack {
        libc::pthread_getspecific(self.stack) as _
    }

    /// # Safety
    /// The caller is responsible for making sure no other alias to the returned [`Stack`].
    #[cfg(windows)]
    unsafe extern "sysv64" fn stack(&self) -> *mut Stack {
        windows_sys::Win32::System::Threading::FlsGetValue(self.stack) as _
    }

    /// # Safety
    /// This may leak the memory if the stack information for the current thread already been set.
    /// In Rust the memory leak is not unsafe but we don't want ignorance people to invoke this without knowing it consequence.
    #[cfg(unix)]
    unsafe fn set_stack(&self, v: Stack) -> *mut Stack {
        let v = Box::into_raw(Box::new(v));
        assert_eq!(libc::pthread_setspecific(self.stack, v as _), 0);
        v
    }

    /// # Safety
    /// This may leak the memory if the stack information for the current thread already been set.
    /// In Rust the memory leak is not unsafe but we don't want ignorance people to invoke this without knowing it consequence.
    #[cfg(windows)]
    unsafe fn set_stack(&self, v: Stack) -> *mut Stack {
        let v = Box::into_raw(Box::new(v));

        assert_ne!(
            windows_sys::Win32::System::Threading::FlsSetValue(self.stack, v as _),
            0
        );

        v
    }

    /// # Safety
    /// No other threads may access the memory of `module`.
    unsafe fn patch_mod(&self, module: &Module) -> Result<usize, PatchModsError> {
        let path = module.path();

        // Patch all executable sections.
        let mem = module.memory();
        let base = mem.addr();
        let mut count = 0;

        for (i, seg) in mem.segments().iter().enumerate() {
            if seg.program().is_none() || !seg.prot().contains(Protections::CPU_EXEC) {
                continue;
            }

            // Unprotect the segment.
            let mut seg = match mem.unprotect_segment(i) {
                Ok(v) => v,
                Err(e) => {
                    return Err(PatchModsError::UnprotectSegmentFailed(
                        path.to_owned(),
                        i,
                        e,
                    ));
                }
            };

            // Patch segment.
            count += self.patch_segment(path, base, mem, seg.as_mut())?;
        }

        Ok(count)
    }

    /// # Safety
    /// No other threads may execute the memory in the code workspace on `mem`.
    unsafe fn patch_segment(
        &self,
        module: &VPath,
        base: usize,
        mem: &Memory,
        seg: &mut [u8],
    ) -> Result<usize, PatchModsError> {
        let addr = seg.as_ptr() as usize;
        let offset = addr - base;
        let mut count = 0;
        let mut i = 0;

        while i < seg.len() {
            let target = &mut seg[i..];
            let offset = offset + i;
            let addr = addr + i;
            let res = if target.starts_with(&[0x48, 0xc7, 0xc0]) {
                // Possible of:
                // mov rax, imm32
                // mov r10, rcx
                // syscall
                self.patch_syscall(module, mem, target, offset, addr)?
            } else if target.starts_with(&[0xcd, 0x44, 0xba, 0xcc, 0xcc, 0xcc, 0xcc]) {
                // int 0x44
                // mov edx, 0xcccccccc
                self.patch_int44(module, mem, target, offset, addr)?
            } else {
                None
            };

            match res {
                Some(v) => {
                    i += v;
                    count += 1;
                }
                None => i += 1,
            }
        }

        Ok(count)
    }

    /// # Safety
    /// No other threads may execute the memory in the code workspace on `mem`.
    unsafe fn patch_syscall(
        &self,
        module: &VPath,
        mem: &Memory,
        target: &mut [u8],
        offset: usize,
        addr: usize,
    ) -> Result<Option<usize>, PatchModsError> {
        // Check if "mov rax, imm32".
        if target.len() < 7 {
            return Ok(None);
        }

        // Check if next instructions are:
        // mov r10, rcx
        // syscall
        if !target[7..].starts_with(&[0x49, 0x89, 0xca, 0x0f, 0x05]) {
            return Ok(None);
        }

        // Build trampoline.
        let id = LE::read_u32(&target[3..]);
        let ret = addr + 7 + 5;
        let tp = match self.build_syscall_trampoline(mem, module, offset + 10, id, ret) {
            Ok(v) => v,
            Err(e) => {
                return Err(PatchModsError::BuildTrampolineFailed(
                    module.to_owned(),
                    offset,
                    e,
                ))
            }
        };

        // Patch "mov rax, imm32" with "jmp rel32".
        let tp = match Self::get_relative_offset(addr + 5, tp) {
            Some(v) => v.to_ne_bytes(),
            None => return Err(PatchModsError::WorkspaceTooFar(module.to_owned())),
        };

        target[0] = 0xe9;
        target[1] = tp[0];
        target[2] = tp[1];
        target[3] = tp[2];
        target[4] = tp[3];
        target[5] = 0xcc;
        target[6] = 0xcc;

        // Patch "mov r10, rcx" and "syscall" with "int3" to catch unknown jump.
        target[7] = 0xcc;
        target[8] = 0xcc;
        target[9] = 0xcc;
        target[10] = 0xcc;
        target[11] = 0xcc;

        Ok(Some(7 + 5))
    }

    /// # Safety
    /// No other threads may execute the memory in the code workspace on `mem`.
    unsafe fn patch_int44(
        &self,
        module: &VPath,
        mem: &Memory,
        target: &mut [u8],
        offset: usize,
        addr: usize,
    ) -> Result<Option<usize>, PatchModsError> {
        // Build trampoline.
        let ret = addr + 2 + 5;
        let tp = match self.build_int44_trampoline(mem, module, offset, ret) {
            Ok(v) => v,
            Err(e) => {
                return Err(PatchModsError::BuildTrampolineFailed(
                    module.to_owned(),
                    offset,
                    e,
                ))
            }
        };

        // Patch "int 0x44" with "nop".
        target[0] = 0x90;
        target[1] = 0x90;

        // Patch "mov edx, 0xcccccccc" with "jmp rel32".
        let tp = match Self::get_relative_offset(addr + 7, tp) {
            Some(v) => v.to_ne_bytes(),
            None => return Err(PatchModsError::WorkspaceTooFar(module.to_owned())),
        };

        target[2] = 0xe9;
        target[3] = tp[0];
        target[4] = tp[1];
        target[5] = tp[2];
        target[6] = tp[3];

        Ok(Some(7))
    }

    /// # Safety
    /// No other threads may execute the memory in the code workspace on `mem`.
    unsafe fn build_syscall_trampoline(
        &self,
        mem: &Memory,
        module: &VPath,
        offset: usize,
        id: u32,
        ret: usize,
    ) -> Result<usize, TrampolineError> {
        let mut asm = CodeAssembler::new(64).unwrap();

        assert_eq!(72, size_of::<Input>());
        assert_eq!(16, size_of::<Output>());

        // Create function prologue.
        asm.push(rbp).unwrap();
        asm.mov(rbp, rsp).unwrap();

        // Clear CF and save rFLAGS.
        asm.clc().unwrap();
        asm.pushfq().unwrap();
        asm.pop(r11).unwrap();
        asm.and(rsp, !15).unwrap(); // Make sure stack is align to 16 bytes boundary.

        // Save registers.
        asm.push(rdi).unwrap();
        asm.push(rsi).unwrap();
        asm.push(rbx).unwrap();
        asm.push(r12).unwrap();
        asm.push(rdx).unwrap();
        asm.push(r11).unwrap();

        // Setup input.
        let module = match mem.push_data(module.to_owned()) {
            Some(v) => v,
            None => return Err(TrampolineError::SpaceNotEnough),
        };

        asm.sub(rsp, 0x50 + 0x10).unwrap();
        asm.mov(rbx, rsp).unwrap();

        asm.mov(dword_ptr(rbx), id).unwrap();
        asm.mov(rax, offset as u64).unwrap();
        asm.mov(qword_ptr(rbx + 0x08), rax).unwrap();
        asm.mov(rax, module as u64).unwrap();
        asm.mov(qword_ptr(rbx + 0x10), rax).unwrap();
        asm.mov(qword_ptr(rbx + 0x18), rdi).unwrap();
        asm.mov(qword_ptr(rbx + 0x20), rsi).unwrap();
        asm.mov(qword_ptr(rbx + 0x28), rdx).unwrap();
        asm.mov(qword_ptr(rbx + 0x30), rcx).unwrap();
        asm.mov(qword_ptr(rbx + 0x38), r8).unwrap();
        asm.mov(qword_ptr(rbx + 0x40), r9).unwrap();

        // Switch to host stack.
        asm.mov(rdi, self as *const Self as u64).unwrap();
        asm.mov(rax, Self::stack as u64).unwrap();
        asm.call(rax).unwrap();

        asm.mov(rsp, qword_ptr(rax)).unwrap();
        asm.and(rsp, !15).unwrap();

        // Set TIB to host stack.
        if cfg!(windows) {
            asm.mov(r12, 0xFFFFFFFFFFFFFFFFu64).unwrap();
            asm.mov(qword_ptr(0x00).gs(), r12).unwrap();
            asm.mov(r12, qword_ptr(rax + 0x10)).unwrap();
            asm.mov(qword_ptr(0x08).gs(), r12).unwrap();
            asm.mov(r12, qword_ptr(rax + 0x08)).unwrap();
            asm.mov(qword_ptr(0x10).gs(), r12).unwrap();
            asm.mov(r12, qword_ptr(rax + 0x28)).unwrap();
            asm.mov(qword_ptr(0x1478).gs(), r12).unwrap();
            asm.mov(r12d, dword_ptr(rax + 0x30)).unwrap();
            asm.mov(dword_ptr(0x1748).gs(), r12d).unwrap();
            asm.mov(r12, rax).unwrap();
        }

        // Invoke our routine.
        asm.mov(rax, rbx).unwrap();
        asm.add(rax, 0x50).unwrap();
        asm.mov(rdx, rax).unwrap();
        asm.mov(rsi, rbx).unwrap();
        asm.mov(rdi, self.syscalls() as u64).unwrap();
        asm.mov(rax, Syscalls::invoke as u64).unwrap();
        asm.call(rax).unwrap();

        // Switch to PS4 stack.
        asm.mov(rsp, rbx).unwrap();

        // Update stack information from TIB.
        if cfg!(windows) {
            asm.mov(rdi, qword_ptr(0x10).gs()).unwrap();
            asm.mov(qword_ptr(r12 + 0x08), rdi).unwrap();
            asm.mov(edi, dword_ptr(0x1748).gs()).unwrap();
            asm.mov(dword_ptr(r12 + 0x30), edi).unwrap();
        }

        // Set TIB to PS4 stack.
        if cfg!(windows) {
            asm.mov(rdi, 0xFFFFFFFFFFFFFFFFu64).unwrap();
            asm.mov(qword_ptr(0x00).gs(), rdi).unwrap();
            asm.mov(rdi, qword_ptr(r12 + 0x20)).unwrap();
            asm.mov(qword_ptr(0x08).gs(), rdi).unwrap();
            asm.mov(rdi, qword_ptr(r12 + 0x18)).unwrap();
            asm.mov(qword_ptr(0x10).gs(), rdi).unwrap();
            asm.mov(qword_ptr(0x1478).gs(), rdi).unwrap();
            asm.xor(edi, edi).unwrap();
            asm.mov(dword_ptr(0x1748).gs(), edi).unwrap();
        }

        // Check error. This mimic the behavior of
        // https://github.com/freebsd/freebsd-src/blob/release/9.1.0/sys/amd64/amd64/vm_machdep.c#L380.
        let mut err = asm.create_label();
        let mut restore = asm.create_label();

        asm.add(rsp, 0x50 + 0x10).unwrap();
        asm.pop(r11).unwrap();
        asm.pop(rdx).unwrap();
        asm.test(rax, rax).unwrap();
        asm.jnz(err).unwrap();

        // Set output.
        asm.mov(rax, qword_ptr(rbx + 0x50)).unwrap();
        asm.mov(rdx, qword_ptr(rbx + 0x58)).unwrap();
        asm.jmp(restore).unwrap();

        // Set CF.
        asm.set_label(&mut err).unwrap();
        asm.or(r11, 1).unwrap();

        // Restore registers.
        asm.set_label(&mut restore).unwrap();
        asm.pop(r12).unwrap();
        asm.pop(rbx).unwrap();
        asm.pop(rsi).unwrap();
        asm.pop(rdi).unwrap();
        asm.mov(rcx, ret as u64).unwrap();
        asm.xor(r8, r8).unwrap();
        asm.xor(r9, r9).unwrap();
        asm.xor(r10, r10).unwrap();

        // Restore rFLAGS.
        asm.mov(r11d, r11d).unwrap(); // Clear the upper dword.
        asm.push(r11).unwrap();
        asm.popfq().unwrap();
        asm.leave().unwrap();

        self.write_trampoline(mem, ret, asm)
    }

    /// # Safety
    /// No other threads may execute the memory in the code workspace on `mem`.
    unsafe fn build_int44_trampoline(
        &self,
        mem: &Memory,
        module: &VPath,
        offset: usize,
        ret: usize,
    ) -> Result<usize, TrampolineError> {
        let mut asm = CodeAssembler::new(64).unwrap();

        // Create stack frame.
        asm.push(rbp).unwrap();
        asm.mov(rbp, rsp).unwrap();
        asm.and(rsp, !15).unwrap(); // Make sure stack is align to 16 bytes boundary.

        asm.push(rdi).unwrap();
        asm.push(rax).unwrap();
        asm.push(rdx).unwrap();
        asm.push(rsi).unwrap();
        asm.push(rbx).unwrap();
        asm.push(r12).unwrap();

        // Switch to host stack.
        asm.mov(rdi, self as *const Self as u64).unwrap();
        asm.mov(rax, Self::stack as u64).unwrap();
        asm.call(rax).unwrap();

        asm.mov(rbx, rsp).unwrap();
        asm.mov(rsp, qword_ptr(rax)).unwrap();
        asm.and(rsp, !15).unwrap();

        // Set TIB to host stack.
        if cfg!(windows) {
            asm.mov(r12, 0xFFFFFFFFFFFFFFFFu64).unwrap();
            asm.mov(qword_ptr(0x00).gs(), r12).unwrap();
            asm.mov(r12, qword_ptr(rax + 0x10)).unwrap();
            asm.mov(qword_ptr(0x08).gs(), r12).unwrap();
            asm.mov(r12, qword_ptr(rax + 0x08)).unwrap();
            asm.mov(qword_ptr(0x10).gs(), r12).unwrap();
            asm.mov(r12, qword_ptr(rax + 0x28)).unwrap();
            asm.mov(qword_ptr(0x1478).gs(), r12).unwrap();
            asm.mov(r12d, dword_ptr(rax + 0x30)).unwrap();
            asm.mov(dword_ptr(0x1748).gs(), r12d).unwrap();
            asm.mov(r12, rax).unwrap();
        }

        // Invoke our routine.
        let module = match mem.push_data(module.to_owned()) {
            Some(v) => v,
            None => return Err(TrampolineError::SpaceNotEnough),
        };

        asm.mov(rdx, module as u64).unwrap();
        asm.mov(rsi, offset as u64).unwrap();
        asm.mov(rdi, self.syscalls() as u64).unwrap();
        asm.mov(rax, Syscalls::int44 as u64).unwrap();
        asm.call(rax).unwrap();

        // Switch to PS4 stack.
        asm.mov(rsp, rbx).unwrap();

        // Update stack information from TIB.
        if cfg!(windows) {
            asm.mov(rax, qword_ptr(0x10).gs()).unwrap();
            asm.mov(qword_ptr(r12 + 0x08), rax).unwrap();
            asm.mov(eax, dword_ptr(0x1748).gs()).unwrap();
            asm.mov(dword_ptr(r12 + 0x30), eax).unwrap();
        }

        // Set TIB to PS4 stack.
        if cfg!(windows) {
            asm.mov(rax, 0xFFFFFFFFFFFFFFFFu64).unwrap();
            asm.mov(qword_ptr(0x00).gs(), rax).unwrap();
            asm.mov(rax, qword_ptr(r12 + 0x20)).unwrap();
            asm.mov(qword_ptr(0x08).gs(), rax).unwrap();
            asm.mov(rax, qword_ptr(r12 + 0x18)).unwrap();
            asm.mov(qword_ptr(0x10).gs(), rax).unwrap();
            asm.mov(qword_ptr(0x1478).gs(), rax).unwrap();
            asm.xor(eax, eax).unwrap();
            asm.mov(dword_ptr(0x1748).gs(), eax).unwrap();
        }

        // Restore registers.
        asm.pop(r12).unwrap();
        asm.pop(rbx).unwrap();
        asm.pop(rsi).unwrap();
        asm.pop(rdx).unwrap();
        asm.pop(rax).unwrap();
        asm.pop(rdi).unwrap();
        asm.leave().unwrap();

        self.write_trampoline(mem, ret, asm)
    }

    /// # Safety
    /// No other threads may execute the memory in the code workspace on `mem`.
    unsafe fn write_trampoline(
        &self,
        mem: &Memory,
        ret: usize,
        mut assembled: CodeAssembler,
    ) -> Result<usize, TrampolineError> {
        // Get workspace.
        let mut mem = match mem.code_workspace() {
            Ok(v) => v,
            Err(e) => return Err(TrampolineError::GetCodeWorkspaceFailed(e)),
        };

        // Align base address to 16 byte boundary for performance.
        let base = mem.addr();
        let offset = match base % 16 {
            0 => 0,
            v => 16 - v,
        };

        // Assemble.
        let addr = base + offset;
        let mut assembled = assembled.assemble(addr as u64).unwrap();

        // Manually JMP rel32 back to patched location.
        assembled.extend({
            // Calculate relative offset.
            let mut code = [0u8; 5];
            let v = match Self::get_relative_offset(addr + assembled.len() + code.len(), ret) {
                Some(v) => v.to_ne_bytes(),
                None => return Err(TrampolineError::ReturnTooFar),
            };

            // Write the instruction.
            code[0] = 0xe9;
            code[1] = v[0];
            code[2] = v[1];
            code[3] = v[2];
            code[4] = v[3];

            code
        });

        // Write the assembled code.
        let output = match mem.as_mut().get_mut(offset..) {
            Some(v) => v,
            None => return Err(TrampolineError::SpaceNotEnough),
        };

        if assembled.len() > output.len() {
            return Err(TrampolineError::SpaceNotEnough);
        }

        output[..assembled.len()].copy_from_slice(&assembled);

        // Seal the workspace.
        mem.seal(offset + assembled.len());

        Ok(addr)
    }

    fn get_relative_offset(from: usize, to: usize) -> Option<i32> {
        let offset = to.wrapping_sub(from) as isize;

        offset.try_into().ok()
    }

    #[cfg(unix)]
    fn allocate_stack_key() -> Result<TlsKey, std::io::Error> {
        use libc::{c_void, pthread_key_create, pthread_key_t};
        use std::mem::MaybeUninit;

        unsafe extern "C" fn dtor(data: *mut c_void) {
            drop(Box::<Stack>::from_raw(data as _));
        }

        let mut key = MaybeUninit::<pthread_key_t>::uninit();
        let err = unsafe { pthread_key_create(key.as_mut_ptr(), Some(dtor)) };

        if err != 0 {
            Err(std::io::Error::from_raw_os_error(err))
        } else {
            Ok(unsafe { key.assume_init() })
        }
    }

    #[cfg(windows)]
    fn allocate_stack_key() -> Result<TlsKey, std::io::Error> {
        use std::ffi::c_void;
        use std::mem::transmute;
        use windows_sys::Win32::System::Threading::{FlsAlloc, FLS_OUT_OF_INDEXES};

        unsafe extern "system" fn dtor(data: *const c_void) {
            drop(Box::<Stack>::from_raw(transmute(data)));
        }

        let key = unsafe { FlsAlloc(Some(dtor)) };

        if key == FLS_OUT_OF_INDEXES {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(key)
        }
    }
}

impl<'a, 'b: 'a> Drop for NativeEngine<'a, 'b> {
    fn drop(&mut self) {
        // Free the stack info for the current thread. When we are here that mean all other threads
        // already been been terminated, which imply that the stack info for those threads has been
        // freed by the TLS destructor.
        #[cfg(unix)]
        unsafe {
            let stack = self.stack();

            if !stack.is_null() {
                // No need to set the value to null because the pthread is not going to call the
                // destructor when the key is deleted.
                drop(Box::from_raw(stack));
            }

            assert_eq!(libc::pthread_key_delete(self.stack), 0);
        }

        #[cfg(windows)]
        unsafe {
            // On Windows the FlsFree() will call the destructor so we don't need to free the
            // data here.
            assert_ne!(
                windows_sys::Win32::System::Threading::FlsFree(self.stack),
                0
            );
        }
    }
}

impl<'a, 'b> ExecutionEngine for NativeEngine<'a, 'b> {
    unsafe fn run(&mut self, mut arg: EntryArg, mut stack: VPages) -> Result<(), Box<dyn Error>> {
        // Check eboot type.
        let ld = self.rtld.read().unwrap();

        if ld.app().file_info().is_none() {
            todo!("statically linked eboot.bin");
        }

        // Get entry point.
        let boot = ld.kernel().unwrap();
        let entry = boot.memory().addr() + boot.entry().unwrap();

        drop(ld);

        // Setup stack information.
        let ps4_start = stack.as_mut_ptr();
        let ps4_end = ps4_start.add(stack.len());
        let si = self.set_stack(Stack {
            host: null_mut(),
            host_start: null_mut(),
            host_end: null_mut(),
            ps4_start,
            ps4_end,
            win32_deallocation: null_mut(),
            win32_guaranteed: 0,
        });

        // Set TIB to PS4 stack.
        #[cfg(windows)]
        asm!(
            "mov rax, gs:[0x10]", // Get TIB.StackLimit.
            "mov [rdi+0x08], rax", // Save host_start.
            "mov rax, gs:[0x08]", // Get TIB.StackBase.
            "mov [rdi+0x10], rax", // Save host_end.
            "mov rax, gs:[0x1478]", // Get TIB.DeallocationStack.
            "mov [rdi+0x28], rax", // Save win32_deallocation.
            "mov eax, gs:[0x1748]", // Get TIB.GuaranteedStackBytes.
            "mov [rdi+0x30], eax", // Save win32_guaranteed.
            "mov rax, 0xFFFFFFFFFFFFFFFF", // EXCEPTION_CHAIN_END
            "mov gs:[0x00], rax", // Set TIB.ExceptionList.
            "mov rax, [rdi+0x20]", // Get ps4_end.
            "mov gs:[0x08], rax", // Set TIB.StackBase.
            "mov rax, [rdi+0x18]", // Get ps4_start.
            "mov gs:[0x10], rax", // Set TIB.StackLimit.
            "mov gs:[0x1478], rax", // Set TIB.DeallocationStack.
            "xor eax, eax",
            "mov gs:[0x1748], eax", // Set TIB.GuaranteedStackBytes.
            in("rdi") si,
            out("rax") _,
        );

        // Jump to the entry point.
        let arg = arg.as_vec();

        asm!(
            "mov rax, rsp",
            "mov [rcx], rax", // Save host stack.
            "mov rsp, [rcx+0x20]", // Get ps4_end.
            "jmp rsi",
            in("rcx") si,
            in("rdi") arg.as_ptr(),
            in("rsi") entry,
            options(noreturn),
        );
    }
}

#[cfg(unix)]
type TlsKey = libc::pthread_key_t;

#[cfg(windows)]
type TlsKey = u32;

#[repr(C)]
struct Stack {
    host: *mut u8,
    host_start: *mut u8, // Only available on Windows.
    host_end: *mut u8,   // Same here.
    ps4_start: *mut u8,
    ps4_end: *mut u8,
    win32_deallocation: *mut u8, // Only available on Windows.
    win32_guaranteed: u32,       // Same here.
}

/// Represents an error when [`NativeEngine`] construction is failed.
#[derive(Debug, Error)]
pub enum NativeEngineError {
    #[error("cannot allocate a TLS key for stack data")]
    AllocateStackTlsFailed(#[source] std::io::Error),
}

/// Errors for [`NativeEngine::patch_mods()`].
#[derive(Debug, Error)]
pub enum PatchModsError {
    #[error("cannot unprotect segment {1} on {0}")]
    UnprotectSegmentFailed(VPathBuf, usize, #[source] UnprotectSegmentError),

    #[error("cannot build a trampoline for {1:#018x} on {0}")]
    BuildTrampolineFailed(VPathBuf, usize, #[source] TrampolineError),

    #[error("workspace of {0} is too far")]
    WorkspaceTooFar(VPathBuf),
}

/// Errors for trampoline building.
#[derive(Debug, Error)]
pub enum TrampolineError {
    #[error("cannot get code workspace")]
    GetCodeWorkspaceFailed(#[source] CodeWorkspaceError),

    #[error("the remaining workspace is not enough")]
    SpaceNotEnough,

    #[error("the address to return is too far")]
    ReturnTooFar,
}
