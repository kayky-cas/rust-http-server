// Uncomment this block to pass the first stage
use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use clap::Parser;
use itertools::Itertools;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    #[clap(long)]
    directory: Option<PathBuf>,
}

type Headers<'a> = HashMap<String, String>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let config: Arc<_> = Args::parse().into();

    // Uncomment this block to pass the first stage
    //
    let listener = tokio::net::TcpListener::bind("127.0.0.1:4221").await?;

    loop {
        let config = config.clone();
        let (stream, _) = listener.accept().await?;

        tokio::spawn(async move { handle(stream, config).await });
    }
}

async fn handle(mut stream: tokio::net::TcpStream, config: Arc<Args>) -> anyhow::Result<()> {
    println!("accepted new connection");

    let mut read_buffer = tokio::io::BufReader::new(&mut stream);
    let mut method: Vec<u8> = Vec::new();

    read_buffer
        .read_until(b' ', &mut method)
        .await
        .context("reading method")?;

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

    let content_len = headers
        .get("Content-Length")
        .unwrap_or(&"0".to_string())
        .parse::<usize>()?;

    let content = if content_len > 0 {
        let mut content = vec![0; content_len];
        read_buffer
            .read_exact(&mut content)
            .await
            .context("reading content")?;
        content
    } else {
        Vec::new()
    };

    match (&method[..method.len() - 1], &path[..path.len() - 1]) {
        (b"GET", b"/") => {
            stream
                .write(b"HTTP/1.1 200 OK\r\n\r\n")
                .await
                .with_context(|| format!("writing on {:?}", stream))?;
        }
        (b"GET", b"/user-agent") => {
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
        (b"GET", path) if path.starts_with(b"/echo/") => {
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
        (b"GET", path) if path.starts_with(b"/files/") => {
            let file_name = std::str::from_utf8(&path[7..]).context("parsing to UTF-8")?;

            let file_path = config
                .directory
                .as_ref()
                .map(|d| d.join(file_name))
                .unwrap();

            let Ok(attr) = tokio::fs::metadata(&file_path).await else {
                stream
                    .write(b"HTTP/1.1 404 Not Found\r\n\r\n")
                    .await
                    .with_context(|| format!("writing on {:?}", stream))?;
                return Ok(());
            };

            let mut file = tokio::fs::File::open(file_path)
                .await
                .with_context(|| format!("opening file {:?}", file_name))?;

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n",
                attr.len()
            );

            stream
                .write(response.as_bytes())
                .await
                .with_context(|| format!("writing on {:?}", stream))?;

            tokio::io::copy(&mut file, &mut stream).await?;
        }
        (b"POST", path) if path.starts_with(b"/files/") => {
            let file_name = std::str::from_utf8(&path[7..]).context("parsing to UTF-8")?;

            let file_path = config
                .directory
                .as_ref()
                .map(|d| d.join(file_name))
                .unwrap();

            let mut file = tokio::fs::File::create(file_path)
                .await
                .with_context(|| format!("creating file {:?}", file_name))?;

            file.write_all(&content)
                .await
                .with_context(|| format!("writing on {:?}", file))?;

            stream
                .write(b"HTTP/1.1 201 Created\r\nContent-Type: text/plain\r\n\r\n")
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
