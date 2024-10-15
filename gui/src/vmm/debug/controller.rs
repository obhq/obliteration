// SPDX-License-Identifier: MIT OR Apache-2.0
use std::hint::unreachable_unchecked;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

pub fn debug_controller<T>() -> (Debuggee<T>, Debugger<T>) {
    let state = Mutex::default();
    let signal = Condvar::new();
    let data = Arc::new(Data { state, signal });
    let debuggee = Debuggee {
        data: data.clone(),
        wakeup: false,
    };

    (debuggee, Debugger(data))
}

/// Provides methods for a debugger thread to interrupt a debuggee thread.
pub struct Debuggee<T> {
    data: Arc<Data<T>>,
    wakeup: bool,
}

impl<T> Debuggee<T> {
    pub fn lock(&mut self) -> LockedData<T> {
        let mut s = self.data.state.lock().unwrap();

        loop {
            s = match s.deref() {
                DataState::None => {
                    *s = DataState::Request;
                    self.wakeup = true;
                    self.data.signal.wait(s).unwrap()
                }
                DataState::Request => self.data.signal.wait(s).unwrap(),
                DataState::DebuggerOwned(_) => break,
                DataState::DebuggeeOwned(_) => {
                    // The debugge is not pickup the previous value yet.
                    self.data.signal.wait(s).unwrap()
                }
            };
        }

        LockedData(s)
    }

    pub fn release(&mut self) {
        let mut s = self.data.state.lock().unwrap();

        match std::mem::take(s.deref_mut()) {
            DataState::DebuggerOwned(v) => *s = DataState::DebuggeeOwned(v),
            _ => panic!("attempt to release a lock that is not owned"),
        }

        if std::mem::take(&mut self.wakeup) {
            self.data.signal.notify_one();
        }
    }
}

/// Provides access to the data sent from the debuggee side.
pub struct LockedData<'a, T>(MutexGuard<'a, DataState<T>>);

impl<'a, T> Deref for LockedData<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self.0.deref() {
            DataState::DebuggerOwned(v) => v,
            _ => unsafe { unreachable_unchecked() },
        }
    }
}

impl<'a, T> DerefMut for LockedData<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self.0.deref_mut() {
            DataState::DebuggerOwned(v) => v,
            _ => unsafe { unreachable_unchecked() },
        }
    }
}

/// Provides methods for a debuggee thread to send data to a debugger thread.
pub struct Debugger<T>(Arc<Data<T>>);

impl<T> Debugger<T> {
    pub fn send(&mut self, v: T) -> ResponseHandle<T> {
        // If the debugger has not request for the data yet we can switch to DebuggerOwned without
        // additional works.
        let mut s = self.0.state.lock().unwrap();

        if matches!(s.deref(), DataState::None) {
            *s = DataState::DebuggerOwned(v);

            return ResponseHandle {
                data: &self.0,
                taken: false,
            };
        }

        // Once the debugger has been requested the data it will wait for the data and a signal from
        // us.
        assert!(matches!(s.deref(), DataState::Request));

        *s = DataState::DebuggerOwned(v);

        self.0.signal.notify_one();

        // The debugger will notify us when they are finished with the requested data.
        loop {
            s = match s.deref() {
                DataState::DebuggerOwned(_) => self.0.signal.wait(s).unwrap(),
                DataState::DebuggeeOwned(_) => break,
                _ => unreachable!(),
            };
        }

        *s = DataState::DebuggerOwned(match std::mem::take(s.deref_mut()) {
            DataState::DebuggeeOwned(v) => v,
            _ => unsafe { unreachable_unchecked() },
        });

        // It is possible for us to wakeup after the debugger has reacquired the lock so we need to
        // wake them up. Condvar::notify_one do nothing if there are no any thread waiting on it.
        self.0.signal.notify_one();

        ResponseHandle {
            data: &self.0,
            taken: false,
        }
    }
}

/// Provides method to get a response from the debugger.
pub struct ResponseHandle<'a, T> {
    data: &'a Data<T>,
    taken: bool,
}

impl<'a, T> ResponseHandle<'a, T> {
    pub fn into_response(mut self) -> T {
        let mut s = self.data.state.lock().unwrap();
        let v = match std::mem::take(s.deref_mut()) {
            DataState::DebuggeeOwned(v) => v,
            _ => panic!("the debugger did not release the data"),
        };

        self.taken = true;

        v
    }
}

impl<'a, T> Drop for ResponseHandle<'a, T> {
    fn drop(&mut self) {
        if !self.taken {
            let mut s = self.data.state.lock().unwrap();

            if !matches!(std::mem::take(s.deref_mut()), DataState::DebuggeeOwned(_)) {
                panic!("the debugger did not release the data");
            }
        }

        // It is possible for this method to get called after the debugger has reacquired the lock
        // so we need to wake them up. Condvar::notify_one do nothing if there are no any thread
        // waiting on it.
        self.data.signal.notify_one();
    }
}

/// Contains data sending and receiving between a debugger and a debuggee thread.
struct Data<T> {
    state: Mutex<DataState<T>>,
    signal: Condvar,
}

/// State of the data sending and receiving between a debugger and a debuggee thread.
#[derive(Default)]
enum DataState<T> {
    #[default]
    None,
    Request,
    DebuggerOwned(T),
    DebuggeeOwned(T),
}
