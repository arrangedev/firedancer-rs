
FROM --platform=linux/amd64 ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive
ENV RUST_BACKTRACE=1

RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    curl \
    git \
    clang \
    libclang-dev \
    llvm-dev \
    make \
    cmake \
    libzstd-dev \
    liblz4-dev \
    libssl-dev \
    libc6-dev \
    linux-libc-dev \
    gcc-multilib \
    g++-multilib \
    && rm -rf /var/lib/apt/lists/*

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

RUN rustup default stable && \
    rustup component add clippy rustfmt && \
    rustup install nightly && \
    rustup component add rustfmt --toolchain nightly && \
    rustup target add x86_64-unknown-linux-gnu

WORKDIR /workspace

COPY . .

RUN cargo fetch --target x86_64-unknown-linux-gnu

RUN echo "================================" && \
    echo "OS: $(cat /etc/os-release | grep PRETTY_NAME)" && \
    echo "architecture: $(uname -m)" && \
    echo "rustc version: $(rustc --version)" && \
    echo "available targets: $(rustup target list --installed)" && \
    echo "cargo version: $(cargo --version)" && \
    echo "================================"

CMD ["/bin/bash"]
