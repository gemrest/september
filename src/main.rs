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

#[allow(clippy::too_many_lines)]
fn gemini_to_html(
  response: &Response,
  url: &Url,
  is_proxy: bool,
) -> (String, String) {
  let mut response_string = String::new();
  let mut in_block = false;
  let mut in_list = false;
  let mut title = String::new();

  for line in String::from_utf8_lossy(&response.data).to_string().lines() {
    match line.get(0..1).unwrap_or("") {
      // Convert links
      "=" => {
        let line = line.replace("=>", "").trim_start().to_owned();
        let mut split = line.split_whitespace().collect::<Vec<_>>();
        let mut href = split.remove(0).to_string();
        let text = split.join(" ");

        if href.starts_with('/') || !href.contains("://") {
          href = link_from_host_href(url, &href);
        }

        if var("PROXY_BY_DEFAULT").unwrap_or_else(|_| "true".to_string())
          == "true"
          && href.contains("gemini://")
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

        response_string
          .push_str(&format!("<p><a href=\"{}\">{}</a></p>\n", href, text));
      }
      // Add whitespace
      "" => {
        if in_list {
          in_list = false;
          response_string.push_str("</ul>\n");
        }

        response_string.push('\n');
      }
      // Convert lists
      "*" => {
        if !in_list {
          in_list = true;
          response_string.push_str("<ul>\n");
        }

        response_string.push_str(&format!(
          "<li>{}</li>\n",
          line.replace('*', "").trim_start()
        ));
      }
      // Convert headings
      "#" => {
        if in_list {
          in_list = false;
          response_string.push_str("</ul>\n");
        }

        match line.get(0..3) {
          Some(heading) =>
            match heading {
              "###" => {
                response_string.push_str(&format!(
                  "<h3>{}</h3>",
                  line.replace("###", "").trim_start()
                ));
              }
              _ =>
                if heading.starts_with("##") {
                  response_string.push_str(&format!(
                    "<h2>{}</h2>",
                    line.replace("##", "").trim_start()
                  ));
                } else {
                  let fixed_line =
                    line.replace('#', "").trim_start().to_owned();

                  response_string.push_str(&format!("<h1>{}</h1>", fixed_line));

                  if title.is_empty() {
                    title = fixed_line;
                  }
                },
            },
          None => {}
        }
      }
      // Convert blockquotes
      ">" => {
        if in_list {
          in_list = false;
          response_string.push_str("</ul>\n");
        }

        response_string.push_str(&format!(
          "<blockquote>{}</blockquote>\n",
          line.replace('>', "").trim_start()
        ));
      }
      // Convert preformatted blocks
      "`" => {
        if in_list {
          in_list = false;
          response_string.push_str("</ul>\n");
        }

        in_block = !in_block;
        if in_block {
          response_string.push_str("<pre>\n");
        } else {
          response_string.push_str("</pre>\n");
        }
      }
      // Add text lines
      _ => {
        if in_list {
          in_list = false;
          response_string.push_str("</ul>\n");
        }

        if in_block {
          response_string.push_str(&format!("{}\n", line));
        } else {
          response_string.push_str(&format!("<p>{}</p>", line));
        }
      }
    }
  }

  (title, response_string)
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
  let mut html_context = gemini_to_html(&response, &url, is_proxy);
  let convert_time_taken = timer.elapsed();

  // Try to add an external stylesheet from the `CSS_EXTERNAL` environment
  // variable.
  if let Ok(css) = var("CSS_EXTERNAL") {
    html_context.1.push_str(&format!(
      "<link rel=\"stylesheet\" type=\"text/css\" href=\"{}\">",
      css
    ));
  }

  // Add a title to HTML response
  html_context
    .1
    .push_str(&format!("<title>{}</title>", html_context.0));

  // Add proxy information to footer of HTML response
  html_context.1.push_str(&format!(
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
</details>",
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
      .body(html_context.1),
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
