## On macOS or Linux
```shell
# vello test scenes:
cargo run --release -- -s flatten vello-test-scenes -m mmark,longpathdas

# SVGs:
$ cargo run --release -- -s flatten svgs ./path/to/svgs
```

## On Android using [xbuild](https://github.com/rust-mobile/xbuild)

```shell
# First copy the SVGS over this directory:
$ adb push path/to/svgs/* /data/local/tmp/svgs
$ x run --device <DEVICE_ID> --release
```
