use {germ::ast::Node, std::env::var, std::fmt::Write, url::Url};

fn link_from_host_href(url: &Url, href: &str) -> Option<String> {
  Some(format!(
    "gemini://{}{}{}",
    url.domain()?,
    { if href.starts_with('/') { "" } else { "/" } },
    href
  ))
}

fn safe(text: &str) -> String {
  comrak::markdown_to_html(text, &comrak::ComrakOptions::default())
    .replace("<p>", "")
    .replace("</p>", "")
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
  let condense_links = {
    let links = var("CONDENSE_LINKS").map_or_else(
      |_| vec![],
      |condense_links| {
        condense_links
          .split(',')
          .map(std::string::ToString::to_string)
          .collect()
      },
    );

    links.contains(&url.path().to_string()) || links.contains(&"*".to_string())
  };
  let mut in_condense_links_flag_trap = true;
  let condensible_headings_value =
    var("CONDENSE_LINKS_AT_HEADINGS").unwrap_or_default();
  let condensible_headings =
    condensible_headings_value.split(',').collect::<Vec<_>>();

  for node in ast {
    if condensible_headings.contains(&node.to_gemtext().as_str()) {
      in_condense_links_flag_trap = true;
    }

    let align_adjacent_links = |html: &str| {
      if previous_link_count > 0 {
        html
          .chars()
          .rev()
          .collect::<String>()
          .replacen(&GEMINI_FRAGMENT.chars().rev().collect::<String>(), "", 1)
          .chars()
          .rev()
          .collect::<String>()
      } else {
        html.to_string()
      }
    };

    if previous_link
      && (!matches!(node, Node::Link { .. })
        || (!condense_links && !in_condense_links_flag_trap))
    {
      if let Some(next) = ast.iter().skip_while(|n| n != &node).nth(1) {
        if matches!(next, Node::Link { .. }) || previous_link {
          html.push_str("<br />");
        } else {
          html.push_str("</p>");
        }
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
        let _ = write!(&mut html, "<p>{}</p>", safe(text));
      }
      Node::Link { to, text } => {
        let mut href = to.to_string();
        let mut surface = false;

        if href.starts_with("./") || href.starts_with("../") {
          if let Ok(url) = url.join(&href) {
            href = url.to_string();
          }
        }

        if href.contains("://") && !href.starts_with("gemini://") {
          surface = true;
        } else if !href.contains("://") && href.contains(':') {
          href = href.to_string();
        } else if !href.starts_with("gemini://") && !href.starts_with('/') {
          href = format!(
            "{}/{}",
            url.domain().unwrap(),
            if url.path().ends_with('/') {
              format!("{}{}", url.path(), href)
            } else {
              format!("{}/{}", url.path(), href)
            }
          )
          .replace("//", "/");
          href = format!("gemini://{href}");
        } else if href.starts_with('/') || !href.contains("://") {
          href = link_from_host_href(url, &href)?;
        }

        if var("PROXY_BY_DEFAULT")
          .unwrap_or_else(|_| "true".to_string())
          .to_lowercase()
          == "true"
          && href.contains("gemini://")
          && !surface
        {
          if (configuration.is_proxy())
            || configuration.is_no_css()
            || href
              .trim_start_matches("gemini://")
              .trim_end_matches('/')
              .split('/')
              .collect::<Vec<_>>()
              .first()
              .unwrap()
              != &url.host().unwrap().to_string().as_str()
          {
            href = format!(
              "/{}/{}",
              if configuration.is_no_css() { "nocss" } else { "proxy" },
              href.trim_start_matches("gemini://")
            );
          } else {
            href = href.trim_start_matches("gemini://").replacen(
              &if let Some(host) = url.host() {
                host.to_string()
              } else {
                return None;
              },
              "",
              1,
            );
          }
        }

        if let Ok(keeps) = var("KEEP_GEMINI_EXACT") {
          let mut keeps = keeps.split(',');

          if (href.starts_with('/') || !href.contains("://")) && !surface {
            let temporary_href = link_from_host_href(url, &href)?;

            if keeps.any(|k| k == &*temporary_href) {
              href = temporary_href;
            }
          }
        }

        if let Ok(keeps) = var("KEEP_GEMINI_DOMAIN") {
          let host = if let Some(host) = url.host() {
            host.to_string()
          } else {
            return None;
          };

          if (href.starts_with('/')
            || !href.contains("://") && keeps.split(',').any(|k| k == &*host))
            && !surface
          {
            href = link_from_host_href(url, &href)?;
          }
        }

        if let Ok(embed_images) = var("EMBED_IMAGES") {
          if let Some(extension) = std::path::Path::new(&href).extension() {
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
                  href,
                  safe(text.as_ref().unwrap_or(to)),
                );
              }

              let _ = writeln!(
                &mut html,
                "<p><img src=\"{}\" alt=\"{}\" /></p>",
                safe(&href),
                safe(text.as_ref().unwrap_or(to)),
              );

              continue;
            }
          }
        }

        previous_link = true;

        let _ = write!(
          &mut html,
          r#"{}<a href="{}">{}</a>"#,
          if condense_links { "" } else { GEMINI_FRAGMENT },
          href,
          safe(text.as_ref().unwrap_or(to)).trim(),
        );
      }
      Node::Heading { level, text } => {
        if !condensible_headings.contains(&node.to_gemtext().as_str()) {
          in_condense_links_flag_trap = false;
        }

        if title.is_empty() && *level == 1 {
          title = safe(text).to_string();
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
          safe(text),
        );
      }
      Node::List(items) => {
        let _ = write!(
          &mut html,
          "<ul>{}</ul>",
          items
            .iter()
            .map(|i| format!("<li>{}</li>", safe(i)))
            .collect::<Vec<String>>()
            .join("\n")
        );
      }
      Node::Blockquote(text) => {
        let _ = write!(&mut html, "<blockquote>{}</blockquote>", safe(text));
      }
      Node::PreformattedText { text, .. } => {
        let mut new_text = text.to_string();

        new_text.pop();

        let _ = write!(&mut html, "<pre>{new_text}</pre>");
      }
      Node::Whitespace => {}
    }
  }

  Some((title, html))
}
