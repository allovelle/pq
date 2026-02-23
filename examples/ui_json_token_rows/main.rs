use std::io::{Read, Write};
use std::net::TcpListener;

const HTML: &str = include_str!("index.html");

fn main()
{
    // port 0 = OS picks one
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let url = format!("http://127.0.0.1:{port}");

    // Browser support
    println!("Opening {url}");
    open::that(&url).unwrap();

    // Serve exactly one request then exit
    let (mut stream, _) = listener.accept().unwrap();
    let mut buf = [0u8; 1024];
    let _ = stream.read(&mut buf).unwrap();

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
        HTML.len(),
        HTML
    );
    stream.write_all(response.as_bytes()).unwrap();
}
