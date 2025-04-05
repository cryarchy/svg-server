# svg-server

A simple static SVG file serving command-line tool.

USAGE:

```
svg-server [OPTIONS] [path]

OPTIONS:
    -b, --bind <address> Specify bind address [default: 127.0.0.1]
    -p, --port <port> Specify port to listen on [default: 5000]
    -i, --index <index> Specify route to redirect / to [default: /home]

ARGS:
    <path> Path to a directory containing the SVG files to be served [default: .]
```
