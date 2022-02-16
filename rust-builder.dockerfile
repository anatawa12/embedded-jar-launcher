FROM rust:1-bullseye

RUN rustup target add x86_64-apple-darwin
RUN apt update && apt install -y clang && rm -rf /var/lib/apt/lists/*
WORKDIR /project
