use {
  crate::{environment::ENVIRONMENT, url::matches_pattern},
  germ::ast::Node,
  std::fmt::Write,
  url::Url,
};

pub fn html_escape(input: &str) -> String {
  input
    .replace('&', "&amp;")
    .replace('"', "&quot;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
}

// Browsers strip control characters when parsing URLs, so remove them before
// checking the scheme to prevent smuggling (e.g. "java\tscript:").
fn sanitize_href(href: &str) -> String {
  let cleaned =
    href.chars().filter(|c| !c.is_ascii_control()).collect::<String>();
  let scheme = cleaned.split(':').next().unwrap_or("").to_ascii_lowercase();

  if matches!(scheme.as_str(), "javascript" | "data" | "vbscript") {
    return "#".to_string();
  }

  html_escape(&cleaned)
}

fn link_from_host_href(url: &Url, href: &str) -> Option<String> {
  if let Some(destination) = href.strip_prefix("/proxy/") {
    Some(format!("gemini://{destination}"))
  } else {
    Some(format!(
      "gemini://{}{}{}",
      url.host_str()?,
      { if href.starts_with('/') { "" } else { "/" } },
      href
    ))
  }
}

fn resolve_link(url: &Url, href: &str) -> Option<String> {
  if href.starts_with('/') && !href.starts_with("//") {
    return if href.starts_with("/proxy/") {
      link_from_host_href(url, href)
    } else {
      Some(url.join(href).ok()?.to_string())
    };
  }

  if href.contains(':') {
    return Some(href.to_string());
  }

  Some(url.join(href).ok()?.to_string())
}

fn render_text(text: &str) -> String {
  let is_ordered_list = text.starts_with(|c: char| c.is_ascii_digit())
    && text.get(1..3) == Some(". ");

  if is_ordered_list {
    html_escape(text)
  } else {
    comrak::markdown_to_html(text, &comrak::ComrakOptions::default())
      .replace("<p>", "")
      .replace("</p>", "")
  }
}

#[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
pub fn from_gemini(
  response: &germ::request::Response,
  url: &Url,
  configuration: &crate::response::configuration::Configuration,
) -> Option<(String, String)> {
  const GEMINI_FRAGMENT: &str =
    r#"<span class="gemini-fragment">=&#62; </span>"#;
  let ast_tree = germ::ast::Ast::from_string(
    response.content().as_ref().map_or_else(String::default, String::clone),
  );
  let ast = ast_tree.inner();
  let mut html = String::new();
  let mut title = String::new();
  let mut previous_link = false;
  let mut previous_link_count = 0;
  let condense_links =
    ENVIRONMENT.condense_links.contains(&url.path().to_string())
      || ENVIRONMENT.condense_links.contains(&"*".to_string());
  let condensible_headings = ENVIRONMENT
    .condense_links_at_headings
    .iter()
    .map(String::as_str)
    .collect::<Vec<_>>();
  // Condensing under CONDENSE_LINKS_AT_HEADINGS only begins once one of the
  // listed headings has been seen.
  let mut in_condense_links_flag_trap = false;

  for node in ast {
    if condensible_headings.contains(&node.to_gemtext().as_str()) {
      in_condense_links_flag_trap = true;
    }

    let align_adjacent_links = |html: &str| {
      if previous_link_count > 0 {
        html.rfind(GEMINI_FRAGMENT).map_or_else(
          || html.to_string(),
          |position| {
            let mut result =
              String::with_capacity(html.len() - GEMINI_FRAGMENT.len());

            result.push_str(&html[..position]);
            result.push_str(&html[position + GEMINI_FRAGMENT.len()..]);

            result
          },
        )
      } else {
        html.to_string()
      }
    };

    if previous_link
      && (!matches!(node, Node::Link { .. })
        || (!condense_links && !in_condense_links_flag_trap))
    {
      if matches!(node, Node::Link { .. }) {
        html.push_str("<br />");
      } else {
        html.push_str("</p>");
      }

      previous_link = false;
      html = align_adjacent_links(&html);
      previous_link_count = 0;
    } else if previous_link {
      html = align_adjacent_links(&html);

      html.push_str(r#" <span class="gemini-fragment">|</span> "#);

      previous_link_count += 1;
    } else if !previous_link && matches!(node, Node::Link { .. }) {
      html.push_str("<p>");
    }

    match node {
      Node::Text(text) => {
        let _ = write!(&mut html, "<p>{}</p>", render_text(text));
      }
      Node::Link { to, text } => {
        let mut href = resolve_link(url, to)?;
        let external_scheme =
          href.contains("://") && !href.starts_with("gemini://");

        if ENVIRONMENT.proxy_by_default
          && href.contains("gemini://")
          && !external_scheme
        {
          if configuration.proxy
            || configuration.no_css
            || href
              .trim_start_matches("gemini://")
              .trim_end_matches('/')
              .split('/')
              .next()
              .unwrap_or_default()
              != url.host_str().unwrap_or_default()
          {
            href = format!(
              "/{}/{}",
              if configuration.no_css { "nocss" } else { "proxy" },
              href.trim_start_matches("gemini://")
            );
          } else {
            href = href.trim_start_matches("gemini://").replacen(
              url.host_str()?,
              "",
              1,
            );
          }
        }

        if let Some(patterns) = &ENVIRONMENT.keep_gemini {
          if (href.starts_with('/') || !href.contains("://"))
            && !external_scheme
          {
            let temporary_href = link_from_host_href(url, &href)?;
            let should_exclude = patterns
              .iter()
              .filter(|p| p.starts_with('!'))
              .any(|p| matches_pattern(&p[1..], &temporary_href));

            if !should_exclude {
              let should_include = patterns
                .iter()
                .filter(|p| !p.starts_with('!'))
                .any(|p| matches_pattern(p, &temporary_href));

              if should_include {
                href = temporary_href;
              }
            }
          }
        }

        if let Some(embed_images) = &ENVIRONMENT.embed_images {
          let href_path = href.split(['?', '#']).next().unwrap_or(&href);

          if let Some(extension) = std::path::Path::new(href_path).extension() {
            if extension == "png"
              || extension == "jpg"
              || extension == "jpeg"
              || extension == "gif"
              || extension == "webp"
              || extension == "svg"
            {
              if embed_images == "1" {
                let _ = writeln!(
                  &mut html,
                  "<p><a href=\"{}\">{}</a> <i>Embedded below</i></p>",
                  sanitize_href(&href),
                  render_text(text.as_ref().unwrap_or(to)),
                );
              }

              let _ = writeln!(
                &mut html,
                "<p><img src=\"{}\" alt=\"{}\" /></p>",
                sanitize_href(&href),
                html_escape(text.as_ref().unwrap_or(to)),
              );

              continue;
            }
          }
        }

        previous_link = true;

        let _ = write!(
          &mut html,
          r#"{}<a href="{}">{}</a>"#,
          GEMINI_FRAGMENT,
          sanitize_href(&href),
          render_text(text.as_ref().unwrap_or(to)).trim(),
        );
      }
      Node::Heading { level, text } => {
        if !condensible_headings.contains(&node.to_gemtext().as_str()) {
          in_condense_links_flag_trap = false;
        }

        if title.is_empty() && *level == 1 {
          title = render_text(text).trim().to_string();
        }

        let _ = write!(
          &mut html,
          "<{}>{}</{0}>",
          match level {
            1 => "h1",
            2 => "h2",
            3 => "h3",
            _ => "p",
          },
          render_text(text),
        );
      }
      Node::List(items) => {
        let _ = write!(
          &mut html,
          "<ul>{}</ul>",
          items
            .iter()
            .map(|i| format!("<li>{}</li>", render_text(i)))
            .collect::<Vec<String>>()
            .join("\n")
        );
      }
      Node::Blockquote(text) => {
        let _ =
          write!(&mut html, "<blockquote>{}</blockquote>", render_text(text));
      }
      Node::PreformattedText { text, .. } => {
        let new_text = text.strip_suffix('\n').unwrap_or(text);
        let _ = write!(&mut html, "<pre>{}</pre>", html_escape(new_text));
      }
      Node::Whitespace => {}
    }
  }

  Some((title, html))
}

#[cfg(test)]
mod tests {
  use {
    super::{link_from_host_href, render_text, resolve_link},
    url::Url,
  };

  #[test]
  fn resolves_links_relative_to_the_current_document() {
    let url = Url::parse("gemini://example.org:1966/dir/page").unwrap();

    assert_eq!(
      resolve_link(&url, "next").as_deref(),
      Some("gemini://example.org:1966/dir/next")
    );
    assert_eq!(
      resolve_link(&url, "../next").as_deref(),
      Some("gemini://example.org:1966/next")
    );
    assert_eq!(
      resolve_link(&url, "?q=one").as_deref(),
      Some("gemini://example.org:1966/dir/page?q=one")
    );
    assert_eq!(
      resolve_link(&url, "/next").as_deref(),
      Some("gemini://example.org:1966/next")
    );
    assert_eq!(
      link_from_host_href(&url, "/proxy/other.org/proxy/next").as_deref(),
      Some("gemini://other.org/proxy/next")
    );
  }

  #[test]
  fn escapes_numbered_text_without_changing_its_label() {
    assert_eq!(
      render_text("1. <script>alert(1)</script>"),
      "1. &lt;script&gt;alert(1)&lt;/script&gt;"
    );
    assert_eq!(
      render_text("1. <img src=x onerror=alert(1)>"),
      "1. &lt;img src=x onerror=alert(1)&gt;"
    );
    assert_eq!(render_text("1. First item"), "1. First item");
  }

  #[test]
  fn omits_raw_html_in_other_numbered_forms() {
    assert!(!render_text("10. <script>alert(1)</script>").contains("<script>"));
    assert!(!render_text("1) <script>alert(1)</script>").contains("<script>"));
  }
}
