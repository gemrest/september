pub mod configuration;

use {
  crate::{
    environment::ENVIRONMENT,
    gemini,
    html::html_escape,
    robots::{self, Access},
    url::{from_path as url_from_path, matches_pattern},
  },
  actix_web::{
    Error, HttpResponse, HttpResponseBuilder,
    http::{StatusCode, header},
  },
  std::{fmt::Write, time::Instant},
};

const CSS: &str = include_str!("../default.css");
const REDIRECT_LIMIT: usize = 5;
const MAXIMUM_PROXY_DURATION: std::time::Duration =
  std::time::Duration::from_secs(90);

// Remote documents must not run scripts or inherit this site's origin.
fn sandboxed_upstream_response(status: StatusCode) -> HttpResponseBuilder {
  let mut response = HttpResponse::build(status);

  response.insert_header((header::CONTENT_SECURITY_POLICY, "sandbox"));
  response.insert_header((header::X_CONTENT_TYPE_OPTIONS, "nosniff"));

  response
}

async fn robots_rejection(url: &url::Url) -> Option<HttpResponse> {
  match robots::check_access(url).await {
    Access::Allowed => None,
    Access::Denied => Some(
      HttpResponse::Forbidden()
        .content_type("text/plain; charset=utf-8")
        .body("The destination capsule prohibits access through web proxies."),
    ),
    Access::Unavailable => Some(
      HttpResponse::ServiceUnavailable()
        .content_type("text/plain; charset=utf-8")
        .body("The destination capsule's robots.txt could not be checked."),
    ),
  }
}

fn upstream_error(message: impl std::fmt::Display) -> HttpResponse {
  HttpResponse::BadGateway()
    .content_type("text/plain; charset=utf-8")
    .body(message.to_string())
}

// Percent-encode user input for use as a Gemini query. Encoding is done
// byte-wise so that multi-byte UTF-8 sequences are encoded correctly.
fn percent_encode_query(input: &str) -> String {
  let mut encoded = String::with_capacity(input.len());

  for byte in input.bytes() {
    match byte {
      b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' =>
        encoded.push(char::from(byte)),
      _ => {
        let _ = write!(&mut encoded, "%{byte:02X}");
      }
    }
  }

  encoded
}

fn document_head(language: &str, title: &str, include_css: bool) -> String {
  let mut head = format!(
    r#"<!DOCTYPE html><html{}><head><meta name="viewport" content="width=device-width, initial-scale=1.0">"#,
    if language.is_empty() {
      String::new()
    } else {
      format!(" lang=\"{}\"", html_escape(language))
    }
  );

  if include_css {
    if let Some(css) = &ENVIRONMENT.css_external {
      for stylesheet in css.split(',').filter(|s| !s.is_empty()) {
        let _ = write!(
          &mut head,
          "<link rel=\"stylesheet\" type=\"text/css\" href=\"{stylesheet}\">",
        );
      }
    } else {
      let _ = write!(
        &mut head,
        r#"<link rel="stylesheet" href="https://latex.vercel.app/style.css"><style>{CSS}</style>"#
      );
      let _ = write!(
        &mut head,
        "<style>:root {{ --primary: {} }}</style>",
        ENVIRONMENT.primary_color.as_deref().unwrap_or("var(--base0D)")
      );
    }

    if ENVIRONMENT.mathjax {
      head.push_str(
        r#"<script type="text/javascript" id="MathJax-script" async
        src="https://cdn.jsdelivr.net/npm/mathjax@3.2.2/es5/tex-mml-chtml.js"
        integrity="sha384-Wuix6BuhrWbjDBs24bXrjf4ZQ5aFeFWBuKkFekO2t8xFU0iNaLQfp2K6/1Nxveei"
        crossorigin="anonymous">
    </script>"#,
      );
    }
  }

  if let Some(favicon) = &ENVIRONMENT.favicon_external {
    let _ = write!(
      &mut head,
      "<link rel=\"icon\" type=\"image/x-icon\" href=\"{favicon}\">",
    );
  }

  if let Some(head_content) = &ENVIRONMENT.head {
    head.push_str(head_content);
  }

  let _ = write!(&mut head, "<title>{title}</title></head><body>");

  head
}

