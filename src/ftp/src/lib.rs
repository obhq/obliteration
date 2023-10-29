use self::retrieve::Retrieve;
use std::fmt::Display;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use thiserror::Error;

pub mod retrieve;

/// The reason we need to implement our own FTP client is because no other FTP client supports the
/// custom command.
///
/// This implementation follows https://www.rfc-editor.org/rfc/rfc959.
pub struct FtpClient {
    con: BufReader<TcpStream>,
}

impl FtpClient {
    pub fn new(con: TcpStream) -> Result<Self, NewError> {
        // Enable no-delay.
        if let Err(e) = con.set_nodelay(true) {
            return Err(NewError::EnableNoDelayFailed(e));
        }

        // Construct the client.
        let mut client = Self {
            con: BufReader::new(con),
        };

        // Wait until the server is ready.
        loop {
            let reply = client.read_reply()?;

            if reply.is_positive_preliminary() {
                continue;
            } else if reply.is_positive_completion() {
                break;
            } else {
                return Err(NewError::UnexpectedGreeting(reply));
            }
        }

        Ok(client)
    }

    pub fn list(&mut self, path: &str) -> Result<Vec<FtpItem>, ListError> {
        // Open data connection.
        let data_addr = self.passive()?;
        let mut data = match TcpStream::connect(&data_addr) {
            Ok(v) => BufReader::new(v),
            Err(e) => {
                return Err(ListError::OpenDataConnectionFailed(data_addr, e));
            }
        };

        // Send LIST command.
        if let Err(e) = self.exec("LIST", path) {
            return Err(ListError::SendCommandFailed(e));
        }

        // Read LIST reply.
        match self.read_reply() {
            Ok(v) => {
                if !v.is_positive_preliminary() {
                    return Err(ListError::UnexpectedReply(v));
                }
            }
            Err(e) => return Err(ListError::ReadReplyFailed(e)),
        }

        // Read items.
        let mut items: Vec<FtpItem> = Vec::new();
        let mut line = String::new();

        loop {
            // Read data.
            line.clear();

            match data.read_line(&mut line) {
                Ok(v) => {
                    if v == 0 {
                        break;
                    }
                }
                Err(e) => return Err(ListError::ReadDataFailed(e)),
            }

            // Parse data.
            let line = line.trim_end();
            let item = FtpItem::new(line).ok_or(ListError::InvalidData(line.into()))?;

            // Skip if item represent current and parent directory.
            if item.name == "." || item.name == ".." {
                continue;
            }

            items.push(item);
        }

        drop(data);

        // Read completed reply.
        match self.read_reply() {
            Ok(v) => {
                if !v.is_positive_completion() {
                    return Err(ListError::UnexpectedReply(v));
                }
            }
            Err(e) => return Err(ListError::ReadReplyFailed(e)),
        }

        Ok(items)
    }

    pub fn retrieve(&mut self, path: &str) -> Result<Retrieve, RetrieveError> {
        // Open data connection.
        let data_addr = self.passive()?;
        let data = match TcpStream::connect(&data_addr) {
            Ok(v) => v,
            Err(e) => {
                return Err(RetrieveError::OpenDataConnectionFailed(data_addr, e));
            }
        };

        // Send RETR command.
        if let Err(e) = self.exec("RETR", path) {
            return Err(RetrieveError::SendCommandFailed(e));
        }

        // Read RETR reply.
        match self.read_reply() {
            Ok(v) => {
                if !v.is_positive_preliminary() {
                    return Err(RetrieveError::UnexpectedReply(v));
                }
            }
            Err(e) => return Err(RetrieveError::ReadReplyFailed(e)),
        }

        Ok(Retrieve::new(self, data))
    }

    pub fn exec(&mut self, cmd: &str, arg: &str) -> Result<(), ExecError> {
        // Setup the command.
        let mut cmd = String::from(cmd);

        if !arg.is_empty() {
            cmd.push(' ');
            cmd.push_str(arg);
        }

        cmd.push_str("\r\n");

        // Send the request.
        if let Err(e) = self.con.get_mut().write_all(cmd.as_bytes()) {
            return Err(ExecError::SendCommandFailed(e));
        }

        Ok(())
    }

