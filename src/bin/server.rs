use std::io;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

const MAX_MESSAGES: usize = 100;

#[derive(Debug, Clone)]
struct Message {
    username: String,
    message: String,
}

impl Message {
    pub const fn new(username: String, message: String) -> Self {
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
    Error(io::Error),
}

async fn read_message(mut connection: &mut TcpStream) -> io::Result<MessageResult> {
    let receiver = BufReader::new(&mut connection);
    let Some(message) = receiver.lines().next_line().await? else {
        return Ok(MessageResult::NothingReceived);
    };
    println!("Received message: {message}");
    let mut sections = message.split(": ");
    let Some(username) = sections.next() else {
        connection.write_all(b"Received an empty message!").await?;
        return Ok(MessageResult::NoUsername);
    };
    let message = sections.collect::<Vec<&str>>().join(": ");
    if message.is_empty() {
        Ok(MessageResult::NoMessage(username.to_owned()))
    } else {
        Ok(MessageResult::Message(Message::new(
            username.to_owned(),
            message,
        )))
    }
}

async fn send_messages(
    connection: &mut TcpStream,
    messages: &[Message],
    username: &str,
) -> io::Result<()> {
    let response = messages
        .iter()
        .map(|message| {
            if message.username() == username {
                format!("you: {}", message.message())
            } else {
                message.to_string()
            }
        })
        .collect::<Vec<String>>()
        .join("\n");
    println!("Created response");

    connection.write_all(response.as_bytes()).await
}

#[tokio::main]
async fn main() {
    let mut messages = Vec::new();
    let mut tasks: Vec<JoinHandle<MessageResult>> = Vec::new();
    let listener = TcpListener::bind("127.0.0.1:2000").await.unwrap();
    loop {
        let Ok((mut connection, _)) = listener.accept().await else {
            continue;
        };

        let mut i = 0;
        while i < tasks.len() {
            if !tasks[i].is_finished() {
                i += 1;
                continue;
            }
            let task = tasks.remove(i);
            match task.await.unwrap() {
                MessageResult::Error(error) => {
                    match error.kind() {
                        io::ErrorKind::BrokenPipe => eprintln!("A pipe closed unexpectedly"),
                        io::ErrorKind::InvalidData => eprintln!("Received invalid data"),
                        io::ErrorKind::TimedOut => eprintln!("Request timed out"),
                        io::ErrorKind::Interrupted => eprintln!("Receiving data was interrupted"),
                        io::ErrorKind::Unsupported => {
                            eprintln!("Receiving data over internet is not supported");
                        }
                        io::ErrorKind::OutOfMemory => eprintln!("Request used too much memory"),
                        io::ErrorKind::Other => eprintln!("Unexpected error occured"),
                        error => eprintln!("Unhandled error occured: {error}"),
                    }
                    continue;
                }
                MessageResult::Message(message) => messages.push(message),
                _ => (),
            };
        }

        while messages.len() > MAX_MESSAGES {
            messages.remove(0);
        }

        let messages_to_send = messages.clone();
        tasks.push(tokio::spawn(async move {
            let (username, message) = match read_message(&mut connection).await {
                Ok(message_state) => match message_state {
                    MessageResult::NoUsername => return MessageResult::NoUsername,
                    MessageResult::NothingReceived => return MessageResult::NothingReceived,
                    MessageResult::Message(message) => {
                        println!("Parsed message: {message:?}");
                        let username = message.username().to_owned();
                        (username, Some(message))
                    }
                    MessageResult::NoMessage(username) => (username, None),
                    MessageResult::Error(error) => return MessageResult::Error(error),
                },
                Err(error) => return MessageResult::Error(error),
            };
            if let Err(error) = send_messages(&mut connection, &messages_to_send, &username).await {
                match error.kind() {
                    io::ErrorKind::BrokenPipe => eprintln!("A pipe closed unexpectedly"),
                    io::ErrorKind::TimedOut => eprintln!("Request timed out"),
                    io::ErrorKind::Interrupted => eprintln!("Sending data was interrupted"),
                    io::ErrorKind::Unsupported => {
                        eprintln!("Sending data over internet is not supported");
                    }
                    io::ErrorKind::OutOfMemory => eprintln!("Request used too much memory"),
                    io::ErrorKind::Other => eprintln!("Unexpected error occured"),
                    error => eprintln!("Unhandled error occured: {error}"),
                }
            };
            if let Some(message) = message {
                MessageResult::Message(message)
            } else {
                MessageResult::NoMessage(username)
            }
        }));
    }
}
