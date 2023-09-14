use std::{
    env::args,
    io::{self, BufRead, Read, Write},
    net::TcpStream,
};

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

/// Get the server address from an argument passed when starting the program or from the screen
fn get_server_address<W: Write, R: BufRead>(output: &mut W, input: &mut R) -> io::Result<String> {
    // Take the server address from the second argument or from the screen
    args().nth(1).map_or_else(
        || read_input_line(output, input, "Enter the address of the server: "),
        Ok,
    )
}

/// Get the username from an argument passed when starting the program or from the screen
fn get_username<W: Write, R: BufRead>(output: &mut W, input: &mut R) -> io::Result<String> {
    args().nth(2).map_or_else(
        || read_input_line(output, input, "Enter your username: "),
        Ok,
    )
}

fn main() {
    // Take a reference to stdout and stdin
    let mut stdout = io::stdout();
    let stdin = io::stdin();

    // Read the configuration
    let server = get_server_address(&mut stdout, &mut stdin.lock())
        .expect("Failed to read the address of the server");
    let username =
        get_username(&mut stdout, &mut stdin.lock()).expect("Failed to read your username");
    let username = username.trim();

    // Create a new client
    let mut client = Client::new(username.to_owned(), server.trim().to_owned());

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
            match error.kind() {
                io::ErrorKind::ConnectionRefused => panic!("The server refused to connect!"),
                io::ErrorKind::ConnectionReset => panic!("The connection was reset by the server!"),
                io::ErrorKind::ConnectionAborted => panic!("The server aborted the connection!"),
                io::ErrorKind::NotConnected => panic!(
                    "The application tried to send the message before the connection was active!"
                ),
                io::ErrorKind::AddrNotAvailable => {
                    panic!("The requested address wasn't available!")
                }
                io::ErrorKind::BrokenPipe => panic!("The pipe broke!"),
                io::ErrorKind::InvalidInput => panic!("The server address is invalid!\n{error}"),
                io::ErrorKind::TimedOut => panic!("The connection took too long!"),
                io::ErrorKind::WriteZero => panic!("0 bytes were sent!"),
                io::ErrorKind::Interrupted => panic!("The connection was interrupted!"),
                io::ErrorKind::Unsupported => panic!("You don't have an internet connection!"),
                io::ErrorKind::OutOfMemory => panic!("Sending the message took too much memory!"),
                io::ErrorKind::Other => panic!("An unknown error occured!\n{error}"),
                error => panic!("An unhandled error occured!\n{error}"),
            }
        };

        // Receive messages from the server
        match client.receive_messages() {
            Err(error) => match error.kind() {
                io::ErrorKind::ConnectionRefused => panic!("The server refused to connect!"),
                io::ErrorKind::ConnectionReset => panic!("The connection was reset by the server!"),
                io::ErrorKind::ConnectionAborted => panic!("The server aborted the connection!"),
                io::ErrorKind::NotConnected => panic!(
                    "The application tried to send the message before the connection was active!"
                ),
                io::ErrorKind::BrokenPipe => panic!("The pipe broke!"),
                io::ErrorKind::InvalidData => panic!("The message wasn't valid utf-8!"),
                io::ErrorKind::TimedOut => panic!("The connection took too long!"),
                io::ErrorKind::Interrupted => panic!("The connection was interrupted!"),
                io::ErrorKind::OutOfMemory => panic!("The received messages took too much memory!"),
                io::ErrorKind::Other => panic!("An unknown error occured!\n{error}"),
                error => panic!("An unhandled error occured!\n{error}"),
            },
            Ok(messages) => println!("{messages}"),
        };

        // Close the connection
        let _ = client.close_connection();
    }
}
