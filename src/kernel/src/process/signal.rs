use crate::signal::{Signal, SignalSet, SIG_MAXSIG};

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
        }
    }

    pub fn handler(&self, sig: Signal) -> usize {
        self.handler[(sig.get() - 1) as usize]
    }

    pub fn set_handler(&mut self, sig: Signal, h: usize) {
        self.handler[(sig.get() - 1) as usize] = h;
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
}
