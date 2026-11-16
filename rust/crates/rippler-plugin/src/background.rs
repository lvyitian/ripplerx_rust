//! The work the editor hands to the plugin's background thread.
//!
//! Decoding a user sample takes a while, and the editor must not freeze
//! while it happens. nice-plug's `AsyncExecutor` runs a task on a
//! background thread and can bring the answer back to the GUI thread, and
//! this module is the thin layer that uses it.
//!
//! The tasks themselves carry no data: the path of the sample that is to be
//! decoded and the sample that was decoded travel through the slots below,
//! which the editor and the plugin share.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use nice_plug::context::gui::AsyncExecutor;

use crate::RipplerX;
use crate::params::UserSample;

/// The background tasks of the plugin.
///
/// The plugin's task executor is the only thing that runs these, and it
/// runs them on a background thread, except for the task that is brought
/// back to the GUI thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundTask {
    /// Decode the sample whose path is waiting.
    DecodeSample,
    /// A sample was decoded and is waiting for the editor.
    ///
    /// This one is scheduled on the GUI thread, so that the editor notices
    /// the sample and picks it up.
    SampleDecoded,
}

/// The background work of the plugin.
///
/// The editor and the plugin share one of these, so that the editor can
/// ask for work and the plugin can do it.
#[derive(Clone, Default)]
pub(crate) struct Background {
    /// The path of the sample that is to be decoded.
    pending: Arc<Mutex<Option<PathBuf>>>,
    /// The sample that was decoded, waiting for the editor.
    decoded: Arc<Mutex<Option<UserSample>>>,
    /// The executor that runs the work.
    ///
    /// The plugin is given its executor when the editor is opened, so the
    /// slot is empty until then.
    executor: Arc<Mutex<Option<AsyncExecutor<RipplerX>>>>,
}

impl Background {
    /// Asks for a sample to be decoded.
    ///
    /// The sample is decoded on a background thread, and the editor picks
    /// it up with [`Background::take_decoded`] the next time it draws
    /// itself. Without an executor — which only happens before the editor
    /// has been opened — the sample is decoded on the calling thread.
    pub fn request_decode(&self, path: PathBuf) {
        if let Ok(mut slot) = self.pending.lock() {
            *slot = Some(path);
        }

        let Some(executor) = self.executor() else {
            return;
        };
        executor.execute_background(BackgroundTask::DecodeSample);
    }

    /// The sample that was decoded, if there is one.
    ///
    /// The sample is taken out of the slot, so that it is only used once.
    pub fn take_decoded(&self) -> Option<UserSample> {
        self.decoded.lock().ok().and_then(|mut slot| slot.take())
    }

    /// The path of the sample that is to be decoded.
    pub(crate) fn take_pending(&self) -> Option<PathBuf> {
        self.pending.lock().ok().and_then(|mut slot| slot.take())
    }

    /// The sample that was decoded.
    pub(crate) fn put_decoded(&self, sample: UserSample) {
        if let Ok(mut slot) = self.decoded.lock() {
            *slot = Some(sample);
        }
    }

    /// The executor that runs the work.
    pub(crate) fn set_executor(&self, executor: AsyncExecutor<RipplerX>) {
        if let Ok(mut slot) = self.executor.lock() {
            *slot = Some(executor);
        }
    }

    /// Tells the editor that a sample was decoded.
    ///
    /// The task is scheduled on the GUI thread, so that the editor draws
    /// itself again and picks the sample up.
    pub(crate) fn notify_decoded(&self) {
        let Some(executor) = self.executor() else {
            return;
        };
        executor.execute_gui(BackgroundTask::SampleDecoded);
    }

    /// The executor, if the editor has been opened.
    fn executor(&self) -> Option<AsyncExecutor<RipplerX>> {
        self.executor.lock().ok().and_then(|slot| slot.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sample_that_was_decoded_is_taken_out_only_once() {
        let background = Background::default();
        let sample = UserSample {
            waveform: vec![0.0, 1.0],
            sample_rate: 44100.0,
        };

        assert!(background.take_decoded().is_none());
        background.put_decoded(sample);
        assert!(background.take_decoded().is_some());
        assert!(background.take_decoded().is_none());
    }

    #[test]
    fn the_path_of_the_sample_that_is_to_be_decoded_is_taken_out_only_once() {
        let background = Background::default();
        background
            .pending
            .lock()
            .expect("the slot is not poisoned")
            .replace(PathBuf::from("sample.wav"));

        assert_eq!(background.take_pending(), Some(PathBuf::from("sample.wav")));
        assert_eq!(background.take_pending(), None);
    }
}
