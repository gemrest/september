use {
  crate::{environment::ENVIRONMENT, url::from_path},
  tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
  },
};

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
      if let Err(error) = handle(stream).await {
        warn!("HTTP/0.9 error from {peer}: {error}");
      }
    });
  }
}

async fn handle(
  stream: tokio::net::TcpStream,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  let (reader, mut writer) = stream.into_split();
  let mut reader = BufReader::new(reader);
  let mut request_line = String::new();

  reader.read_line(&mut request_line).await?;

  let path = parse_request(&request_line)?;
  let mut configuration = crate::response::configuration::Configuration::new();
  let url = from_path(&path, false, &mut configuration)?;
  let mut response = germ::request::request(&url).await?;

  if *response.status() == germ::request::Status::PermanentRedirect
    || *response.status() == germ::request::Status::TemporaryRedirect
  {
    let redirect = if response.meta().starts_with('/') {
      format!(
        "gemini://{}{}",
        url.domain().unwrap_or_default(),
        response.meta()
      )
    } else {
      response.meta().to_string()
    };

    response = germ::request::request(&url::Url::parse(&redirect)?).await?;
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
