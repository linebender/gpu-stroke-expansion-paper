# beztoy

A toy for experimenting with Bézier curves, intended to become a testbed for Euler spiral based stroke expansion.

To build the WASM artifacts, run the following from the `beztoy/` directory:
```shell
$ make beztoy
```

This will compile the WASM and JS bindings and copy them over to the `/docs` directory. To interact
the newly built demo, start a local web server and navigate your browser to `/docs/index.html`.
