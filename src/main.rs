// Uncomment this block to pass the first stage
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
};

use anyhow::Context;

fn main() -> anyhow::Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    //
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("accepted new connection");

                let mut read_buffer = BufReader::new(&stream);
                let mut method: Vec<u8> = Vec::new();

                read_buffer
                    .read_until(b' ', &mut method)
                    .context("reading method")?;

                let _ = method.pop();

                // match &method[..] {
                //     b"GET" => {
                //         println!("Get")
                //     }
                //     _ => panic!("oops"),
                // }

                let mut path = Vec::new();

                read_buffer
                    .read_until(b' ', &mut path)
                    .context("reading path")?;

                match &path[..path.len() - 1] {
                    b"/" => {
                        stream
                            .write(b"HTTP/1.1 200 OK\r\n\r\n")
                            .with_context(|| format!("writing on {:?}", stream))?;
                    }
                    path if path.starts_with(b"/echo/") => {
                        let content = &path[6..];

                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}\r\n",
                            content.len(),
                            std::str::from_utf8(content).context("parsing to UTF-8")?
                        );

                        stream
                            .write(response.as_bytes())
                            .with_context(|| format!("writing on {:?}", stream))?;
                    }
                    _ => {
                        stream
                            .write(b"HTTP/1.1 404 Not Found\r\n\r\n")
                            .with_context(|| format!("writing on {:?}", stream))?;
                    }
                };
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }

    Ok(())
}
