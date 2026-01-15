use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// A simple spinner that blinks ● while waiting for response
pub struct Spinner {
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    visible: Arc<AtomicBool>,
}

impl Spinner {
    /// Start a new spinner that blinks ● every 500ms
    pub fn start() -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let visible = Arc::new(AtomicBool::new(false));

        let running_clone = running.clone();
        let visible_clone = visible.clone();

        let handle = thread::spawn(move || {
            let mut stdout = io::stdout();

            while running_clone.load(Ordering::Relaxed) {
                // Show ●
                print!("●");
                stdout.flush().ok();
                visible_clone.store(true, Ordering::Relaxed);

                // Wait 500ms or until stopped
                for _ in 0..50 {
                    if !running_clone.load(Ordering::Relaxed) {
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                }

                if !running_clone.load(Ordering::Relaxed) {
                    break;
                }

                // Hide ● (backspace, space, backspace)
                print!("\x08 \x08");
                stdout.flush().ok();
                visible_clone.store(false, Ordering::Relaxed);

                // Wait 500ms or until stopped
                for _ in 0..50 {
                    if !running_clone.load(Ordering::Relaxed) {
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            }
        });

        Self {
            running,
            handle: Some(handle),
            visible,
        }
    }

    /// Stop the spinner and clean up
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);

        if let Some(handle) = self.handle.take() {
            handle.join().ok();
        }

        // If ● was visible, remove it
        if self.visible.load(Ordering::Relaxed) {
            print!("\x08 \x08");
            io::stdout().flush().ok();
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        if self.running.load(Ordering::Relaxed) {
            self.stop();
        }
    }
}

/// Streaming indicator that shows ● at the end of text while streaming
pub struct StreamingIndicator {
    has_indicator: bool,
}

impl StreamingIndicator {
    pub fn new() -> Self {
        Self {
            has_indicator: false,
        }
    }

    /// Print chunk and add ● indicator at the end
    pub fn print_chunk(&mut self, chunk: &str) {
        // Remove previous indicator if present
        if self.has_indicator {
            print!("\x08 \x08");
        }

        // Print the actual content
        print!("{}", chunk);

        // Add indicator
        print!("●");
        io::stdout().flush().ok();
        self.has_indicator = true;
    }

    /// Remove the indicator and finalize
    pub fn finish(&mut self) {
        if self.has_indicator {
            print!("\x08 \x08");
            io::stdout().flush().ok();
            self.has_indicator = false;
        }
    }
}

impl Default for StreamingIndicator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_start_stop() {
        let mut spinner = Spinner::start();
        std::thread::sleep(std::time::Duration::from_millis(100));
        spinner.stop();
        // Should not panic and should clean up properly
    }

    #[test]
    fn test_spinner_drop_stops_automatically() {
        let spinner = Spinner::start();
        std::thread::sleep(std::time::Duration::from_millis(50));
        drop(spinner);
        // Should not panic, drop should call stop
    }

    #[test]
    fn test_streaming_indicator_lifecycle() {
        let mut indicator = StreamingIndicator::new();
        indicator.print_chunk("Hello ");
        indicator.print_chunk("World");
        indicator.finish();
        // Should not panic
    }

    #[test]
    fn test_streaming_indicator_finish_without_chunks() {
        let mut indicator = StreamingIndicator::new();
        indicator.finish();
        // Should not panic even without any chunks
    }

    #[test]
    fn test_streaming_indicator_default() {
        let indicator = StreamingIndicator::default();
        assert!(!indicator.has_indicator);
    }
}
