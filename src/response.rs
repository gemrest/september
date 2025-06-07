pub mod configuration;

use {
  crate::url::{from_path as url_from_path, matches_pattern},
  actix_web::{Error, HttpResponse},
  std::{env::var, fmt::Write, time::Instant},
};

const CSS: &str = include_str!("../default.css");

#[allow(clippy::future_not_send, clippy::too_many_lines)]
pub async fn default(
  http_request: actix_web::HttpRequest,
) -> Result<HttpResponse, Error> {
  if ["/proxy", "/proxy/", "/x", "/x/", "/raw", "/raw/", "/nocss", "/nocss/"]
    .contains(&http_request.path())
  {
    return Ok(HttpResponse::Ok()
        .content_type("text/html")
      .body(r"<h1>September</h1>
<p>This is a proxy path. Specify a Gemini URL without the protocol (<code>gemini://</code>) to proxy it.</p>
<p>To proxy <code>gemini://fuwn.me/uptime</code>, visit <code>https://fuwn.me/proxy/fuwn.me/uptime</code>.</p>
<p>Additionally, you may visit <code>/raw</code> to view the raw Gemini content, or <code>/nocss</code> to view the content without CSS.</p>
      "));
  }

  let mut configuration = configuration::Configuration::new();
  let url = match url_from_path(
    &format!("{}{}", http_request.path(), {
      if !http_request.query_string().is_empty()
        || http_request.uri().to_string().ends_with('?')
      {
        format!("?{}", http_request.query_string())
      } else {
        String::new()
      }
    }),
    false,
    &mut configuration,
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
  let mut timer = Instant::now();
  let mut response = match germ::request::request(&url).await {
    Ok(response) => response,
    Err(e) => {
      return Ok(HttpResponse::Ok().body(e.to_string()));
    }
  };
  let mut redirect_response_status = None;
  let mut redirect_url = None;

  if *response.status() == germ::request::Status::PermanentRedirect
    || *response.status() == germ::request::Status::TemporaryRedirect
  {
    redirect_response_status = Some(*response.status());
    redirect_url = Some(
      url::Url::parse(&if response.meta().starts_with('/') {
        format!(
          "gemini://{}{}",
          url.domain().unwrap_or_default(),
          response.meta()
        )
      } else {
        response.meta().to_string()
      })
      .unwrap(),
    );
    response =
      match germ::request::request(&redirect_url.clone().unwrap()).await {
        Ok(response) => response,
        Err(e) => {
          return Ok(HttpResponse::Ok().body(e.to_string()));
        }
      }
  }

  let response_time_taken = timer.elapsed();
  let meta = germ::meta::Meta::from_string(response.meta().to_string());
  let charset = meta
    .parameters()
    .get("charset")
    .map_or_else(|| "utf-8".to_string(), ToString::to_string);
  let language =
    meta.parameters().get("lang").map_or_else(String::new, ToString::to_string);

  timer = Instant::now();

  if response.meta().starts_with("image/") {
    if let Some(content_bytes) = &response.content_bytes() {
      return Ok(
        HttpResponse::build(actix_web::http::StatusCode::OK)
          .content_type(response.meta().as_ref())
          .body(content_bytes.to_vec()),
      );
    }
  }

  let mut html_context = if configuration.is_raw() {
    String::new()
  } else {
    format!(
      r#"<!DOCTYPE html><html{}><head><meta name="viewport" content="width=device-width, initial-scale=1.0">"#,
      if language.is_empty() {
        String::new()
      } else {
        format!(" lang=\"{language}\"")
      }
    )
  };
  let gemini_html =
    crate::html::from_gemini(&response, &url, &configuration).unwrap();
  let gemini_title = gemini_html.0;
  let convert_time_taken = timer.elapsed();

  if configuration.is_raw() {
    html_context.push_str(
      &response.content().as_ref().map_or_else(String::default, String::clone),
    );

    return Ok(
      HttpResponse::Ok()
        .content_type(format!("{}; charset={charset}", meta.mime()))
        .body(html_context),
    );
  }

  if configuration.is_no_css() {
    html_context.push_str(&gemini_html.1);

    return Ok(
      HttpResponse::Ok()
        .content_type(format!("text/html; charset={}", meta.mime()))
        .body(html_context),
    );
  }

  if let Ok(css) = var("CSS_EXTERNAL") {
    let stylesheets =
      css.split(',').filter(|s| !s.is_empty()).collect::<Vec<_>>();

    for stylesheet in stylesheets {
      let _ = write!(
        &mut html_context,
        "<link rel=\"stylesheet\" type=\"text/css\" href=\"{stylesheet}\">",
      );
    }
  } else if !configuration.is_no_css() {
    let _ = write!(
      &mut html_context,
      r#"<link rel="stylesheet" href="https://latex.vercel.app/style.css"><style>{CSS}</style>"#
    );

    if let Ok(primary) = var("PRIMARY_COLOUR") {
      let _ = write!(
        &mut html_context,
        "<style>:root {{ --primary: {primary} }}</style>"
      );
    } else {
      let _ = write!(
        &mut html_context,
        "<style>:root {{ --primary: var(--base0D); }}</style>"
      );
    }
  }

  if let Ok(favicon) = var("FAVICON_EXTERNAL") {
    let _ = write!(
      &mut html_context,
      "<link rel=\"icon\" type=\"image/x-icon\" href=\"{favicon}\">",
    );
  }

  if var("MATHJAX").unwrap_or_else(|_| "true".to_string()).to_lowercase()
    == "true"
  {
    html_context.push_str(
      r#"<script type="text/javascript" id="MathJax-script" async
        src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js">
    </script>"#,
    );
  }

  if let Ok(head) = var("HEAD") {
    html_context.push_str(&head);
  }

  let _ = write!(&mut html_context, "<title>{gemini_title}</title>");
  let _ = write!(&mut html_context, "</head><body>");

  if !http_request.path().starts_with("/proxy") {
    if let Ok(header) = var("HEADER") {
      let _ = write!(
        &mut html_context,
        "<big><blockquote>{header}</blockquote></big>"
      );
    }
  }

  match response.status() {
    germ::request::Status::Success => {
      if let (Some(status), Some(url)) =
        (redirect_response_status, redirect_url)
      {
        let _ = write!(
          &mut html_context,
          "<blockquote>This page {} redirects to <a \
           href=\"{}\">{}</a>.</blockquote>",
          if status == germ::request::Status::PermanentRedirect {
            "permanently"
          } else {
            "temporarily"
          },
          url,
          url
        );
      }

      html_context.push_str(&gemini_html.1);
    }
    _ => {
      let _ = write!(&mut html_context, "<p>{}</p>", response.meta());
    }
  }

  let _ = write!(
    &mut html_context,
    "<details>\n<summary>Proxy Information</summary>
<dl>
<dt>Original URL</dt><dd><a \
     href=\"{}\">{0}</a></dd>
<dt>Status Code</dt>
<dd>{} ({})</dd>
<dt>Meta</dt><dd><code>{}</code></dd>\

<dt>Capsule Response Time</dt>
<dd>{} milliseconds</dd>
<dt>Gemini-to-HTML \
     Time</dt>
<dd>{} milliseconds</dd>
</dl>
<p>This content has been proxied \
     by <a href=\"https://github.com/gemrest/september{}\">September \
     ({})</a>.</p>
</details></body></html>",
    url,
    response.status(),
    i32::from(*response.status()),
    response.meta(),
    response_time_taken.as_nanos() as f64 / 1_000_000.0,
    convert_time_taken.as_nanos() as f64 / 1_000_000.0,
    format_args!("/tree/{}", env!("VERGEN_GIT_SHA")),
    env!("VERGEN_GIT_SHA").get(0..5).unwrap_or("UNKNOWN"),
  );

  if let Ok(plain_texts) = var("PLAIN_TEXT_ROUTE") {
    if plain_texts.split(',').any(|r| {
      matches_pattern(r, http_request.path())
        || matches_pattern(r, http_request.path().trim_end_matches('/'))
    }) {
      return Ok(HttpResponse::Ok().body(
        response.content().as_ref().map_or_else(String::default, String::clone),
      ));
    }
  }

  Ok(
    HttpResponse::Ok()
      .content_type(format!("text/html; charset={charset}"))
      .body(html_context),
  )
}
