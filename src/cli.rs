//! Local command console.
//!
//! The relay already executes commands received from the master node (`CommandRegistry`).
//! This exposes the same registry on the process stdin, so an operator can drive the relay
//! from the terminal that launched it:
//!
//! - interactive stdin (a TTY): a `> ` prompt with command history (Up/Down);
//! - non-interactive stdin (piped, redirected or launched by a service manager): the lines
//!   are still read and executed, without a prompt, so `echo status | noxrelay` works.

use std::io::{self, IsTerminal, Write};
use std::sync::{Arc, Mutex, OnceLock};

use rustyline::ExternalPrinter;
use tokio::sync::Notify;
use tracing::{debug, warn};
use tracing_subscriber::fmt::MakeWriter;

use crate::handlers::context::AppState;

/// Prompt displayed before every interactive command.
const PROMPT: &str = "> ";

/// Printer of the running interactive editor, if any.
///
/// Logging through it (see [`LogMakeWriter`]) makes rustyline redraw the prompt around the
/// log line instead of leaving a log written over the `> ` indicator.
static PRINTER: OnceLock<Mutex<Option<Box<dyn ExternalPrinter + Send>>>> = OnceLock::new();

fn printer() -> &'static Mutex<Option<Box<dyn ExternalPrinter + Send>>> {
    PRINTER.get_or_init(|| Mutex::new(None))
}

/// Writes one complete log line: through the interactive editor when one is running, directly
/// to stdout otherwise.
fn print_line(line: &str) {
    let mut guard = match printer().lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };

    match guard.as_mut() {
        Some(printer) => {
            let _ = printer.print(line.to_string());
        }
        None => {
            let mut out = io::stdout().lock();
            let _ = out.write_all(line.as_bytes());
            let _ = out.flush();
        }
    }
}

fn install_printer(value: Box<dyn ExternalPrinter + Send>) {
    if let Ok(mut guard) = printer().lock() {
        *guard = Some(value);
    }
}

fn clear_printer() {
    if let Ok(mut guard) = printer().lock() {
        *guard = None;
    }
}

/// `tracing` writer that sends each formatted event to [`print_line`].
#[derive(Default)]
pub struct LogWriter {
    line: Vec<u8>,
}

impl Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for &byte in buf {
            if byte == b'\n' {
                let text = String::from_utf8_lossy(&self.line);
                print_line(&format!("{text}\n"));
                self.line.clear();
            } else {
                self.line.push(byte);
            }
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if !self.line.is_empty() {
            let text = String::from_utf8_lossy(&self.line);
            print_line(&format!("{text}\n"));
            self.line.clear();
        }
        Ok(())
    }
}

/// `MakeWriter` used by the tracing subscriber (`cli::LogMakeWriter`).
#[derive(Clone, Copy, Default)]
pub struct LogMakeWriter;

impl<'a> MakeWriter<'a> for LogMakeWriter {
    type Writer = LogWriter;

    fn make_writer(&'a self) -> Self::Writer {
        LogWriter::default()
    }
}

/// Starts the stdin command loop on a dedicated OS thread.
///
/// It is deliberately **not** a Tokio task: `readline()` blocks on stdin, and the Tokio
/// runtime waits for its blocking tasks when it shuts down, which kept the process alive after
/// a graceful `stop`/`exit`. A detached thread is terminated with the process.
///
/// `shutdown` is notified when Ctrl+C is pressed at the interactive prompt: rustyline
/// consumes the signal to clear the line, so the console has to request the same graceful
/// shutdown as the process signal handlers.
pub fn spawn(state: Arc<AppState>, shutdown: Arc<Notify>) {
    let interactive = io::stdin().is_terminal();

    let spawned = std::thread::Builder::new()
        .name("relay-cli".to_string())
        .spawn(move || {
            if interactive {
                run_interactive(state, shutdown);
            } else {
                blocking_lines(&state);
            }
        });

    if let Err(e) = spawned {
        warn!("[CLI] failed to start the command console thread: {e}");
    }
}

/// Executes one command line through the shared registry.
fn execute(state: &AppState, line: &str) {
    let line = line.trim();
    if line.is_empty() {
        return;
    }

    match state.commands.lock() {
        Ok(guard) => match guard.as_ref() {
            Some(registry) => registry.execute(line),
            None => warn!("[CLI] command registry is not ready"),
        },
        Err(_) => warn!("[CLI] command registry lock is poisoned"),
    }
}

/// Interactive stdin: prompt, line editing and history (rustyline).
fn run_interactive(state: Arc<AppState>, shutdown: Arc<Notify>) {
    use rustyline::{error::ReadlineError, DefaultEditor};

    let mut editor = match DefaultEditor::new() {
        Ok(editor) => editor,
        Err(e) => {
            warn!("[CLI] failed to start the interactive console: {e}");
            blocking_lines(&state);
            return;
        }
    };

    // Log lines are printed through rustyline so they do not erase the `> ` prompt.
    match editor.create_external_printer() {
        Ok(value) => install_printer(Box::new(value)),
        Err(e) => warn!("[CLI] failed to install the log printer: {e}"),
    }

    debug!("[CLI] interactive command console ready (type `help`)");

    loop {
        match editor.readline(PROMPT) {
            Ok(line) => {
                if line.trim().is_empty() {
                    continue;
                }

                // Keep the command in the Up/Down history.
                let _ = editor.add_history_entry(line.as_str());
                execute(&state, &line);
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl+C at the prompt: clear the line and request a graceful shutdown.
                let _ = io::stdout().write_all(b"\n");
                let _ = io::stdout().flush();
                shutdown.notify_one();
                break;
            }
            Err(ReadlineError::Eof) => break,
            Err(e) => {
                warn!("[CLI] stdin read error: {e}");
                break;
            }
        }
    }

    // Back to direct stdout writes for the shutdown logs.
    clear_printer();
}

/// Non-interactive stdin: read every line, without a prompt or history.
fn blocking_lines(state: &AppState) {
    use std::io::BufRead;

    for line in io::stdin().lock().lines() {
        match line {
            Ok(line) => execute(state, &line),
            Err(e) => {
                debug!("[CLI] stdin closed: {e}");
                return;
            }
        }
    }
}
