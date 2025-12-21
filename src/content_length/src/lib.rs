use std::{
    io::{self, BufRead, Write},
    str::FromStr,
};

pub fn write(writer: &mut impl Write, content: &str) -> io::Result<()> {
    writer.write(b"Content-Length: ")?;
    writer.write(format!("{}", content.len()).as_bytes())?;
    writer.write(b"\r\n\r\n")?;
    writer.write_all(content.as_bytes())?;
    writer.flush()?;
    Ok(())
}

pub fn read(reader: &mut impl BufRead) -> io::Result<Option<String>> {
    let mut size = None;
    let mut buffer = String::with_capacity(1024);

    loop {
        buffer.clear();

        if reader.read_line(&mut buffer)? == 0 {
            return Ok(None);
        }

        let Some(buf) = buffer.strip_suffix("\r\n") else {
            return Err(malformed(format!("Malformed Header: {:?}", buffer)));
        };

        if buf.is_empty() {
            break;
        }

        let Some((header_name, header_value)) = buf.split_once(": ") else {
            return Err(malformed(format!("Malformed Header: {:?}", buf)));
        };

        if header_name.eq_ignore_ascii_case("Content-Length") {
            size = Some(usize::from_str(header_value).map_err(malformed)?);
        }
    }

    let size: usize = size.ok_or_else(|| malformed(format!("Missing Content-Length")))?;
    let mut buffer = buffer.into_bytes();
    buffer.resize(size, 0);
    reader.read_exact(&mut buffer)?;

    Ok(Some(String::from_utf8(buffer).map_err(malformed)?))
}

fn malformed(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}