    pub fn read_reply(&mut self) -> Result<Reply, ReadReplyError> {
        // Read the first line.
        let mut line = String::new();

        match self.con.read_line(&mut line) {
            Ok(v) => {
                if v == 0 {
                    return Err(ReadReplyError::ConnectionClosed);
                }
            }
            Err(e) => return Err(ReadReplyError::ReadFailed(e)),
        }

        // Check roughly format.
        if line.len() < 7 || !line.ends_with("\r\n") {
            return Err(ReadReplyError::InvalidData(line));
        }

        // Extract code.
        let code: [u8; 3] = line[..3].as_bytes().try_into().unwrap();

        if !code[0].is_ascii_digit()
            || code[0] == b'0'
            || code[0] > b'5'
            || !code[1].is_ascii_digit()
            || code[1] > b'5'
        {
            return Err(ReadReplyError::InvalidData(line));
        }

        // Extract text.
        let sep = line.as_bytes()[3];
        let text = if sep == b'-' {
            // Read a multi-line reply.
            let mut text = String::from(line[4..].trim_start());
            let mut end = String::from(&line[..3]);

            end.push(' ');

            loop {
                // Read the next line.
                line.clear();

                match self.con.read_line(&mut line) {
                    Ok(v) => {
                        if v == 0 {
                            return Err(ReadReplyError::ConnectionClosed);
                        }
                    }
                    Err(e) => return Err(ReadReplyError::ReadFailed(e)),
                }

                // Check if line valid.
                if !line.ends_with("\r\n") {
                    return Err(ReadReplyError::InvalidData(line));
                }

                // Check if the last line.
                if line.starts_with(&end) {
                    text.push_str(line[4..].trim());
                    break;
                }

                // Concat line.
                text.push_str(line.trim_start());
            }

            text
        } else if sep == b' ' {
            line[4..].trim().into()
        } else {
            return Err(ReadReplyError::InvalidData(line));
        };

        Ok(Reply { code, text })
    }

    fn passive(&mut self) -> Result<String, PassiveError> {
        // Send the command.
        if let Err(e) = self.exec("PASV", "") {
            return Err(PassiveError::ExecFailed(e));
        }

        // Parse the reply.
        let addr = match self.read_reply() {
            Ok(v) => {
                // Check if 2xx reply.
                if !v.is_positive_completion() {
                    return Err(PassiveError::UnexpectedReply(v));
                }

                // Extract host and port.
                let t = v.text();
                let i = match t.chars().position(|c| c.is_ascii_digit()) {
                    Some(v) => v,
                    None => return Err(PassiveError::UnexpectedReply(v)),
                };

                match Self::parse_port(&t[i..]) {
                    Some(v) => v,
                    None => return Err(PassiveError::UnexpectedReply(v)),
                }
            }
            Err(e) => return Err(PassiveError::ReadReplyFailed(e)),
        };

        Ok(addr)
    }

    fn parse_port(v: &str) -> Option<String> {
        use std::fmt::Write;

        // Parse the address.
        let mut r = String::with_capacity(21);
        let mut i = 0;
        let mut p = 0u16;

        for v in v.split(',') {
            if i <= 3 {
                // IP.
                match v.parse::<u8>() {
                    Ok(_) => {
                        if i == 0 {
                            r.push_str(v);
                        } else {
                            r.push('.');
                            r.push_str(v);
                        }
                    }
                    Err(_) => return None,
                }
            } else if i == 4 {
                // Port 1.
                match v.parse::<u8>() {
                    Ok(v) => p = v as u16,
                    Err(_) => return None,
                }
            } else if i == 5 {
                // Port 2.
                let i = v
                    .chars()
                    .position(|c| !c.is_ascii_digit())
                    .unwrap_or(v.len());

                match v[..i].parse::<u8>() {
                    Ok(v) => p = (p << 8) | v as u16,
                    Err(_) => return None,
                }
            } else {
                return None;
            }

            i += 1;
        }

        if i < 6 {
            return None;
        }

        // Append port number.
        write!(r, ":{p}").unwrap();

        Some(r)
    }
}

/// Represents an item on the server.
pub struct FtpItem {
    ty: ItemType,
    name: String,
    len: u64,
}

impl FtpItem {
    fn new(data: &str) -> Option<Self> {
        // Parse line.
        let mut mode: Option<&str> = None;
        let mut len: Option<&str> = None;
        let mut name: Option<&str> = None;
        let mut next = data;

        for i in 0..9 {
            let remain = next.trim_start();
            let end = remain
                .chars()
                .position(|c| c.is_whitespace())
                .unwrap_or(remain.len());
            let value = &remain[..end];

            match i {
                0 => mode = Some(value),
                1 => {} // What is this?
                2 => {} // Owner.
                3 => {} // Group.
                4 => len = Some(value),
                5 => {} // Month.
                6 => {} // Day.
                7 => {} // Year.
                8 => name = Some(value),
                _ => unreachable!(),
            }

            next = &remain[end..];
        }

        let mode = mode?;
        let len = len?;
        let name = name?;

        // Check type.
        let ty = match mode.chars().next().unwrap() {
            '-' => ItemType::RegularFile,
            'd' => ItemType::Directory,
            _ => return None,
        };

        // Parse length.
        let len = match len.parse() {
            Ok(v) => v,
            Err(_) => return None,
        };

        Some(Self {
            ty,
            name: name.into(),
            len,
        })
    }

