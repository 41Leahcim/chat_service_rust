use std::{
    io::{self, BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
};

const MAX_MESSAGES: usize = 100;

#[derive(Debug, Clone)]
struct Message {
    username: String,
    message: String,
}

impl Message {
    pub fn new(username: String, message: String) -> Self {
        Self { username, message }
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}: {}", self.username, self.message))
    }
}

enum MessageResult {
    NothingReceived,
    NoUsername,
    NoMessage(String),
    Message(Message),
}

fn read_message(mut connection: &mut TcpStream) -> io::Result<MessageResult> {
    let receiver = BufReader::new(&mut connection);
    let Some(message) = receiver.lines().next() else {
        return Ok(MessageResult::NothingReceived);
    };
    let message = message?;
    println!("Received message: {message}");
    let mut sections = message.split(": ");
    let Some(username) = sections.next() else {
        connection.write_all("Received an empty message!".as_bytes())?;
        return Ok(MessageResult::NoUsername);
    };
    let message = sections
        .map(str::to_owned)
        .collect::<Vec<String>>()
        .join(": ");
    if message.is_empty() {
        Ok(MessageResult::NoMessage(username.to_owned()))
    } else {
        Ok(MessageResult::Message(Message::new(
            username.to_owned(),
            message,
        )))
    }
}

fn send_messages(
    connection: &mut TcpStream,
    messages: &[Message],
    username: &str,
) -> io::Result<()> {
    let response = messages
        .iter()
        .map(|message| {
            if message.username() == username {
                Message::new("you".to_owned(), message.message().to_owned())
            } else {
                message.to_owned()
            }
            .to_string()
        })
        .collect::<Vec<String>>()
        .join("\n");
    println!("Created response");

    connection.write_all(response.as_bytes())
}

fn main() {
    let mut messages = Vec::new();
    let listener = TcpListener::bind("127.0.0.1:2000").unwrap();
    for connection in listener.incoming() {
        println!("Received connection");
        let Ok(mut connection) = connection else {
            continue;
        };

        let username = match read_message(&mut connection) {
            Ok(message_state) => match message_state {
                MessageResult::NoUsername | MessageResult::NothingReceived => continue,
                MessageResult::Message(message) => {
                    println!("Parsed message: {message:?}");
                    let username = message.username().to_owned();
                    messages.push(message);
                    username
                }
                MessageResult::NoMessage(username) => username,
            },
            Err(error) => {
                match error.kind() {
                    io::ErrorKind::BrokenPipe => eprintln!("A pipe closed unexpectedly"),
                    io::ErrorKind::InvalidData => eprintln!("Received invalid data"),
                    io::ErrorKind::TimedOut => eprintln!("Request timed out"),
                    io::ErrorKind::Interrupted => eprintln!("Receiving data was interrupted"),
                    io::ErrorKind::Unsupported => {
                        eprintln!("Receiving data over internet is not supported")
                    }
                    io::ErrorKind::OutOfMemory => eprintln!("Request used too much memory"),
                    io::ErrorKind::Other => eprintln!("Unexpected error occured"),
                    error => eprintln!("Unhandled error occured: {error}"),
                }
                continue;
            }
        };

        if messages.len() > MAX_MESSAGES {
            messages.remove(0);
        }

        if let Err(error) = send_messages(&mut connection, &messages, &username) {
            match error.kind() {
                io::ErrorKind::BrokenPipe => eprintln!("A pipe closed unexpectedly"),
                io::ErrorKind::TimedOut => eprintln!("Request timed out"),
                io::ErrorKind::Interrupted => eprintln!("Sending data was interrupted"),
                io::ErrorKind::Unsupported => {
                    eprintln!("Sending data over internet is not supported")
                }
                io::ErrorKind::OutOfMemory => eprintln!("Request used too much memory"),
                io::ErrorKind::Other => eprintln!("Unexpected error occured"),
                error => eprintln!("Unhandled error occured: {error}"),
            }
        };
    }
}
