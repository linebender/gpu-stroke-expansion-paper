# Fast-forward animation

The ["fast forward" video] for this paper is an animated sequence with voice-over.
The animations were written in Rust and screen recorded (using OBS).

It's easy enough to play the animation, just `cargo run`.
Release mode is helpful but probably not necessary.

Most of the animations are directly on top of Vello, using stroke expansion geometry from the `flatten` crate in this repo.
In addition, a couple of SVG assets (an equation typeset with typst and a figure from the paper, created with Inkscape), are pulled in and rendered using vello_svg.

["fast forward" video]: https://www.youtube.com/watch?v=gkeqny6zMDM
