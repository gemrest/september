// This file is part of September <https://github.com/gemrest/september>.
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
// Copyright (C) 2022-2023 Fuwn <contact@fuwn.me>
// SPDX-License-Identifier: GPL-3.0-only

use std::{env::var, time::Instant};

use actix_web::{Error, HttpResponse};

use crate::url::make as make_url;

#[allow(clippy::unused_async, clippy::future_not_send, clippy::too_many_lines)]
pub async fn default(
  req: actix_web::HttpRequest,
) -> Result<HttpResponse, Error> {
  if vec![
    "/proxy", "/proxy/", "/x", "/x/", "/raw", "/raw/", "/nocss", "/nocss/",
  ]
  .contains(&req.path())
  {
    return Ok(
      HttpResponse::Ok()
        .content_type("text/html")
        .body(r#"<pre>This is a proxy path. Please specify a Gemini URL without the "gemini://" to proxy.

For example: to proxy "gemini://fuwn.me/uptime", visit "/proxy/fuwn.me/uptime".</pre>"#),
    );
  }

  let mut is_proxy = false;
  let mut is_raw = false;
  let mut is_nocss = false;
  // Try to construct a Gemini URL
  let url = match make_url(
    &format!("{}{}", req.path(), {
      if !req.query_string().is_empty() || req.uri().to_string().ends_with('?')
      {
        format!("?{}", req.query_string())
      } else {
        String::new()
      }
    }),
    false,
    &mut is_proxy,
    &mut is_raw,
    &mut is_nocss,
  ) {
    Ok(url) => url,
    Err(e) => {
      return Ok(
        HttpResponse::BadRequest()
          .content_type("text/plain")
          .body(format!("{e}")),
      );
    }
  };
  // Make a request to get Gemini content and time it.
  let mut timer = Instant::now();
  let mut response = match gmi::request::make_request(&url) {
    Ok(response) => response,
    Err(e) => {
      return Ok(HttpResponse::Ok().body(e.to_string()));
    }
  };

  if response.data.is_empty() {
    response = match gmi::request::make_request(&match make_url(
      req.path(),
      true,
      &mut is_proxy,
      &mut is_raw,
      &mut is_nocss,
    ) {
      Ok(url) => url,
      Err(e) => {
        return Ok(
          HttpResponse::BadRequest()
            .content_type("text/plain")
            .body(format!("{e}")),
        );
      }
    }) {
      Ok(response) => response,
      Err(e) => {
        return Ok(HttpResponse::Ok().body(e.to_string()));
      }
    };
  }

  let response_time_taken = timer.elapsed();
  let meta = germ::meta::Meta::from_string(response.meta.clone());
  let charset = meta
    .parameters()
    .get("charset")
    .map_or_else(|| "utf-8".to_string(), ToString::to_string);
  let language = meta
    .parameters()
    .get("lang")
    .map_or_else(String::new, ToString::to_string);

  // Reset timer for below
  timer = Instant::now();

  // Convert Gemini Response to HTML and time it.
  let mut html_context = if is_raw {
    String::new()
  } else {
    format!(
      "<!DOCTYPE html><html{}><head>",
      if language.is_empty() {
        String::new()
      } else {
        format!(" lang=\"{language}\"")
      }
    )
  };
  let gemini_html = crate::html::from_gemini(&response, &url, is_proxy);
  let gemini_title = gemini_html.0;
  let convert_time_taken = timer.elapsed();

  if is_raw {
    html_context.push_str(&String::from_utf8_lossy(&response.data));

    return Ok(
      HttpResponse::Ok()
        .content_type(format!("{}; charset={charset}", meta.mime()))
        .body(html_context),
    );
  }

  if is_nocss {
    html_context.push_str(&gemini_html.1);

    return Ok(
      HttpResponse::Ok()
        .content_type(format!("text/html; charset={}", meta.mime()))
        .body(html_context),
    );
  }

  // Try to add an external stylesheet from the `CSS_EXTERNAL` environment
  // variable.
  if let Ok(css) = var("CSS_EXTERNAL") {
    let stylesheets =
      css.split(',').filter(|s| !s.is_empty()).collect::<Vec<_>>();

    for stylesheet in stylesheets {
      html_context.push_str(&format!(
        "<link rel=\"stylesheet\" type=\"text/css\" href=\"{stylesheet}\">",
      ));
    }
  }

  // Try to add an external favicon from the `FAVICON_EXTERNAL` environment
  // variable.
  if let Ok(favicon) = var("FAVICON_EXTERNAL") {
    html_context.push_str(&format!(
      "<link rel=\"icon\" type=\"image/x-icon\" href=\"{favicon}\">",
    ));
  }

  if var("MATHJAX")
    .unwrap_or_else(|_| "true".to_string())
    .to_lowercase()
    == "true"
  {
    html_context.push_str(
      r#"<script type="text/javascript" id="MathJax-script" async
        src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js">
    </script>"#,
    );
  }

  // Add a title to HTML response
  html_context.push_str(&format!("<title>{gemini_title}</title>"));
  html_context.push_str("</head><body>");

  match response.status {
    gmi::protocol::StatusCode::Success(_) => {
      html_context.push_str(&gemini_html.1);
    }
    _ => html_context.push_str(&format!("<p>{}</p>", response.meta)),
  }

  // Add proxy information to footer of HTML response
  html_context.push_str(&format!(
    "<details>\n<summary>Proxy information</summary>
<dl>
<dt>Original URL</dt><dd><a href=\"{}\">{0}</a></dd>
<dt>Status code</dt>
<dd>{:?}</dd>
<dt>Meta</dt><dd>{}</dd>
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

  if let Ok(plain_texts) = var("PLAIN_TEXT_ROUTE") {
    if plain_texts.split(',').any(|r| r == req.path()) {
      return Ok(
        HttpResponse::Ok()
          .body(String::from_utf8_lossy(&response.data).to_string()),
      );
    }
  }

  Ok(
    HttpResponse::Ok()
      .content_type(format!("text/html; charset={charset}"))
      .body(html_context),
  )
}
