use slint::platform::{duration_until_next_timer_update, update_timers_and_animations};
use std::error::Error;
use std::time::Instant;
use winit::event::StartCause;
use winit::event_loop::ControlFlow;
use winit::window::WindowId;

/// Implementation of [`crate::rt::Hook`] for [`SlintBackend`].
pub struct Hook;

impl crate::rt::Hook for Hook {
    fn new_events(&mut self, cause: &StartCause) -> Result<(), Box<dyn Error + Send + Sync>> {
        // The pre_window_event will run after StartCause::WaitCancelled to we don't need to do its
        // work here.
        if !matches!(
            cause,
            StartCause::WaitCancelled {
                start: _,
                requested_resume: _
            }
        ) {
            update_timers_and_animations();
        }

        Ok(())
    }

    fn pre_window_event(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        update_timers_and_animations();
        Ok(())
    }

    fn window_destroyed(&mut self, _: WindowId) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }

    fn post_window_event(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }

    fn about_to_wait(&mut self) -> Result<ControlFlow, Box<dyn Error + Send + Sync>> {
        let f = match duration_until_next_timer_update() {
            Some(t) if !t.is_zero() => ControlFlow::WaitUntil(Instant::now() + t),
            _ => ControlFlow::Wait,
        };

        Ok(f)
    }
}
