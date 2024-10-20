// SPDX-License-Identifier: MIT OR Apache-2.0
use super::GdbRegs;
use std::sync::mpsc::{Receiver, Sender};

pub fn channel() -> (Debuggee, Debugger) {
    let (sender, rx) = std::sync::mpsc::channel();
    let (tx, receiver) = std::sync::mpsc::channel();
    let debuggee = Debuggee::new(sender, receiver);
    let debugger = Debugger {
        receiver: rx,
        sender: tx,
    };

    (debuggee, debugger)
}

/// Encapsulates channels to communicate with a debuggee thread.
///
/// All method need a mutable reference to prevent request-response out of sync.
pub struct Debuggee {
    sender: Sender<DebugReq>,
    receiver: Receiver<DebugRes>,
    locked: bool,
}

impl Debuggee {
    fn new(sender: Sender<DebugReq>, receiver: Receiver<DebugRes>) -> Self {
        Self {
            sender,
            receiver,
            locked: false,
        }
    }

    pub fn get_regs(&mut self) -> Option<GdbRegs> {
        self.sender.send(DebugReq::GetRegs).ok()?;
        self.locked = true;
        self.receiver
            .recv()
            .map(|v| match v {
                DebugRes::Regs(v) => v,
            })
            .ok()
    }

    pub fn lock(&mut self) {
        self.sender.send(DebugReq::Lock).ok();
        self.locked = true;
    }

    pub fn release(&mut self) {
        if std::mem::take(&mut self.locked) {
            self.sender.send(DebugReq::Release).ok();
        }
    }
}

/// Encapsulates channels to communicate with a debugger thread.
pub struct Debugger {
    receiver: Receiver<DebugReq>,
    sender: Sender<DebugRes>,
}

impl Debugger {
    pub fn recv(&self) -> Option<DebugReq> {
        self.receiver.recv().ok()
    }

    pub fn send(&self, r: DebugRes) {
        self.sender.send(r).ok();
    }
}

/// Debug request from a debugger to a debuggee.
pub enum DebugReq {
    GetRegs,
    Lock,
    Release,
}

/// Debug response from a debuggee to a debugger.
pub enum DebugRes {
    Regs(GdbRegs),
}
