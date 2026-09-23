use {
  crate::{environment::ENVIRONMENT, gemini, robots, url::from_path},
  log::{error, info, warn},
  tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
  },
};

const MAXIMUM_REQUEST_LINE_BYTES: usize = 1024;
const MAXIMUM_PROXY_DURATION: std::time::Duration =
  std::time::Duration::from_secs(90);

pub async fn serve() {
  let address = format!("0.0.0.0:{}", ENVIRONMENT.http09_port);
  let listener = match TcpListener::bind(&address).await {
    Ok(listener) => {
      info!("HTTP/0.9 server listening on {address}");

      listener
    }
    Err(error) => {
      error!("failed to bind HTTP/0.9 server to {address}: {error}");

      return;
    }
  };

  loop {
    let (stream, peer) = match listener.accept().await {
      Ok(connection) => connection,
      Err(error) => {
        warn!("HTTP/0.9 accept error: {error}");

        continue;
      }
    };

    tokio::spawn(async move {
      match tokio::time::timeout(MAXIMUM_PROXY_DURATION, handle(stream)).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => warn!("HTTP/0.9 error from {peer}: {error}"),
        Err(_) => warn!("HTTP/0.9 request from {peer} timed out"),
      }
    });
  }
}

async fn handle(
  stream: tokio::net::TcpStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  let (reader, mut writer) = stream.into_split();
  let mut reader =
    BufReader::new(reader).take(MAXIMUM_REQUEST_LINE_BYTES as u64 + 1);
  let mut request_line = String::new();

  tokio::time::timeout(
    std::time::Duration::from_secs(10),
    reader.read_line(&mut request_line),
  )
  .await??;

  if request_line.len() > MAXIMUM_REQUEST_LINE_BYTES
    || !request_line.ends_with('\n')
  {
    return Err(
      std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "HTTP/0.9 request line exceeds the size limit or has no terminator",
      )
      .into(),
    );
  }

  let path = parse_request(&request_line)?;
  let mut configuration =
    crate::response::configuration::Configuration::default();
  let url = from_path(&path, &mut configuration)?;

  ensure_allowed(&url).await?;

  let mut response = gemini::request(&url).await?;

  if *response.status() == germ::request::Status::PermanentRedirect
    || *response.status() == germ::request::Status::TemporaryRedirect
  {
    let redirect = url.join(&response.meta())?;

    ensure_allowed(&redirect).await?;

    response = gemini::request(&redirect).await?;
  }

  if response.meta().starts_with("image/") {
    if let Some(bytes) = response.content_bytes() {
      writer.write_all(bytes).await?;
    }
  } else if let Some(content) = response.content() {
    writer.write_all(content.as_bytes()).await?;
  }

  writer.shutdown().await?;

  Ok(())
}

async fn ensure_allowed(url: &url::Url) -> std::io::Result<()> {
  match robots::check_access(url).await {
    robots::Access::Allowed => Ok(()),
    robots::Access::Denied => Err(std::io::Error::new(
      std::io::ErrorKind::PermissionDenied,
      "The destination capsule prohibits access through web proxies.",
    )),
    robots::Access::Unavailable => Err(std::io::Error::other(
      "The destination capsule's robots.txt could not be checked.",
    )),
  }
}

fn parse_request(
  line: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
  let line = line.trim();

  line.strip_prefix("GET ").map_or_else(
    || {
      if line.starts_with('/') {
        Ok(line.to_string())
      } else {
        Err(format!("invalid HTTP/0.9 request: {line}").into())
      }
    },
    |path| Ok(path.split_whitespace().next().unwrap_or("/").to_string()),
  )
}
