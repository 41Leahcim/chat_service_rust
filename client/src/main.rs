use std::{
    io::{self, BufRead, Read, Write},
    net::TcpStream,
};

use clap::Parser;

/// Controlls the connection with the server
struct Client {
    username: String,
    server: String,
    connection: Option<TcpStream>,
}

impl Client {
    /// Creates a new client
    pub const fn new(username: String, server: String) -> Self {
        Self {
            username,
            server,
            connection: None,
        }
    }

    /// Open a connection
    pub fn open_connection(&mut self) -> io::Result<()> {
        self.connection = Some(TcpStream::connect(&self.server)?);
        Ok(())
    }

    /// Closes the current connection
    pub fn close_connection(&mut self) -> io::Result<()> {
        if let Some(connection) = self.connection.as_mut() {
            connection.flush()?;
        }
        self.connection = None;
        Ok(())
    }

    /// Sends the passed message over the connection.
    /// Creates a new connection if necessary.
    pub fn send_message(&mut self, message: &str) -> io::Result<()> {
        // Create a new connection if needed
        if self.connection.is_none() {
            self.open_connection()?;
        }

        // Send the message
        let connection = self.connection.as_mut().unwrap();
        writeln!(connection, "{}: {message}", self.username)?;
        Ok(())
    }

    /// Receives and returns messages.
    /// Creates a new connection if needed
    pub fn receive_messages(&mut self) -> io::Result<String> {
        // Open a new connection if needed
        if self.connection.is_none() {
            self.open_connection()?;
        }

        // Create a String for the messages
        let mut received = String::new();

        // Take a mutable reference to the connection
        let connection = self.connection.as_mut().unwrap();

        // Send any messages that are still waiting to be sent
        connection.flush()?;

        // Receive the messages
        connection.read_to_string(&mut received)?;
        Ok(received)
    }
}

/// Reads a line of input from the screen
fn read_input_line<W: Write, R: BufRead>(
    output: &mut W,
    input: &mut R,
    request: &str,
) -> io::Result<String> {
    // Print the request
    output.write_all(request.as_bytes())?;

    // Flush the writer
    output.flush()?;

    // Read the line of input
    let mut buffer = String::new();
    input.read_line(&mut buffer)?;
    Ok(buffer)
}

/// Handles most if not all errors you could get with this application
fn handle_io_error(error: io::ErrorKind) {
    match error {
        io::ErrorKind::ConnectionRefused => panic!("The server refused to connect!"),
        io::ErrorKind::ConnectionReset => panic!("The connection was reset by the server!"),
        io::ErrorKind::ConnectionAborted => panic!("The server aborted the connection!"),
        io::ErrorKind::NotConnected => {
            panic!("The application tried to send the message before the connection was active!")
        }
        io::ErrorKind::AddrNotAvailable => {
            panic!("The requested address wasn't available!")
        }
        io::ErrorKind::BrokenPipe => panic!("The pipe broke!"),
        io::ErrorKind::InvalidInput => panic!("The server address is invalid!\n{error}"),
        io::ErrorKind::TimedOut => panic!("The connection took too long!"),
        io::ErrorKind::WriteZero => panic!("0 bytes were sent!"),
        io::ErrorKind::Interrupted => panic!("The connection was interrupted!"),
        io::ErrorKind::Unsupported => panic!("You don't have an internet connection!"),
        io::ErrorKind::OutOfMemory => panic!("Out of memory memory!"),
        io::ErrorKind::Other => panic!("An unknown error occured!\n{error}"),
        io::ErrorKind::InvalidData => panic!("The message wasn't valid utf-8!"),
        error => panic!("An unhandled error occured!\n{error}"),
    }
}

#[derive(Debug, Parser)]
struct Args {
    /// Server address
    #[arg(short, long)]
    server: Option<String>,

    /// Your username
    #[arg(short, long)]
    username: Option<String>,
}

fn init() -> (io::Stdin, io::Stdout, Client) {
    // Take a reference to stdout and stdin
    let mut stdout = io::stdout();
    let stdin = io::stdin();

    // Parse the arguments
    let args = Args::parse();

    // Read the configuration
    let server = args.server.unwrap_or(
        read_input_line(
            &mut stdout,
            &mut stdin.lock(),
            "Enter the address of the server: ",
        )
        .unwrap(),
    );
    let username = args.username.unwrap_or(
        read_input_line(&mut stdout, &mut stdin.lock(), "Enter your username: ").unwrap(),
    );

    // Create a new client
    (stdin, stdout, Client::new(username, server))
}

fn main() {
    // Initialize the client
    let (stdin, mut stdout, mut client) = init();
    loop {
        // Read the message from the screen
        let message = match read_input_line(
            &mut stdout,
            &mut stdin.lock(),
            "Enter a message to send or just press enter to update: ",
        ) {
            Ok(message) => message,
            Err(error) => {
                eprintln!("Failed to read message: {error}");
                continue;
            }
        };
        let message = message.trim();

        // Send the message
        if let Err(error) = client.send_message(message) {
            handle_io_error(error.kind())
        };

        // Receive messages from the server
        match client.receive_messages() {
            Err(error) => handle_io_error(error.kind()),
            Ok(messages) => println!("{messages}"),
        };

        // Close the connection
        let _ = client.close_connection();
    }
}