fn body_preamble(
  request_path: &str,
  redirect_response_status: Option<germ::request::Status>,
  redirect_url: Option<&url::Url>,
) -> String {
  let mut preamble = String::new();

  if !request_path.starts_with("/proxy") {
    if let Some(header) = &ENVIRONMENT.header {
      let _ =
        write!(&mut preamble, "<big><blockquote>{header}</blockquote></big>");
    }
  }

  if let (Some(status), Some(redirected_to)) =
    (redirect_response_status, redirect_url)
  {
    let _ = write!(
      &mut preamble,
      "<blockquote>This page {} redirects to <a \
       href=\"{}\">{}</a>.</blockquote>",
      if status == germ::request::Status::PermanentRedirect {
        "permanently"
      } else {
        "temporarily"
      },
      redirected_to,
      redirected_to
    );
  }

  preamble
}

#[derive(serde::Deserialize)]
pub struct InputSubmission {
  input:  String,
  target: Option<String>,
}

#[allow(clippy::future_not_send)]
pub async fn default(
  http_request: actix_web::HttpRequest,
  input_submission: Option<actix_web::web::Form<InputSubmission>>,
) -> Result<HttpResponse, Error> {
  tokio::time::timeout(
    MAXIMUM_PROXY_DURATION,
    default_inner(http_request, input_submission),
  )
  .await
  .unwrap_or_else(|_| {
    Ok(
      HttpResponse::GatewayTimeout()
        .content_type("text/plain; charset=utf-8")
        .body("The Gemini request timed out."),
    )
  })
}

