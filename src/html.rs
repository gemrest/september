use {
  crate::{environment::ENVIRONMENT, url::matches_pattern},
  germ::{ast::Node, convert::html::Renderer},
  std::fmt::Write,
  url::Url,
};

const GEMINI_FRAGMENT: &str = r#"<span class="gemini-fragment">=&#62; </span>"#;

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

fn embedded_image(href: &str, label: &str, mode: &str) -> Option<String> {
  let href_path = href.split(['?', '#']).next().unwrap_or(href);
  let extension = std::path::Path::new(href_path).extension()?.to_str()?;

  if !["png", "jpg", "jpeg", "gif", "webp", "svg"].contains(&extension) {
    return None;
  }

  let mut html = String::new();

  if mode == "1" {
    let _ = write!(
      &mut html,
      "<p><a href=\"{}\">{}</a> <i>Embedded below</i></p>",
      sanitize_href(href),
      render_text(label).trim(),
    );
  }

  let _ = write!(
    &mut html,
    "<p><img src=\"{}\" alt=\"{}\" /></p>",
    sanitize_href(href),
    html_escape(label),
  );

  Some(html)
}

fn align_adjacent_links(html: &str, previous_link_count: usize) -> String {
  if previous_link_count == 0 {
    return html.to_string();
  }

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
}

struct ProxyRenderer<'a> {
  url:                          &'a Url,
  configuration: &'a crate::response::configuration::Configuration,
  title:                        String,
  previous_link_count:          Option<usize>,
  condense_links:               bool,
  condense_links_under_heading: bool,
  link_failed:                  bool,
}

impl ProxyRenderer<'_> {
  fn close_links(&mut self, html: &mut String) {
    if let Some(previous_link_count) = self.previous_link_count.take() {
      html.push_str("</p>");

      *html = align_adjacent_links(html, previous_link_count);
    }
  }

  #[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
  fn render_link(
    &mut self,
    destination: &str,
    label: Option<&str>,
    html: &mut String,
  ) -> Option<()> {
    let label = label.unwrap_or(destination);
    let mut href = resolve_link(self.url, destination)?;
    let external_scheme =
      href.contains("://") && !href.starts_with("gemini://");

    if ENVIRONMENT.proxy_by_default
      && href.contains("gemini://")
      && !external_scheme
    {
      let destination_host = href
        .trim_start_matches("gemini://")
        .trim_end_matches('/')
        .split('/')
        .next()
        .unwrap_or_default();

      if self.configuration.proxy
        || self.configuration.no_css
        || destination_host != self.url.host_str().unwrap_or_default()
      {
        href = format!(
          "/{}/{}",
          if self.configuration.no_css { "nocss" } else { "proxy" },
          href.trim_start_matches("gemini://")
        );
      } else {
        href = href.trim_start_matches("gemini://").replacen(
          self.url.host_str()?,
          "",
          1,
        );
      }
    }

    if let Some(patterns) = &ENVIRONMENT.keep_gemini {
      if (href.starts_with('/') || !href.contains("://")) && !external_scheme {
        let temporary_href = link_from_host_href(self.url, &href)?;
        let should_exclude = patterns
          .iter()
          .filter(|pattern| pattern.starts_with('!'))
          .any(|pattern| matches_pattern(&pattern[1..], &temporary_href));

        if !should_exclude {
          let should_include = patterns
            .iter()
            .filter(|pattern| !pattern.starts_with('!'))
            .any(|pattern| matches_pattern(pattern, &temporary_href));

          if should_include {
            href = temporary_href;
          }
        }
      }
    }

    if let Some(image) = ENVIRONMENT
      .embed_images
      .as_deref()
      .and_then(|mode| embedded_image(&href, label, mode))
    {
      self.close_links(html);
      html.push_str(&image);

      return Some(());
    }

    if let Some(previous_link_count) = self.previous_link_count {
      if self.condense_links || self.condense_links_under_heading {
        *html = align_adjacent_links(html, previous_link_count);

        html.push_str(r#" <span class="gemini-fragment">|</span> "#);

        self.previous_link_count = Some(previous_link_count + 1);
      } else {
        html.push_str("<br />");
      }
    } else {
      html.push_str("<p>");

      self.previous_link_count = Some(0);
    }

    let _ = write!(
      html,
      r#"{}<a href="{}">{}</a>"#,
      GEMINI_FRAGMENT,
      sanitize_href(&href),
      render_text(label).trim(),
    );

    Some(())
  }
}

