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

#[macro_use]
extern crate log;

use std::{env::var, time::Instant};

use actix_web::{web, Error, HttpResponse};
use germ::ast::Node;
use gmi::{protocol::Response, url::Url};

fn link_from_host_href(url: &Url, href: &str) -> String {
  format!(
    "gemini://{}{}{}",
    url.authority.host,
    {
      if href.starts_with('/') {
        ""
      } else {
        "/"
      }
    },
    href
  )
}

fn gemini_to_html(
  response: &Response,
  url: &Url,
  is_proxy: bool,
) -> (String, String) {
  let ast = germ::ast::build(&String::from_utf8_lossy(&response.data));
  let mut html = String::new();
  let mut title = "".to_string();

  for node in ast {
    match node {
      Node::Text(text) => html.push_str(&format!("<p>{}</p>", text)),
      Node::Link {
        to,
        text,
      } => {
        let mut href = to.clone();

        if href.starts_with('/') || !href.contains("://") {
          href = link_from_host_href(url, &href);
        }

        if var("PROXY_BY_DEFAULT").unwrap_or_else(|_| "true".to_string())
          == "true"
          && to.contains("gemini://")
        {
          if is_proxy
            || href
              .trim_start_matches("gemini://")
              .trim_end_matches('/')
              .split('/')
              .collect::<Vec<_>>()
              .get(0)
              .unwrap()
              != &url.authority.host.as_str()
          {
            href = format!("/proxy/{}", href.trim_start_matches("gemini://"));
          } else {
            href = href
              .trim_start_matches("gemini://")
              .replace(&url.authority.host, "");
          }
        }

        if let Ok(keeps) = var("KEEP_GEMINI_EXACT") {
          let mut keeps = keeps.split(',');

          if href.starts_with('/') || !href.contains("://") {
            let temporary_href = link_from_host_href(url, &href);

            if keeps.any(|k| k == &*temporary_href) {
              href = temporary_href;
            }
          }
        }

        if let Ok(keeps) = var("KEEP_GEMINI_DOMAIN") {
          if href.starts_with('/')
            || !href.contains("://")
              && keeps.split(',').any(|k| k == &*url.authority.host)
          {
            href = link_from_host_href(url, &href);
          }
        }

        html.push_str(&format!(
          "<p><a href=\"{}\">{}</a></p>\n",
          href,
          text.unwrap_or(to)
        ));
      }
      Node::Heading {
        level,
        text,
      } => {
        if title.is_empty() && level == 1 {
          title = text.clone();
        }

        html.push_str(&format!(
          "<{}>{}</{0}>",
          match level {
            1 => "h1",
            2 => "h2",
            3 => "h3",
            _ => "p",
          },
          text
        ));
      }
      Node::List(items) =>
        html.push_str(&format!(
          "<ul>{}</ul>",
          items
            .into_iter()
            .map(|i| format!("<li>{}</li>", i))
            .collect::<Vec<String>>()
            .join("\n")
        )),
      Node::Blockquote(text) =>
        html.push_str(&format!("<blockquote>{}</blockquote>", text)),
      Node::PreformattedText {
        text, ..
      } => {
        html.push_str(&format!("<pre>{}</pre>", text));
      }
      Node::Whitespace => {}
    }
  }

  (title, html)
}

fn make_url(
  path: &str,
  fallback: bool,
  is_proxy: &mut bool,
) -> Result<Url, gmi::url::UrlParseError> {
  Ok(
    match Url::try_from(&*if path.starts_with("/proxy") {
      *is_proxy = true;

      format!(
        "gemini://{}{}",
        path.replace("/proxy/", ""),
        if fallback { "/" } else { "" }
      )
    } else if path.starts_with("/x") {
      *is_proxy = true;

      format!(
        "gemini://{}{}",
        path.replace("/x/", ""),
        if fallback { "/" } else { "" }
      )
    } else {
      // Try to set `ROOT` as `ROOT` environment variable, or use
      // `"gemini://fuwn.me"` as default.
      format!(
        "{}{}{}",
        {
          if let Ok(root) = var("ROOT") {
            root
          } else {
            warn!(
              "could not use ROOT from environment variables, proceeding with \
               default root: gemini://fuwn.me"
            );

            "gemini://fuwn.me".to_string()
          }
        },
        path,
        if fallback { "/" } else { "" }
      )
    }) {
      Ok(url) => url,
      Err(e) => return Err(e),
    },
  )
}

