# 🖌️ Van Gogh Image Viewer

Qt6-based image viewer and manipulation tool. In dire need of modernization. And cool features.

## Setting up
To build this project, you'll need some tools and dependencies on your system. 

First off is installing the build tools. Perhapse not so surprisingly, you'll need Rust, [which you can install using the instructions here](https://rustup.rs/). Furthermore, we'll need Clang and CMake.

If you're on **Fedora**, run the following commands:

```shell
sudo dnf install clang cmake
```

On **Ubuntu** or **Debian**, run:

```shell
sudo apt install clang cmake
```

With that out of the way, we need to install some dependencies namely Qt and libpng, which this project relies upon.

On **Fedora**:

```shell
sudo dnf install qt6-qttools-deve
```

On **Ubuntu** or **Debian**:

```shell
sudo apt install qt6-tools-dev qt6-tools-dev-tools qt6-l10n-tools libgl1-mesa-dev qt6-wayland qt6-wayland-dev qt6-wayland-dev-tools
```

## Building and running
If you want to build run Van Gogh from the command line, you can run the following commands:

```shell
cd build
cmake ..
cmake --build .
cd ..
build/Van_Gogh <your_image_here>
```
