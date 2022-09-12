use std::{collections::HashSet};

use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use url::Url;

macro_rules! scan {
  ( $string:expr, $sep:expr, $( $x:ty ),+ ) => {{
      let mut iter = $string.split($sep);
      ($(iter.next().and_then(|word| word.parse::<$x>().ok()),)*)
  }}
}

fn parse_handshake(buf: Vec<u8>) -> (String, String, String) {
    let line = String::from_utf8(buf).unwrap();
    let (method, remote, protocol) = scan!(line, char::is_whitespace, String, String, String);

    (method.unwrap(), remote.unwrap(), protocol.unwrap())
}

async fn handle_connection(mut socket: TcpStream, allow_set: HashSet<&str>) {
    let mut buf = vec![0; 1024];

    let handshake_n = socket.read(&mut buf).await.unwrap();

    let (method, url, _) = parse_handshake(buf.clone());

    let parsed_url = Url::parse(url.as_str()).unwrap();
    
    let host = parsed_url.host().unwrap();
    let port = parsed_url.port_or_known_default().unwrap();

    if !allow_set.contains(host.to_string().as_str()) {
        socket.write("HTTP/1.1 403 Forbidden\r\n\r\n".as_bytes()).await;
    } else {
        let mut remote = TcpStream::connect(std::format!("{}:{}", host, port)).await.unwrap();

        remote.write(&buf[..handshake_n]).await;
    
        let (mut ri, mut wi) = socket.split();
        let (mut ro, mut wo) = remote.split();
    
        let client_to_server = async {
            io::copy(&mut ri, &mut wo).await?;
            wo.shutdown().await
        };
    
        let server_to_client = async {
            io::copy(&mut ro, &mut wi).await?;
            wi.shutdown().await
        };
    
        tokio::try_join!(client_to_server, server_to_client);
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let allow_set: HashSet<&str> = vec!(
      "baidu.com",
    )
    .into_iter()
    .collect();

    let listener = TcpListener::bind("0.0.0.0:3000").await?;

    loop {
        let (socket, _) = listener.accept().await?;

        // no clone?
        let handler = handle_connection(socket, allow_set.clone());
        tokio::spawn(handler);
    }
}
