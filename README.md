# rust-gpu vulkano example
An example project on how to integrate [rust-gpu](https://github.com/EmbarkStudios/rust-gpu) with [vulkano](https://github.com/vulkano-rs/vulkano). It based on vulkano's `image` example but the glsl shaders are replaced with rust-gpu shaders.

If you want to look at the changes made to the image example, have a look at the history, specifically these two commits:
* [fb07112c](https://github.com/Firestar99/rust-gpu-vulkano-example/commit/fb07112c249127476838ad2798a55558a83fcf45) move image shaders into separate mod, add ENTRY_POINT constants
* [37e5eece](https://github.com/Firestar99/rust-gpu-vulkano-example/commit/37e5eece229e630fd2aa262cab65680a44538ae4) integrate rust-gpu
* [ea077ef7](https://github.com/Firestar99/rust-gpu-vulkano-example/commit/ea077ef75f8f02e7b36b07aecbd64aa88209a251) utilize shader! macro's new root_path_env property from [my PR](https://github.com/vulkano-rs/vulkano/pull/2180), by using OUT_DIR directly
* [d30f310b](https://github.com/Firestar99/rust-gpu-vulkano-example/commit/d30f310bdb92b59e36db22d60966d9abba5d79ab) instead of using OUT_DIR, specify a custom env variable SHADER_OUT_DIR in the build script
