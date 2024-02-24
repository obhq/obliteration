use crate::signal::{SigChldFlags, Signal, SignalFlags, SignalSet, SIGCHLD, SIG_MAXSIG};

/// An implementation of `sigacts` structure.
#[derive(Debug)]
pub struct SignalActs {
    handler: [usize; SIG_MAXSIG as usize],       // ps_sigact
    catchmask: [SignalSet; SIG_MAXSIG as usize], // ps_catchmask
    stack: SignalSet,                            // ps_sigonstack
    interupt: SignalSet,                         // ps_sigintr
    reset: SignalSet,                            // ps_sigreset
    nodefer: SignalSet,                          // ps_signodefer
    modern: SignalSet,                           // ps_siginfo
    ignore: SignalSet,                           // ps_sigignore
    catch: SignalSet,                            // ps_sigcatch
    flag: SigChldFlags,                          // ps_flag
}

impl SignalActs {
    pub(super) fn new() -> Self {
        Self {
            handler: [0; SIG_MAXSIG as usize],
            catchmask: [SignalSet::default(); SIG_MAXSIG as usize],
            stack: SignalSet::default(),
            interupt: SignalSet::default(),
            reset: SignalSet::default(),
            nodefer: SignalSet::default(),
            modern: SignalSet::default(),
            ignore: SignalSet::default(),
            catch: SignalSet::default(),
            flag: SigChldFlags::empty(),
        }
    }

    pub fn handler(&self, sig: Signal) -> usize {
        self.handler[(sig.get() - 1) as usize]
    }

    pub fn set_handler(&mut self, sig: Signal, h: usize) {
        self.handler[(sig.get() - 1) as usize] = h;
    }

    pub fn catchmask(&self, sig: Signal) -> SignalSet {
        self.catchmask[(sig.get() - 1) as usize]
    }

    pub fn set_catchmask(&mut self, sig: Signal, mask: SignalSet) {
        self.catchmask[(sig.get() - 1) as usize] = mask;
    }

    pub fn remove_stack(&mut self, sig: Signal) {
        self.stack.remove(sig);
    }

    pub fn set_interupt(&mut self, sig: Signal) {
        self.interupt.add(sig);
    }

    pub fn remove_reset(&mut self, sig: Signal) {
        self.reset.remove(sig);
    }

    pub fn remove_nodefer(&mut self, sig: Signal) {
        self.nodefer.remove(sig);
    }

    pub fn set_modern(&mut self, sig: Signal) {
        self.modern.add(sig);
    }

    pub fn remove_ignore(&mut self, sig: Signal) {
        self.ignore.remove(sig);
    }

    pub fn set_catch(&mut self, sig: Signal) {
        self.catch.add(sig);
    }

    pub fn remove_catch(&mut self, sig: Signal) {
        self.catch.remove(sig);
    }

    pub fn flag(&self) -> SigChldFlags {
        self.flag
    }

    pub fn set_flag(&mut self, flag: SigChldFlags) {
        self.flag = flag;
    }

    pub fn signal_flags(&self, sig: Signal) -> SignalFlags {
        let mut flags: SignalFlags = SignalFlags::empty();

        if self.stack.contains(sig) {
            flags |= SignalFlags::SA_ONSTACK;
        }
        if self.interupt.contains(sig) {
            flags |= SignalFlags::SA_RESTART;
        }
        if self.reset.contains(sig) {
            flags |= SignalFlags::SA_RESETHAND;
        }
        if self.nodefer.contains(sig) {
            flags |= SignalFlags::SA_NODEFER;
        }
        if self.modern.contains(sig) {
            flags |= SignalFlags::SA_SIGINFO;
        }

        if sig == SIGCHLD {
            if self.flag.intersects(SigChldFlags::PS_NOCLDSTOP) {
                flags |= SignalFlags::SA_NOCLDSTOP;
            }
            if self.flag.intersects(SigChldFlags::PS_NOCLDWAIT) {
                flags |= SignalFlags::SA_NOCLDWAIT
            }
        }

        flags
    }
}
