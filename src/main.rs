// Uncomment this block to pass the first stage
use std::collections::HashMap;

use anyhow::Context;
use itertools::Itertools;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

type Headers<'a> = HashMap<String, String>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    //
    let listener = tokio::net::TcpListener::bind("127.0.0.1:4221").await?;

    loop {
        let (stream, _) = listener.accept().await?;

        tokio::spawn(async move { handle(stream).await });
    }
}

async fn handle(mut stream: tokio::net::TcpStream) -> anyhow::Result<()> {
    println!("accepted new connection");

    let mut read_buffer = tokio::io::BufReader::new(&mut stream);
    let mut method: Vec<u8> = Vec::new();

    read_buffer
        .read_until(b' ', &mut method)
        .await
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
        .await
        .context("reading path")?;

    let mut http_version = Vec::new();

    read_buffer
        .read_until(b'\n', &mut http_version)
        .await
        .context("reading http version")?;

    let mut header = Vec::new();
    let mut headers: Headers = Headers::default();

    loop {
        read_buffer
            .read_until(b'\n', &mut header)
            .await
            .context("reading http version")?;

        if header == b"\r\n" {
            break;
        }

        if let Some((key, value)) = header.split(|&x| x == b':').collect_tuple() {
            headers.insert(
                String::from_utf8(key.to_vec())?,
                String::from_utf8(value[1..value.len() - 2].to_vec())?,
            );
        }

        header.clear();
    }

    match &path[..path.len() - 1] {
        b"/" => {
            stream
                .write(b"HTTP/1.1 200 OK\r\n\r\n")
                .await
                .with_context(|| format!("writing on {:?}", stream))?;
        }
        b"/user-agent" => {
            let Some(user_agent) = headers.get("User-Agent") else {
                anyhow::bail!("User-Agent not found")
            };

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                user_agent.len(),
                user_agent
            );

            stream
                .write(response.as_bytes())
                .await
                .with_context(|| format!("writing on {:?}", stream))?;
        }
        path if path.starts_with(b"/echo/") => {
            let content = &path[6..];

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                content.len(),
                std::str::from_utf8(content).context("parsing to UTF-8")?
            );

            stream
                .write(response.as_bytes())
                .await
                .with_context(|| format!("writing on {:?}", stream))?;
        }

        _ => {
            stream
                .write(b"HTTP/1.1 404 Not Found\r\n\r\n")
                .await
                .with_context(|| format!("writing on {:?}", stream))?;
        }
    };

    Ok(())
}
