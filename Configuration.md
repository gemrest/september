# Configuration

Configuration for September is done entirely through environment variables.

## `PORT`

Bind September to a custom port

Generally, you shouldn't touch this option if you are deploying using Docker.

If no `PORT` is provided or the `PORT` could not be properly parsed as an
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

## `KEEP_GEMINI_EXACT`

A comma-separated list of Gemini URIs to keep as is when proxying.

```dotenv
# These two URIs will be kept pointing to their original Gemini URIs when
# proxied instead of being replaced with their proxied equivalents.
KEEP_GEMINI_EXACT=gemini://fuwn.me/something,gemini://fuwn.me/another
```

## `HEAD`

Insert any string at the end of the HTMl `<head>`

```dotenv
HEAD=<script>/* September */</script><style>/* September */</style>
```

## `KEEP_GEMINI_DOMAIN`

Similar to `KEEP_GEMINI_EXACT`, except matches based on entire domain or domains
instead of exact URIs

```dotenv
KEEP_GEMINI_DOMAIN=fuwn.me,example.com
```

## `PROXY_BY_DEFAULT`

Control whether or not all Gemini URLs will be proxied

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

```dotenv
PLAIN_TEXT_ROUTE=/robots.txt,/license.txt
```

## `MATHJAX`

Enable MathJax support for rendering LaTeX within `$` and `$$` delimiters

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

## `EMBED_IMAGES

Embed images in the HTML response if a link to an image is found

Any non-empty value will enable this feature.

```dotenv
EMBED_IMAGES=true
```