    pub fn ty(&self) -> ItemType {
        self.ty
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// Type of [`FtpItem`].
#[derive(Clone, Copy)]
pub enum ItemType {
    RegularFile,
    Directory,
}

/// Represents a reply from server.
#[derive(Debug)]
pub struct Reply {
    code: [u8; 3],
    text: String,
}

impl Reply {
    pub fn is_positive_preliminary(&self) -> bool {
        self.code[0] == b'1'
    }

    pub fn is_positive_completion(&self) -> bool {
        self.code[0] == b'2'
    }

    pub fn text(&self) -> &str {
        self.text.as_ref()
    }
}

impl Display for Reply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}",
            std::str::from_utf8(&self.code).unwrap(),
            self.text
        )
    }
}

/// Represents an error for [`FtpClient::passive()`].
enum PassiveError {
    ExecFailed(ExecError),
    ReadReplyFailed(ReadReplyError),
    UnexpectedReply(Reply),
}

/// Represents an error for [`FtpClient::new()`].
#[derive(Debug, Error)]
pub enum NewError {
    #[error("cannot enable TCP no-delay")]
    EnableNoDelayFailed(#[from] std::io::Error),

    #[error("cannot read greeting reply")]
    ReadGreetingFailed(#[from] ReadReplyError),

    #[error("the server reply with an unexpected greeting ({0})")]
    UnexpectedGreeting(Reply),
}

/// Represents an error for [`FtpClient::list()`].
#[derive(Debug, Error)]
pub enum ListError {
    #[error("cannot enable passive mode")]
    EnablePassiveFailed(#[source] ExecError),

    #[error("cannot read the reply of PASV")]
    ReadPassiveFailed(#[from] ReadReplyError),

    #[error("unexpected reply for PASV ({0})")]
    UnexpectedPassiveReply(Reply),

    #[error("cannot open a data connection to {0}")]
    OpenDataConnectionFailed(String, #[source] std::io::Error),

    #[error("cannot send 'LIST' command")]
    SendCommandFailed(#[source] ExecError),

    #[error("cannot read the reply of 'LIST' command")]
    ReadReplyFailed(#[source] ReadReplyError),

    #[error("unexpected reply for 'LIST' command ({0})")]
    UnexpectedReply(Reply),

    #[error("cannot read data")]
    ReadDataFailed(#[source] std::io::Error),

    #[error("invalid data ({0})")]
    InvalidData(String),
}

impl From<PassiveError> for ListError {
    fn from(value: PassiveError) -> Self {
        match value {
            PassiveError::ExecFailed(e) => Self::EnablePassiveFailed(e),
            PassiveError::ReadReplyFailed(e) => Self::ReadPassiveFailed(e),
            PassiveError::UnexpectedReply(r) => Self::UnexpectedPassiveReply(r),
        }
    }
}

/// Represents an error for [`FtpClient::retrieve()`].
#[derive(Debug, Error)]
pub enum RetrieveError {
    #[error("cannot enable passive mode")]
    EnablePassiveFailed(#[source] ExecError),

    #[error("cannot read the reply of PASV")]
    ReadPassiveFailed(#[source] ReadReplyError),

    #[error("unexpected reply for PASV ({0})")]
    UnexpectedPassiveReply(Reply),

    #[error("cannot open a data connection to {0}")]
    OpenDataConnectionFailed(String, #[source] std::io::Error),

    #[error("cannot send 'RETR' command")]
    SendCommandFailed(#[source] ExecError),

    #[error("cannot read the reply of 'RETR' command")]
    ReadReplyFailed(#[source] ReadReplyError),

    #[error("unexpected reply for 'RETR' command ({0})")]
    UnexpectedReply(Reply),
}

impl From<PassiveError> for RetrieveError {
    fn from(value: PassiveError) -> Self {
        match value {
            PassiveError::ExecFailed(e) => Self::EnablePassiveFailed(e),
            PassiveError::ReadReplyFailed(e) => Self::ReadPassiveFailed(e),
            PassiveError::UnexpectedReply(r) => Self::UnexpectedPassiveReply(r),
        }
    }
}

/// Represents an error for [`FtpClient::exec()`].
#[derive(Debug, Error)]
pub enum ExecError {
    #[error("cannot send the command")]
    SendCommandFailed(#[source] std::io::Error),
}

/// Represents an error for [`FtpClient::read_reply()`].
#[derive(Debug, Error)]
pub enum ReadReplyError {
    #[error("the connection was closed")]
    ConnectionClosed,

    #[error("cannot read the data")]
    ReadFailed(#[source] std::io::Error),

    #[error("invalid data ({0})")]
    InvalidData(String),
}
