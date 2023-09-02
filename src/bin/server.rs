use std::{env::args, io};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};

/// The maximum number of messages to be stored
const MAX_MESSAGES: usize = 100;

/// Stores the message and the user who send it
#[derive(Debug, Clone)]
struct Message {
    username: String,
    message: String,
}

impl Message {
    /// Create a new message
    pub const fn new(username: String, message: String) -> Self {
        Self { username, message }
    }

    /// Return the username of the user who send it
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Returns the message content
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Write the message to the formatter
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

/// Reads and parses the message
async fn read_message(mut connection: &mut TcpStream) -> MessageResult {
    // Create a buffer for reading the message
    let receiver = BufReader::new(&mut connection);

    // The message can only be one line currently, so just read that line.
    // Return NothingReceived or the io error on failure
    let message = match receiver.lines().next_line().await {
        Ok(Some(message)) => message,
        Ok(None) => return MessageResult::NothingReceived,
        Err(error) => return MessageResult::Error(error),
    };

    // Split the message to receive the username
    let mut sections = message.split(": ");

    // Check whether the message contains a username.
    // It is unlikely not to return Some, so even an empty username could be used
    let Some(username) = sections.next() else {
        return if let Err(error) = connection.write_all(b"Received an empty message!").await {
            MessageResult::Error(error)
        } else {
            MessageResult::NoUsername
        };
    };

    // Everything after ": " is part of the message
    let message = sections.collect::<Vec<&str>>().join(": ");

    // If the message is empty, it was an update request so only return the username.
    // Otherwise, return both the message and the username
    if message.is_empty() {
        MessageResult::NoMessage(username.to_owned())
    } else {
        MessageResult::Message(Message::new(username.to_owned(), message))
    }
}

/// Sends messages to the user
async fn send_messages(
    connection: &mut TcpStream,
    messages: &[Message],
    username: &str,
) -> io::Result<()> {
    // Create a string containing all messages.
    // Replace the username with "you" for messages send by this user.
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

    // Send the messages
    connection.write_all(response.as_bytes()).await
}

async fn receive_messages(tasks: &mut Vec<JoinHandle<MessageResult>>, messages: &mut Vec<Message>) {
    let mut i = 0;
    while i < tasks.len() {
        if !tasks[i].is_finished() {
            i += 1;
            continue;
        }
        let task = tasks.remove(i);
        match task.await.unwrap() {
            MessageResult::Error(error) => match error.kind() {
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
            },
            MessageResult::Message(message) => messages.push(message),
            _ => (),
        };
    }
}

#[tokio::main]
async fn main() {
    // Create arrays for messages and tasks
    let mut messages = Vec::new();
    let mut tasks: Vec<JoinHandle<MessageResult>> = Vec::new();

    //Check whether the user passed an address, use the local address with port 2000 if not
    let address = if let Some(address) = args().nth(1) {
        address
    } else if let Ok(address) = local_ip_address::local_ip() {
        format!("{address}:2000")
    } else if let Ok(address) = local_ip_address::local_ipv6() {
        format!("{address}:2000")
    } else {
        "127.0.0.1:2000".to_owned()
    };

    // Create a listener for connections
    let listener = TcpListener::bind(&address).await.unwrap();

    println!("Listening on: {address}");

    loop {
        // Wait for a connection, continue to the next iteration if not
        let Ok((mut connection, _)) = listener.accept().await else {
            continue;
        };

        // Finish tasks started in a previous iteration if possible, adding messages if available
        receive_messages(&mut tasks, &mut messages).await;

        // Remove messages while there are more than MAX_MESSAGES messages
        while messages.len() > MAX_MESSAGES {
            messages.remove(0);
        }

        // Clone the messages to be send to prevent it from being moved
        let messages_to_send = messages.clone();

        // Spawn a new task to receive messages
        tasks.push(tokio::spawn(async move {
            // Receive the message
            let (username, message) = match read_message(&mut connection).await {
                MessageResult::NoUsername => return MessageResult::NoUsername,
                MessageResult::NothingReceived => return MessageResult::NothingReceived,
                MessageResult::Message(message) => {
                    println!("Parsed message: {message:?}");
                    let username = message.username().to_owned();
                    (username, Some(message))
                }
                MessageResult::NoMessage(username) => (username, None),
                MessageResult::Error(error) => return MessageResult::Error(error),
            };

            // Send the message, return the error on failure.
            // Return the message, if available.
            // Return the username otherwise
            if let Err(error) = send_messages(&mut connection, &messages_to_send, &username).await {
                MessageResult::Error(error)
            } else if let Some(message) = message {
                MessageResult::Message(message)
            } else {
                MessageResult::NoMessage(username)
            }
        }));
    }
}
