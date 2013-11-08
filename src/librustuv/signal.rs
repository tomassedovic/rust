// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::libc::c_int;
use std::rt::io::signal::Signum;
use std::rt::sched::{SchedHandle, Scheduler};
use std::comm::{SharedChan, SendDeferred};
use std::rt::local::Local;
use std::rt::rtio::RtioSignal;

use super::{Loop, UvError, UvHandle};
use uvll;
use uvio::HomingIO;

pub struct SignalWatcher {
    handle: *uvll::uv_signal_t,
    home: SchedHandle,

    channel: SharedChan<Signum>,
    signal: Signum,
}

impl SignalWatcher {
    pub fn new(loop_: &mut Loop, signum: Signum,
               channel: SharedChan<Signum>) -> Result<~SignalWatcher, UvError> {
        let handle = UvHandle::alloc(None::<SignalWatcher>, uvll::UV_SIGNAL);
        assert_eq!(unsafe {
            uvll::uv_signal_init(loop_.handle, handle)

        }, 0);

        match unsafe { uvll::uv_signal_start(handle, signal_cb, signum as c_int) } {
            0 => {
                let s = ~SignalWatcher {
                    handle: handle,
                    home: get_handle_to_current_scheduler!(),
                    channel: channel,
                    signal: signum,
                };
                Ok(s.install())
            }
            n => {
                unsafe { uvll::free_handle(handle) }
                Err(UvError(n))
            }
        }

    }
}

extern fn signal_cb(handle: *uvll::uv_signal_t, signum: c_int) {
    let s: &mut SignalWatcher = unsafe { UvHandle::from_uv_handle(&handle) };
    assert_eq!(signum as int, s.signal as int);
    s.channel.send_deferred(s.signal);
}

impl HomingIO for SignalWatcher {
    fn home<'r>(&'r mut self) -> &'r mut SchedHandle { &mut self.home }
}

impl UvHandle<uvll::uv_signal_t> for SignalWatcher {
    fn uv_handle(&self) -> *uvll::uv_signal_t { self.handle }
}

impl RtioSignal for SignalWatcher {}

impl Drop for SignalWatcher {
    fn drop(&mut self) {
        let _m = self.fire_homing_missile();
        self.close_async_();
    }
}
