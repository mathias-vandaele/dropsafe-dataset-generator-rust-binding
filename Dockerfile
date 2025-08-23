FROM rust:1.88.0-bookworm AS builder

WORKDIR /usr/src/app
COPY Cargo.toml Cargo.lock ./
COPY ./src ./src

RUN apt-get update && \
    apt-get -y --no-install-recommends --no-install-suggests install \
        ca-certificates \
        cmake \
        g++ \
        gcc \
        git \
        libboost1.81-all-dev \
        libbz2-dev \
        liblua5.4-dev \
        libtbb-dev \
        libxml2-dev \
        libzip-dev \
        lua5.4 \
        make \
        pkg-config \
        libfmt-dev

RUN ls -la /usr/lib/x86_64-linux-gnu/libboost_thread*

RUN cargo build --release -vv

FROM debian:bookworm-slim

WORKDIR /usr/src/app
COPY --from=builder /usr/src/app/target/release/dropsafe-dataset-generator-rust-binding ./

RUN apt-get update && \
    apt-get install -y --no-install-recommends --no-install-suggests \
        expat \
        libboost-date-time1.81.0 \
        libboost-iostreams1.81.0 \
        libboost-program-options1.81.0 \
        libboost-thread1.81.0 \
        liblua5.4-0 \
        libtbb12 && \
        rm -rf /var/lib/apt/lists/* && \
        ldconfig /usr/local/lib

CMD ["./dropsafe-dataset-generator-rust-binding"]