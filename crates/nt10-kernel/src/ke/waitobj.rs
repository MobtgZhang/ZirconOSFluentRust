//! Waitable objects — dispatcher / `KeWait*` bring-up.
//!
//! Message-queue cooperative waits use [`crate::ke::msg_wait::MsgWaitGen`] (generation counter +
//! [`crate::ke::sched::block_cooperative_idle`] inside
//! [`crate::ke::msg_wait::MsgWaitGen::wait_until_changed`]); see
//! [`crate::subsystems::win32::msg_dispatch::get_message_wait_kernel`] and
//! [`crate::subsystems::win32::user32::wait_pop_message_zr`].
