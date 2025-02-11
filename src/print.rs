use std::io::{stderr, stdout, Write};
use std::sync::{Arc, Mutex};
use termcolor::{ColorChoice, StandardStream, WriteColor as _};
use rustc_errors::{ColorSpec, WriteColor, Color as RustColor};
use crate::Color;

#[derive(Clone)]
pub struct Printer {
    inner: Arc<Mutex<PrinterInner>>,
}

impl Write for Printer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut inner = self.inner.lock().unwrap();
        let col = inner.current_color.clone();
        inner.messages.push(PrintMessage::RustcErrTerm(RustcErrTermMessage::new(buf.to_vec(), col)));
        Ok(buf.len())
    }

    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl WriteColor for Printer {
    #[inline]
    fn supports_color(&self) -> bool {
        self.inner.lock().unwrap().supports_color
    }

    #[inline]
    fn set_color(&mut self, spec: &ColorSpec) -> std::io::Result<()> {
        self.inner.lock().unwrap().current_color = Some(spec.clone());
        Ok(())
    }

    #[inline]
    fn reset(&mut self) -> std::io::Result<()> {
        self.inner.lock().unwrap().current_color.take();
        Ok(())
    }
}

struct PrinterInner {
    color_setting: Color,
    current_color: Option<ColorSpec>,
    messages: Vec<PrintMessage>,
    supports_color: bool,
}

impl Printer {

    pub fn new(term_output_color: Color) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PrinterInner {
                color_setting: term_output_color,
                current_color: None,
                messages: vec![],
                supports_color: true, // Todo: Actually check
            })),
        }
    }

    #[inline]
    pub fn push_msg(&self, msg: PrintMessage) {
        self.inner.lock().unwrap().messages.push(msg);
    }

    pub fn dump(&self) -> Result<(), std::io::Error> {
        let inner = self.inner.lock().unwrap();
        let mut use_term_stdout = term::stdout().filter(|t| inner.color_setting.use_colored_tty() && t.supports_color());
        let use_rustc_error_color = inner.color_setting.use_colored_tty() && term::stderr().map(|t| t.supports_color()).unwrap_or_default();
        let mut rustc_err_out = use_rustc_error_color.then_some(StandardStream::stderr(ColorChoice::Always));
        for msg in &inner.messages {
            match msg {
                PrintMessage::Stdout(out) => {
                    stdout().write_all(out)?;
                }
                PrintMessage::StdErr(err) => {
                    stderr().write_all(err)?;
                }
                PrintMessage::Term(t_msg) => {
                    if let Some(t) = &mut use_term_stdout {
                        if let Some(col) = t_msg.color {
                            t.fg(col).unwrap()
                        }
                        t.write_all(&t_msg.message)?;
                        if t_msg.color.is_some() {
                            t.reset().unwrap();
                        }
                    } else {
                        stdout().write_all(&t_msg.message)?;
                    }
                }
                PrintMessage::RustcErrTerm(msg) => {
                    if let Some(t) = &mut rustc_err_out {
                        if let Some(col) = msg.color.as_ref().map(rustc_colorspec_compat) {
                            t.set_color(&col)?;
                        }
                        t.write_all(&msg.message)?;
                        if msg.color.is_some() {
                            t.reset().unwrap();
                        }
                    } else {
                        stderr().write_all(&msg.message)?;
                    }
                }
            }
        }

        Ok(())
    }
}

// Rustc vendors termcolor, but not everything needed to use it,
// as far as I can tell
fn rustc_colorspec_compat(rustc: &ColorSpec) -> termcolor::ColorSpec {
    let mut cs = termcolor::ColorSpec::new();
    let col = rustc.fg().and_then(rustc_color_compat);
    cs.set_fg(col);
    cs
}

fn rustc_color_compat(rustc: &RustColor) -> Option<termcolor::Color> {
    let col = match rustc {
        RustColor::Black => termcolor::Color::Black,
        RustColor::Blue => termcolor::Color::Blue,
        RustColor::Green => termcolor::Color::Green,
        RustColor::Red => termcolor::Color::Red,
        RustColor::Cyan => termcolor::Color::Cyan,
        RustColor::Magenta => termcolor::Color::Magenta,
        RustColor::Yellow => termcolor::Color::Yellow,
        RustColor::White => termcolor::Color::White,
        RustColor::Ansi256(c) => termcolor::Color::Ansi256(*c),
        RustColor::Rgb(r, g, b) => termcolor::Color::Rgb(*r, *g, *b),
        _ => return None,
    };
    Some(col)
}

#[macro_export]
macro_rules! buf_println {
    ($pb: expr, $($arg:tt)*) => {{
        let mut msg_buf = Vec::new();
        let _ = writeln!(&mut msg_buf, $($arg)*);
        $pb.push_msg($crate::print::PrintMessage::Stdout(msg_buf));
    }};
}

#[macro_export]
macro_rules! buf_eprintln {
    ($pb: expr, $($arg:tt)*) => {{
        let mut msg_buf = Vec::new();
        let _ = writeln!(&mut msg_buf, $($arg)*);
        $pb.push_msg($crate::print::PrintMessage::StdErr(msg_buf));
    }};
}

#[macro_export]
macro_rules! buf_term_println {
    ($pb: expr, $col:expr, $($arg:tt)*) => {{
        let mut msg_buf = Vec::new();
        let _ = writeln!(&mut msg_buf, $($arg)*);
        $pb.push_msg($crate::print::PrintMessage::Term($crate::print::TermMessage::new(msg_buf, $col)));
    }};
}

pub enum PrintMessage {
    Stdout(Vec<u8>),
    StdErr(Vec<u8>),
    Term(TermMessage),
    RustcErrTerm(RustcErrTermMessage)
}

pub struct TermMessage {
    message: Vec<u8>,
    color: Option<term::color::Color>,
}

impl TermMessage {
    pub fn new(message: Vec<u8>, color: Option<term::color::Color>) -> Self {
        Self { message, color }
    }
}

pub struct RustcErrTermMessage {
    message: Vec<u8>,
    color: Option<ColorSpec>,
}

impl RustcErrTermMessage {
    pub fn new(message: Vec<u8>, color: Option<ColorSpec>) -> Self {
        Self { message, color }
    }
}