#[allow(clippy::unused_async, clippy::future_not_send, clippy::too_many_lines)]
async fn default(req: actix_web::HttpRequest) -> Result<HttpResponse, Error> {
  let mut is_proxy = false;
  // Try to construct a Gemini URL
  let url = make_url(
    &format!("{}{}", req.path(), {
      if !req.query_string().is_empty() || req.uri().to_string().ends_with('?')
      {
        format!("?{}", req.query_string())
      } else {
        "".to_string()
      }
    }),
    false,
    &mut is_proxy,
  )
  .unwrap();
  // Make a request to get Gemini content and time it.
  let mut timer = Instant::now();
  let mut response = match gmi::request::make_request(&url) {
    Ok(response) => response,
    Err(e) => {
      return Ok(HttpResponse::Ok().body(e.to_string()));
    }
  };
  if response.data.is_empty() {
    response = match gmi::request::make_request(
      &make_url(req.path(), true, &mut is_proxy).unwrap(),
    ) {
      Ok(response) => response,
      Err(e) => {
        return Ok(HttpResponse::Ok().body(e.to_string()));
      }
    };
  }
  let response_time_taken = timer.elapsed();

  // Reset timer for below
  timer = Instant::now();

  // Convert Gemini Response to HTML and time it.
  let mut html_context = String::from("<!DOCTYPE html><html><head>");
  let gemini_html = gemini_to_html(&response, &url, is_proxy);
  let gemini_title = gemini_html.0;
  let convert_time_taken = timer.elapsed();

  // Try to add an external stylesheet from the `CSS_EXTERNAL` environment
  // variable.
  if let Ok(css) = var("CSS_EXTERNAL") {
    html_context.push_str(&format!(
      "<link rel=\"stylesheet\" type=\"text/css\" href=\"{}\">",
      css
    ));
  }

  // Try to add an external favicon from the `FAVICON_EXTERNAL` environment
  // variable.
  if let Ok(favicon) = var("FAVICON_EXTERNAL") {
    html_context.push_str(&format!(
      "<link rel=\"icon\" type=\"image/x-icon\" href=\"{}\">",
      favicon
    ));
  }

  // Add a title to HTML response
  html_context.push_str(&format!("<title>{}</title>", gemini_title));

  html_context.push_str("</head><body>");

  html_context.push_str(&gemini_html.1);

  // Add proxy information to footer of HTML response
  html_context.push_str(&format!(
    "<details>\n<summary>Proxy information</summary>
<dl>
<dt>Original URL</dt>
<dd><a href=\"{}\">{0}</a></dd>
<dt>Status code</dt>
<dd>{:?}</dd>
<dt>Meta</dt>
<dd>{}</dd>
<dt>Capsule response time</dt>
<dd>{} milliseconds</dd>
<dt>Gemini-to-HTML time</dt>
<dd>{} milliseconds</dd>
</dl>
<p>This content has been proxied by \
<a href=\"https://github.com/gemrest/september{}\">September ({})</a>.</p>
</details></body></html>",
    url,
    response.status,
    response.meta,
    response_time_taken.as_nanos() as f64 / 1_000_000.0,
    convert_time_taken.as_nanos() as f64 / 1_000_000.0,
    format_args!("/tree/{}", env!("VERGEN_GIT_SHA")),
    env!("VERGEN_GIT_SHA").get(0..5).unwrap_or("UNKNOWN"),
  ));

  // Return HTML response
  Ok(
    HttpResponse::Ok()
      .content_type("text/html; charset=utf-8")
      .body(html_context),
  )
}

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
