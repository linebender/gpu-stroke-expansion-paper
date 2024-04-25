## On macOS or Linux
```shell
# vello test scenes:
cargo run --release -- -s flatten vello-test-scenes -m mmark,longpathdas

# SVGs:
cargo run --release -- -s flatten svgs ./path/to/svgs
```

## On Android using [xbuild](https://github.com/rust-mobile/xbuild)

```shell
x run --device <DEVICE_ID> --release
```
