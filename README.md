# GPU-friendly stroke expansion paper

This repo is for open collaboration on a research paper on GPU path rendering. It is intended to cover the work on [Vello], particularly the GPU stroke expansion algorithms based on Euler spirals.

The repo contains the TeX sources, bibliography, and figures. As we work, it will also contain scripts to run benchmarks, test data, and related support. The renderer itself will remain in its own repository, however. Any code is covered under the [Apache 2 license](LICENSE). The text is covered under the [CC-BY 4.0](https://creativecommons.org/licenses/by/4.0/).

This paper will be presented at [High Performance Graphics 2024] in Denver, Colorado, July 26, 2024.

The `flatten` subdirectory contains a command-line tool used to make a number of figures and some measurements. At its heart is a CPU implementation of the algorithm described in the paper. It also exports a library, intended more as a research prototype than for production.

The `anims` subdirectory contains animations for the "fast forward" video, implemented on top of [Vello], and using the `flatten` library to generate stroked paths for visualization.

Suggestions and comments are welcome through the repo's issue tracker, though we expect development to stop as soon as the final version of the paper is sent. A [preprint] is available on arXiv.

Contributions are welcome under the same terms as other [Linebender] projects. The [Rust code of conduct] applies.

[Vello]: https://github.com/linebender/vello
[Linebender]: https://linebender.org/
[Rust code of conduct]: https://www.rust-lang.org/policies/code-of-conduct
[High Performance Graphics 2024]: https://www.highperformancegraphics.org/2024/index.html
[preprint]: https://arxiv.org/abs/2405.00127
[Vello]: https://github.com/linebender/vello