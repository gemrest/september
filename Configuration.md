# Configuration

The configuration for September is managed entirely through environment variables.

## `PORT`

Bind September to a custom port.

Generally, you shouldn't touch this option if you are deploying using Docker.

If no `PORT` is provided or the `PORT` could not be parsed appropriately as an
unsigned 16-bit integer, `PORT` will default to `80`.

```dotenv
PORT=1337
```

## `ROOT`

Root Gemini capsule to proxy when not visiting a "/proxy" route

If no `ROOT` is provided, `ROOT` will default to `"gemini://fuwn.me"`.

```dotenv
ROOT=gemini://fuwn.me
```

## `CSS_EXTERNAL`

A comma-separated list of external CSS files to apply to the HTML response

If no `CSS_EXTERNAL` value is provided, a default stylesheet of
[LaTeX.css](https://latex.vercel.app/) and the styles within
[`default.css`](./default.css) will be applied.

```dotenv
CSS_EXTERNAL=https://cdnjs.cloudflare.com/ajax/libs/mini.css/3.0.1/mini-default.min.css
```

## `KEEP_GEMINI`

A comma-separated list of Gemini URL fragments to keep as is when proxying.

Wildcards are supported using the `*` character, and exceptions can be made
using the `!` character

```dotenv
# These rules ensure that all Gemini URLs will be left untouched in the proxied
# HTML response except for URLs under the "fuwn.me" domain
KEEP_GEMINI=!*fuwn.me/*,gemini://*
```

## `HEAD`

Insert any string at the end of the HTML `<head>`

```dotenv
HEAD=<script>/* September */</script><style>/* September */</style>
```

## `PROXY_BY_DEFAULT`

Control whether all Gemini URLs are proxied.

Similar to `KEEP_GEMINI_EXACT` and `KEEP_GEMINI_DOMAIN`, but global

This configuration value defaults to `true`.

```dotenv
PROXY_BY_DEFAULT=false
```

## `FAVICON_EXTERNAL`

An external favicon file to apply to the HTML response

```dotenv
FAVICON_EXTERNAL=https://example.com/favicon.ico
```

## `PLAIN_TEXT_ROUTE`

A comma-separated list of paths to treat as plain text routes

These patterns do not support regular expressions, but do support the use of `*`
as a wildcard.

```dotenv
PLAIN_TEXT_ROUTE=/robots.txt,/license.txt,*.xml
```

## `MATHJAX`

Enable MathJax support for rendering LaTeX within `$` and `$$` delimiters.

This configuration value defaults to `false`.

```dotenv
MATHJAX=true
```

## `HEADER`

Adds a large text header to the top of a proxy page

Only available in styled routes

```dotenv
HEADER="This string will show up at the top of my proxied capsule."
```

## `EMBED_IMAGES`

Embed images in the HTML response if a link to an image is found.

A value of `1` will enable this feature, while keeping a link to the image.

Any non-empty value other than `1` will enable this feature, while removing the link to the image.

```dotenv
EMBED_IMAGES=2
```

## `CONDENSE_LINKS`

Condense adjacent links to a single line

A value of `*` will condense all adjacent links to a single line.

A comma-separated list of paths will condense adjacent links to a single line only on those paths.

### Example

```plaintext
<!-- Not condensed -->

<p><a href="/">Link</a></p>
<p><a href="/">Link</a></p>
<p><a href="/">Link</a></p>

<!-- Condensed -->
<p><a href="/">Link</a> | <a href="/">Link</a> | <a href="/">Link</a></p>
```

## `PRIMARY_COLOUR`

Set the primary colour of elements in the default stylesheet. This field
controls the colour of items such as links and highlights.

Popular choices are `var(--base0D)` for a blue, or `var(--base09)` for an
amber colour.

### Examples

```plaintext
PRIMARY_COLOUR=var(--base09)
PRIMARY_COLOUR=red
PRIMARY_COLOUR=#ff0000
```

## `HTTP09`

Enable a separate HTTP/0.9 TCP server alongside the main HTTP server

HTTP/0.9 is the simplest version of HTTP. Requests are a bare `GET /path` line,
and responses are the raw body with no status line or headers. The server returns
the proxied Gemini content directly (text/gemini for text, raw bytes for images).

This configuration value defaults to `false`.

```dotenv
HTTP09=true
```

## `HTTP09_PORT`

Bind the HTTP/0.9 server to a custom port

If no `HTTP09_PORT` is provided or it could not be parsed appropriately as an
unsigned 16-bit integer, `HTTP09_PORT` will default to `90`.

```dotenv
HTTP09_PORT=9009
```

### Testing

```sh
echo "GET /" | nc localhost 9009
curl --http0.9 http://localhost:9009/
```

## `CONDENSE_LINKS_AT_HEADING`

This configuration option is similar to `CONDENSE_LINKS`, but only condenses
links found under specific headings.

For instance, I condense the few links I have on my index page under the
"# Fuwn[.me]" heading, and I condense my quick links/navigation panel under the
"## Quick Links" heading.

This way, I don't accidentally condense my entire sitemap, which could be
hundreds of links long, but I do condense my quick links on every page.

```dotenv
CONDENSE_LINKS_AT_HEADINGS="## Quick Links,# Fuwn[.me]"
```
