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

mod html;
mod response;
mod url;

#[macro_use] extern crate log;

use {actix_web::web, response::default, std::env::var};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  // Set the `RUST_LOG` environment variable so Actix can provide logs.
  //
  // This can be overridden using the `RUST_LOG` environment variable in
  // configuration.
  std::env::set_var("RUST_LOG", "actix_web=info");

  // Initialise `dotenv` so we can access `.env` files.
  dotenv::dotenv().ok();
  // Initialise logger so we can see logs
  pretty_env_logger::init();

  // Setup Actix web-server
  actix_web::HttpServer::new(move || {
    actix_web::App::new()
      .default_service(web::get().to(default))
      .wrap(actix_web::middleware::Logger::default())
  })
  .bind((
    // Bind Actix web-server to localhost
    "0.0.0.0",
    // If the `PORT` environment variable is present, try to use it, otherwise;
    // use port `80`.
    var("PORT").map_or(80, |port| match port.parse::<_>() {
      Ok(port) => port,
      Err(e) => {
        warn!("could not use PORT from environment variables: {}", e);
        warn!("proceeding with default port: 80");

        80
      }
    }),
  ))?
  .run()
  .await
}
