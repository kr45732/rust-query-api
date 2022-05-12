ARG RUST_TARGET="x86_64-unknown-linux-musl"
ARG MUSL_TARGET="x86_64-linux-musl"

FROM alpine:latest as build
ARG RUST_TARGET
ARG MUSL_TARGET

RUN apk upgrade && \
    apk add curl gcc musl-dev openssl-dev && \
    curl -sSf https://sh.rustup.rs | sh -s -- --profile minimal --default-toolchain nightly --component rust-src -y

# Build-std
RUN source $HOME/.cargo/env && \
    mkdir -p /app/.cargo && \
    if [ "$RUST_TARGET" != $(rustup target list --installed) ]; then \
        rustup target add $RUST_TARGET && \
        curl -L "https://musl.cc/$MUSL_TARGET-cross.tgz" -o /toolchain.tgz && \
        tar xf toolchain.tgz && \
        ln -s "/$MUSL_TARGET-cross/bin/$MUSL_TARGET-gcc" "/usr/bin/$MUSL_TARGET-gcc" && \
        ln -s "/$MUSL_TARGET-cross/bin/$MUSL_TARGET-ld" "/usr/bin/$MUSL_TARGET-ld" && \
        ln -s "/$MUSL_TARGET-cross/bin/$MUSL_TARGET-strip" "/usr/bin/actual-strip" && \
        GCC_VERSION=$($MUSL_TARGET-gcc --version | grep gcc | awk '{print $3}') && \
        echo -e "\
[build]\n\
rustflags = [\"-L\", \"native=/$MUSL_TARGET-cross/$MUSL_TARGET/lib\", \"-L\", \"native=/$MUSL_TARGET-cross/lib/gcc/$MUSL_TARGET/$GCC_VERSION/\", \"-l\", \"static=gcc\", \"-Z\", \"gcc-ld=lld\"]\n\
[target.$RUST_TARGET]\n\
linker = \"$MUSL_TARGET-gcc\"\n\
[unstable]\n\
build-std = [\"std\", \"panic_abort\"]\n\
" > /app/.cargo/config; \
    else \
        echo "skipping toolchain as we are native" && \
        echo -e "\
[build]\n\
rustflags = [\"-L\", \"native=/usr/lib\"]\n\
[unstable]\n\
build-std = [\"std\", \"panic_abort\"]\n\
" > /app/.cargo/config && \
        ln -s /usr/bin/strip /usr/bin/actual-strip; \
    fi

WORKDIR /app

COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# Build deps
RUN mkdir src/
RUN echo 'fn main() {}' > ./src/main.rs
RUN source $HOME/.cargo/env && \
      cargo build --release \
          --target="$RUST_TARGET"

# Actually build the binary
RUN rm -f target/$RUST_TARGET/release/deps/query_api*
COPY ./src ./src

RUN source $HOME/.cargo/env && \
      cargo build --release \
        --target="$RUST_TARGET" && \
    cp target/$RUST_TARGET/release/query_api /query_api && \
    actual-strip /query_api

FROM scratch

COPY --from=build /query_api /query_api

CMD ["./query_api"]