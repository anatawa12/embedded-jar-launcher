# embedded-jar-launcher
[![a12 maintenance: Archived](https://anatawa12.com/short.php?q=a12-archived-svg)](https://anatawa12.com/short.php?q=a12-archived-doc)

This project was a jar launcher which
- is written in rust
- supports linux, windows, and macOS
- supports cross-building with [cross]
- does not embed JRE
- searches local JRE
- embeds fat jar file
- was for [lightweight-protobuf]

However, I found this is not required for [lightweight-protobuf]
but I don't want to it lost forever so here's this project. 
I'll never maintain currently.

## Requirements
### for building
- java
- rust compiler
- [cross] (for cross building)

### for running
- java in JAVA_HOME environment variable or on PATH

## Building

Just run ``./gradlew build``.
There's executable file at `native/target/release/embedded-jar-launcher(.exe)`.

For cross-build, just run ``./gradlew build -Pcross``.
You may need to comment `-darwin` configurations because 
they're only supported on darwin/macOS.
There are executable files at `native/target/<target-triple>/release/embedded-jar-launcher(.exe)`.

## Project structure

- [`buildSrc`](buildSrc)

  contains a plugin to build cargo project.
- [`native`](native)

  contains a rust cargo project to search and launch java.

- [`src/main/lib`](src/main/lib)

  contains a java-side launcher.

- [`src/main/java`](src/main/java)

  contains a example java code.

- [`build1.gradle.kts`](build1.gradle.kts)

[lightweight-protobuf]: https://github.com/anatawa12/lightweight-protobuf
[cross]: https://github.com/rust-embedded/cross
