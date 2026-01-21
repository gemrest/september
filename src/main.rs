#![deny(
  warnings,
  nonstandard_style,
  unused,
  future_incompatible,
  rust_2018_idioms,
  unsafe_code
)]
#![deny(clippy::all, clippy::nursery, clippy::pedantic)]
#![recursion_limit = "128"]
#![allow(clippy::cast_precision_loss)]

mod environment;
mod html;
mod response;
mod url;

#[macro_use] extern crate log;

use {actix_web::web, response::default, std::env::var};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  dotenv::dotenv().ok();
  pretty_env_logger::formatted_builder()
    .parse_filters(
      &std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "actix_web=info".to_string()),
    )
    .init();

  actix_web::HttpServer::new(move || {
    actix_web::App::new()
      .default_service(web::get().to(default))
      .wrap(actix_web::middleware::Logger::default())
  })
  .bind((
    "0.0.0.0",
    var("PORT").map_or(80, |port| match port.parse::<_>() {
      Ok(port) => port,
      Err(e) => {
        warn!("could not use PORT from environment variables: {e}");
        warn!("proceeding with default port: 80");

        80
      }
    }),
  ))?
  .run()
  .await
}
