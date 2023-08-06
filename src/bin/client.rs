use std::{
    env::args,
    io::{self, BufRead, Read, Write},
    net::TcpStream,
};

struct Client {
    username: String,
    server: String,
    connection: Option<TcpStream>,
}

impl Client {
    pub fn new(username: String, server: String) -> Self {
        Self {
            username,
            server,
            connection: None,
        }
    }

    pub fn open_connection(&mut self) -> io::Result<()> {
        self.connection = Some(TcpStream::connect(&self.server)?);
        Ok(())
    }

    pub fn close_connection(&mut self) {
        self.connection = None
    }

    pub fn send_message(&mut self, message: &str) -> io::Result<()> {
        if self.connection.is_none() {
            self.open_connection()?;
        }
        self.connection
            .as_mut()
            .unwrap()
            .write_fmt(format_args!("{}: {message}\n", self.username))?;
        Ok(())
    }

    pub fn receive_messages(&mut self) -> io::Result<String> {
        if self.connection.is_none() {
            self.open_connection()?;
        }
        let mut received = String::new();
        let connection = self.connection.as_mut().unwrap();
        connection.flush()?;
        connection.read_to_string(&mut received)?;
        Ok(received)
    }
}

fn read_input_line<W: Write, R: BufRead>(
    output: &mut W,
    input: &mut R,
    request: &str,
) -> io::Result<String> {
    output.write_all(request.as_bytes())?;
    output.flush()?;
    let mut buffer = String::new();
    input.read_line(&mut buffer)?;
    Ok(buffer)
}

fn get_server_address<W: Write, R: BufRead>(output: &mut W, input: &mut R) -> io::Result<String> {
    match args().nth(1) {
        Some(server) => Ok(server),
        None => read_input_line(output, input, "Enter the address of the server: "),
    }
}

fn get_username<W: Write, R: BufRead>(output: &mut W, input: &mut R) -> io::Result<String> {
    match args().nth(2) {
        Some(username) => Ok(username),
        None => read_input_line(output, input, "Enter your username: "),
    }
}

fn main() {
    let mut stdout = io::stdout();
    let stdin = io::stdin();
    let server = get_server_address(&mut stdout, &mut stdin.lock())
        .expect("Failed to read the address of the server");
    let username =
        get_username(&mut stdout, &mut stdin.lock()).expect("Failed to read your username");
    let username = username.trim();
    let mut client = Client::new(username.to_owned(), server.trim().to_owned());

    loop {
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
        client.close_connection();
    }
}
