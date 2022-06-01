// This file is part of September <https://github.com/gemrest/september>.
// Copyright (C) 2022-2022 Fuwn <contact@fuwn.me>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.
//
// This program is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.
//
// Copyright (C) 2022-2022 Fuwn <contact@fuwn.me>
// SPDX-License-Identifier: GPL-3.0-only

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

mod gemini_to_html;
mod response;
mod url;

#[macro_use]
extern crate log;

use std::env::var;

use actix_web::web;
use response::default;

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
    if let Ok(port) = var("PORT") {
      match port.parse::<_>() {
        Ok(port) => port,
        Err(e) => {
          warn!("could not use PORT from environment variables: {}", e);
          warn!("proceeding with default port: 80");

          80
        }
      }
    } else {
      80
    },
  ))?
  .run()
  .await
}
