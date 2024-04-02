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

use {germ::ast::Node, std::env::var, url::Url};

fn link_from_host_href(url: &Url, href: &str) -> Option<String> {
  Some(format!(
    "gemini://{}{}{}",
    url.domain()?,
    { if href.starts_with('/') { "" } else { "/" } },
    href
  ))
}

#[allow(clippy::too_many_lines)]
pub fn from_gemini(
  response: &germ::request::Response,
  url: &Url,
  is_proxy: bool,
) -> Option<(String, String)> {
  let ast_tree =
    germ::ast::Ast::from_string(response.content().clone().unwrap_or_default());
  let ast = ast_tree.inner();
  let mut html = String::new();
  let mut title = String::new();
  let safe = html_escape::encode_text;

  for node in ast {
    match node {
      Node::Text(text) => html.push_str(&format!("<p>{}</p>", safe(text))),
      Node::Link { to, text } => {
        let mut href = to.clone();
        let mut surface = false;

        if href.contains("://") && !href.starts_with("gemini://") {
          surface = true;
        } else if !href.starts_with("gemini://") && !href.starts_with('/') {
          href = format!("./{href}");
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
          if is_proxy
            || href
              .trim_start_matches("gemini://")
              .trim_end_matches('/')
              .split('/')
              .collect::<Vec<_>>()
              .first()
              .unwrap()
              != &url.host().unwrap().to_string().as_str()
          {
            href = format!("/proxy/{}", href.trim_start_matches("gemini://"));
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

        if var("EMBED_IMAGES").is_ok() {
          if let Some(extension) = std::path::Path::new(&href).extension() {
            if extension == "png"
              || extension == "jpg"
              || extension == "jpeg"
              || extension == "gif"
              || extension == "webp"
              || extension == "svg"
            {
              html.push_str(&format!(
                "<p><a href=\"{}\">{}</a> <i>Embedded below</i></p>\n<p><img \
                 src=\"{}\" alt=\"{}\" /></p>\n",
                href,
                safe(&text.clone().unwrap_or_default()),
                safe(&href),
                safe(&text.clone().unwrap_or_default())
              ));

              continue;
            }
          }
        }

        html.push_str(&format!(
          "<p><a href=\"{}\">{}</a></p>\n",
          href,
          safe(&text.clone().unwrap_or_default()),
        ));
      }
      Node::Heading { level, text } => {
        if title.is_empty() && *level == 1 {
          title = safe(&text.clone()).to_string();
        }

        html.push_str(&format!(
          "<{}>{}</{0}>",
          match level {
            1 => "h1",
            2 => "h2",
            3 => "h3",
            _ => "p",
          },
          safe(text),
        ));
      }
      Node::List(items) => html.push_str(&format!(
        "<ul>{}</ul>",
        items
          .iter()
          .map(|i| format!("<li>{i}</li>"))
          .collect::<Vec<String>>()
          .join("\n")
      )),
      Node::Blockquote(text) => {
        html.push_str(&format!("<blockquote>{}</blockquote>", safe(text)));
      }
      Node::PreformattedText { text, .. } => {
        html.push_str(&format!("<pre>{}</pre>", safe(text)));
      }
      Node::Whitespace => {}
    }
  }

  Some((title, html))
}
