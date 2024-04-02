# September

[![github.com](https://github.com/gemrest/september/actions/workflows/check.yaml/badge.svg?branch=main)](https://github.com/gemrest/september/actions/workflows/check.yaml)

A simple and efficient Gemini-to-HTTP proxy written in Rust.

## Usage

A production deployment of September can be found at https://fuwn.me, with the root capsule set as [gemini://fuwn.me](gemini://fuwn.me).

You can try proxying any external capsule through the /proxy route: e.g., https://fuwn.me/proxy/geminiprotocol.net/

### Docker

```shell
docker run -d [ -e ROOT="gemini://fuwn.me" ] [ -e PORT="8080"] [ -e CSS_EXTERNAL="https://example.com/style.css"] fuwn/september:latest
```

### Docker Compose

Edit the `docker-compose.yaml` file to your liking, and then

```shell
docker-compose up -d
```

### Executable

```shell
[ ROOT="gemini://fuwn.me" ] [ PORT="8080"] [ CSS_EXTERNAL="https://example.com/style.css"] ./september
```

or use a `.env` file

```dotenv
# .env

ROOT=gemini://fuwn.me
PORT=8080
CSS_EXTERNAL=https://example.com/style.css
HEAD=<script>/* september */</script>
```

and then

```shell
./september
```

## Configuration

All configuration options with examples can be found in the [Configuration.md](./Configuration.md) file.

## Styling

Want to give your website a shiny new look? Try using one of sources
to find a stylish and **minimal** (!!) CSS theme/ framework!

- [dohliam/dropin-minimal-css](https://github.com/dohliam/dropin-minimal-css): Drop-in switcher for previewing minimal CSS frameworks
- [dbohdan/classless-css](https://github.com/dbohdan/classless-css): A list of classless CSS themes/frameworks with screenshots

## Capsules using September

[Add yours!](https://github.com/gemrest/september/edit/main/README.md)

- <https://fuwn.me/>
- <https://gem.rest/>

## License

This project is licensed with the
[GNU General Public License v3.0](https://github.com/gemrest/september/blob/main/LICENSE).