#[allow(clippy::future_not_send, clippy::too_many_lines)]
async fn default_inner(
  http_request: actix_web::HttpRequest,
  input_submission: Option<actix_web::web::Form<InputSubmission>>,
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

  let mut configuration = configuration::Configuration::default();
  let submitted_input =
    if *http_request.method() == actix_web::http::Method::POST {
      input_submission.as_ref().map(|submission| submission.input.clone())
    } else {
      None
    };
  let submitted_target =
    if *http_request.method() == actix_web::http::Method::POST {
      input_submission.as_ref().and_then(|submission| submission.target.clone())
    } else {
      None
    };
  let mut url = match url_from_path(
    &format!("{}{}", http_request.path(), {
      if !http_request.query_string().is_empty()
        || http_request.uri().to_string().ends_with('?')
      {
        format!("?{}", http_request.query_string())
      } else {
        String::new()
      }
    }),
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

  if let Some(target) = submitted_target {
    if let Ok(parsed_target) = url::Url::parse(&target) {
      if parsed_target.scheme() == "gemini" {
        url = parsed_target;
      }
    }
  }

  if let Some(input) = submitted_input {
    let input = input.replace("\r\n", "\n").replace('\r', "\n");

    url.set_query(Some(&percent_encode_query(&input)));
  }

  if let Some(rejection) = robots_rejection(&url).await {
    return Ok(rejection);
  }

  let mut timer = Instant::now();
  let mut response = match gemini::request(&url).await {
    Ok(response) => response,
    Err(error) => return Ok(upstream_error(error)),
  };
  let mut redirect_response_status = None;
  let mut redirect_url: Option<url::Url> = None;

  for _ in 0..REDIRECT_LIMIT {
    if *response.status() != germ::request::Status::PermanentRedirect
      && *response.status() != germ::request::Status::TemporaryRedirect
    {
      break;
    }

    let target =
      match redirect_url.as_ref().unwrap_or(&url).join(&response.meta()) {
        Ok(target) => target,
        Err(error) =>
          return Ok(upstream_error(format!(
            "invalid redirect target: {error}"
          ))),
      };

    redirect_response_status.get_or_insert_with(|| *response.status());

    if let Some(rejection) = robots_rejection(&target).await {
      return Ok(rejection);
    }

    response = match gemini::request(&target).await {
      Ok(response) => response,
      Err(error) => return Ok(upstream_error(error)),
    };
    redirect_url = Some(target);
  }

  let response_time_taken = timer.elapsed();
  let meta = germ::meta::Meta::from_string(response.meta().to_string());
  let charset = meta
    .parameters()
    .get("charset")
    .map_or_else(|| "utf-8".to_string(), ToString::to_string);
  let language =
    meta.parameters().get("lang").map_or_else(String::new, ToString::to_string);
  let http_status = match i32::from(*response.status()) {
    20..=29 => actix_web::http::StatusCode::OK,
    51 => actix_web::http::StatusCode::NOT_FOUND,
    52 => actix_web::http::StatusCode::GONE,
    _ => actix_web::http::StatusCode::BAD_GATEWAY,
  };

  timer = Instant::now();

  if response.meta().starts_with("image/") {
    if let Some(content_bytes) = &response.content_bytes() {
      return Ok(
        sandboxed_upstream_response(http_status)
          .content_type(response.meta().as_ref())
          .body(content_bytes.to_vec()),
      );
    }
  }

  if let Some(plain_texts) = &ENVIRONMENT.plain_text_route {
    if plain_texts.split(',').any(|r| {
      matches_pattern(r, http_request.path())
        || matches_pattern(r, http_request.path().trim_end_matches('/'))
    }) {
      return Ok(
        HttpResponse::build(http_status)
          .content_type(format!("text/plain; charset={charset}"))
          .body(
            response
              .content()
              .as_ref()
              .map_or_else(String::default, String::clone),
          ),
      );
    }
  }

  if *response.status() == germ::request::Status::Input
    || *response.status() == germ::request::Status::SensitiveInput
  {
    if configuration.raw {
      return Ok(
        HttpResponse::Ok()
          .content_type(format!("text/plain; charset={charset}"))
          .body(response.meta().to_string()),
      );
    }

    let mut html_context = document_head(
      &language,
      &html_escape(&response.meta()),
      !configuration.no_css,
    );

    html_context.push_str(&body_preamble(
      http_request.path(),
      redirect_response_status,
      redirect_url.as_ref(),
    ));

    let input_url = redirect_url.unwrap_or_else(|| url.clone());
    let input_field =
      if *response.status() == germ::request::Status::SensitiveInput {
        "<input name=\"input\" type=\"password\" autofocus>"
      } else {
        "<textarea name=\"input\" rows=\"8\" autofocus></textarea>"
      };
    let _ = write!(
      &mut html_context,
      "<p>{}</p><form method=\"post\" action=\"{}\"><input type=\"hidden\" \
       name=\"target\" value=\"{}\">{}<button \
       type=\"submit\">Submit</button></form></body></html>",
      html_escape(&response.meta()),
      html_escape(&http_request.uri().to_string()),
      html_escape(input_url.as_ref()),
      input_field,
    );
    let mut response_builder = HttpResponse::Ok();

    if *response.status() == germ::request::Status::SensitiveInput {
      response_builder
        .insert_header((actix_web::http::header::CACHE_CONTROL, "no-store"));
    }

    return Ok(
      response_builder
        .content_type(format!("text/html; charset={charset}"))
        .body(html_context),
    );
  }

  if configuration.raw {
    return Ok(
      sandboxed_upstream_response(http_status)
        .content_type(format!("{}; charset={charset}", meta.mime()))
        .body(
          response
            .content()
            .as_ref()
            .map_or_else(String::default, String::clone),
        ),
    );
  }

  let content = response.content().unwrap_or_default();
  let rendered_url = redirect_url.as_ref().unwrap_or(&url);
  let Some((gemini_title, gemini_body)) =
    crate::html::from_gemini(&content, rendered_url, &configuration)
  else {
    return Ok(upstream_error("could not convert Gemini content to HTML"));
  };
  let convert_time_taken = timer.elapsed();

  if configuration.no_css {
    return Ok(
      HttpResponse::build(http_status)
        .content_type(format!("text/html; charset={charset}"))
        .body(gemini_body),
    );
  }

  let mut html_context = document_head(&language, &gemini_title, true);

  html_context.push_str(&body_preamble(
    http_request.path(),
    redirect_response_status,
    redirect_url.as_ref(),
  ));

  if *response.status() == germ::request::Status::Success {
    html_context.push_str(&gemini_body);
  } else {
    let _ =
      write!(&mut html_context, "<p>{}</p>", html_escape(&response.meta()));
  }

  let _ = write!(
    &mut html_context,
    "<details>\n<summary>Proxy Information</summary>
<dl>
<dt>Original URL</dt><dd><a href=\"{}\">{0}</a></dd>
<dt>Status Code</dt><dd>{} ({})</dd>
<dt>Meta</dt><dd><code>{}</code></dd>
<dt>Capsule Response Time</dt><dd>{} milliseconds</dd>
<dt>Gemini-to-HTML Time</dt><dd>{} milliseconds</dd>
</dl>
<p>This content has been proxied by <a \
     href=\"https://github.com/gemrest/september{}\">September ({})</a>.</p>
</details></body></html>",
    url,
    response.status(),
    i32::from(*response.status()),
    html_escape(&response.meta()),
    response_time_taken.as_nanos() as f64 / 1_000_000.0,
    convert_time_taken.as_nanos() as f64 / 1_000_000.0,
    format_args!("/tree/{}", env!("GIT_SHA")),
    env!("GIT_SHA").get(0..5).unwrap_or("UNKNOWN"),
  );

  Ok(
    HttpResponse::build(http_status)
      .content_type(format!("text/html; charset={charset}"))
      .body(html_context),
  )
}

#[cfg(test)]
mod tests {
  use {
    super::{document_head, sandboxed_upstream_response, upstream_error},
    actix_web::http::{StatusCode, header},
  };

  #[test]
  fn escapes_gemini_language_in_html_attribute() {
    let meta =
      germ::meta::Meta::from_string("text/gemini; lang=en\" onload=\"alert(1)");
    let language = meta.parameters().get("lang").unwrap();
    let head = document_head(language, "Title", false);

    assert!(head.starts_with(
      "<!DOCTYPE html><html lang=\"en&quot; onload=&quot;alert(1)\""
    ));
    assert!(!head.contains(" onload=\""));
    assert!(
      document_head("en&quot; onload=alert(1)", "Title", false).starts_with(
        "<!DOCTYPE html><html lang=\"en&amp;quot; onload=alert(1)\""
      )
    );
    assert!(
      document_head("en-GB", "Title", false)
        .starts_with("<!DOCTYPE html><html lang=\"en-GB\"")
    );
  }

  #[test]
  fn sandboxes_direct_upstream_content() {
    for content_type in ["text/html", "image/svg+xml", "image/png"] {
      let response = sandboxed_upstream_response(StatusCode::OK)
        .content_type(content_type)
        .body("upstream body");

      assert_eq!(
        response.headers().get(header::CONTENT_SECURITY_POLICY).unwrap(),
        "sandbox"
      );
      assert_eq!(
        response.headers().get(header::X_CONTENT_TYPE_OPTIONS).unwrap(),
        "nosniff"
      );
      assert_eq!(
        response.headers().get(header::CONTENT_TYPE).unwrap(),
        content_type
      );
    }
  }

  #[test]
  fn reports_upstream_errors_as_bad_gateway() {
    let response = upstream_error("capsule unavailable");

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    assert_eq!(
      response.headers().get(header::CONTENT_TYPE).unwrap(),
      "text/plain; charset=utf-8"
    );
  }
}