impl Renderer for ProxyRenderer<'_> {
  fn format_text(&mut self, text: &str, html: &mut String) {
    html.push_str(&render_text(text));
  }

  fn format_preformatted(&mut self, text: &str, html: &mut String) {
    let trimmed_text = text.strip_suffix('\n').unwrap_or(text);

    html.push_str(&html_escape(trimmed_text));
  }

  fn list_item_separator(&self) -> &'static str { "\n" }

  fn render_node(&mut self, node: &Node, html: &mut String) -> bool {
    if self.link_failed {
      return true;
    }

    let condensible_heading =
      ENVIRONMENT.condense_links_at_headings.contains(&node.to_gemtext());

    if condensible_heading {
      self.condense_links_under_heading = true;
    }

    if !matches!(node, Node::Link { .. }) {
      self.close_links(html);
    }

    match node {
      Node::Link { to, text } => {
        self.link_failed =
          self.render_link(to, text.as_deref(), html).is_none();

        true
      }

      Node::Heading { level, text } => {
        if !condensible_heading {
          self.condense_links_under_heading = false;
        }

        if self.title.is_empty() && *level == 1 {
          self.title = render_text(text).trim().to_string();
        }

        false
      }

      _ => false,
    }
  }

  fn finish(&mut self, html: &mut String) { self.close_links(html); }
}

pub fn from_gemini(
  content: &str,
  url: &Url,
  configuration: &crate::response::configuration::Configuration,
) -> Option<(String, String)> {
  let mut renderer = ProxyRenderer {
    url,
    configuration,
    title: String::new(),
    previous_link_count: None,
    condense_links: ENVIRONMENT
      .condense_links
      .contains(&url.path().to_string())
      || ENVIRONMENT.condense_links.contains(&"*".to_string()),
    condense_links_under_heading: false,
    link_failed: false,
  };
  let html = germ::convert::html::from_string(content, &mut renderer);

  if renderer.link_failed {
    return None;
  }

  Some((renderer.title, html))
}

#[cfg(test)]
mod tests {
  use {
    super::{
      embedded_image, from_gemini, link_from_host_href, render_text,
      resolve_link,
    },
    crate::response::configuration::Configuration,
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
  fn embeds_images_as_separate_paragraphs() {
    assert_eq!(
      embedded_image("gemini://example.org/a.png", "A", "1").as_deref(),
      Some(
        "<p><a href=\"gemini://example.org/a.png\">A</a> <i>Embedded \
         below</i></p><p><img src=\"gemini://example.org/a.png\" alt=\"A\" \
         /></p>"
      )
    );
    assert_eq!(embedded_image("gemini://example.org/a.txt", "A", "1"), None);
  }

  #[test]
  fn closes_a_final_link_paragraph() {
    let url = Url::parse("gemini://example.org/current").unwrap();
    let (_, html) =
      from_gemini("=> /next Next\n", &url, &Configuration::default()).unwrap();

    assert!(html.contains("<p>"));
    assert!(html.ends_with("</p>"));
  }

  #[test]
  fn preserves_formatting_in_standard_elements() {
    let url = Url::parse("gemini://example.org/current").unwrap();
    let (title, html) = from_gemini(
      "# *Title*\nText **bold**\n* One\n* Two\n> Quote\n```alt\n<&>\"'\n\n```",
      &url,
      &Configuration::default(),
    )
    .unwrap();

    assert_eq!(title, "<em>Title</em>");
    assert_eq!(
      html,
      concat!(
        "<h1><em>Title</em>\n</h1><p>Text <strong>bold</strong>\n</p>",
        "<ul><li>One\n</li>\n<li>Two\n</li></ul>",
        "<blockquote>Quote\n</blockquote><pre>&lt;&amp;&gt;&quot;'\n</pre>",
      ),
    );
  }

  #[test]
  fn closes_link_groups_at_whitespace_and_headings() {
    let url = Url::parse("gemini://example.org/current").unwrap();
    let (_, html) = from_gemini(
      "=> /one One\n\n=> /two Two\n# Heading\n=> /three Three",
      &url,
      &Configuration::default(),
    )
    .unwrap();

    assert!(html.contains("One</a></p><p>"));
    assert!(html.contains("Two</a></p><h1>Heading\n</h1><p>"));
    assert!(html.ends_with("Three</a></p>"));
  }

  #[test]
  fn rejects_unresolvable_links_without_returning_partial_html() {
    let url = Url::parse("gemini://example.org/current").unwrap();

    assert!(
      from_gemini(
        "Text\n=> //[invalid Broken\n# Heading",
        &url,
        &Configuration::default(),
      )
      .is_none()
    );
  }

  #[test]
  fn escapes_numbered_html_across_gemtext_nodes() {
    let gemtext = "1. <img src=x onerror=alert(1)>\n# 1. <img src=x \
                   onerror=alert(1)>\n=> /next 1. <img src=x \
                   onerror=alert(1)>\n* 1. <img src=x onerror=alert(1)>\n> 1. \
                   <img src=x onerror=alert(1)>\n";
    let url = Url::parse("gemini://example.org/current").unwrap();
    let (title, html) =
      from_gemini(gemtext, &url, &Configuration::default()).unwrap();

    assert!(!title.contains("<img"));
    assert!(!html.contains("<img"));
    assert!(html.contains("&lt;img"));
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
