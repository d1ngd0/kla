use std::{fs::File, io, str};

pub enum OutputType {
    File(Box<File>),
    StdOut,
}

impl io::Write for OutputType {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            OutputType::StdOut => {
                print!(
                    "{}",
                    str::from_utf8(buf)
                        .unwrap_or("Binary data, unsafe to write to write to standard out")
                );
                Ok(buf.len())
            }
            OutputType::File(file) => {
                file.as_mut().write_all(buf)?;
                Ok(buf.len())
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            OutputType::StdOut => Ok(()),
            OutputType::File(file) => file.as_mut().flush(),
        }
    }
}
