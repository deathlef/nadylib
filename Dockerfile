# Rust syntax target, either x86_64-unknown-linux-musl, aarch64-unknown-linux-musl, arm-unknown-linux-musleabi etc.
ARG RUST_TARGET="x86_64-unknown-linux-musl"
# Musl target, either x86_64-linux-musl, aarch64-linux-musl, arm-linux-musleabi, etc.
ARG MUSL_TARGET="x86_64-linux-musl"
# This ONLY works with defaults which is rather annoying
# but better than nothing
# Uses docker's own naming for architectures
# e.g. x86_64 -> amd64, aarch64 -> arm64v8, arm -> arm32v7
ARG FINAL_TARGET="amd64"

FROM docker.io/library/alpine:edge AS builder
ARG MUSL_TARGET
ARG RUST_TARGET

RUN apk upgrade && \
    apk add curl gcc musl-dev && \
    curl -sSf https://sh.rustup.rs | sh -s -- --profile minimal --default-toolchain nightly -y

RUN source $HOME/.cargo/env && \
    if [ "$RUST_TARGET" != $(rustup target list --installed) ]; then \
        rustup target add $RUST_TARGET && \
        curl -L "https://musl.cc/$MUSL_TARGET-cross.tgz" -o /toolchain.tgz && \
        tar xf toolchain.tgz && \
        ln -s "/$MUSL_TARGET-cross/bin/$MUSL_TARGET-gcc" "/usr/bin/$MUSL_TARGET-gcc" && \
        ln -s "/$MUSL_TARGET-cross/bin/$MUSL_TARGET-ld" "/usr/bin/$MUSL_TARGET-ld" && \
        ln -s "/$MUSL_TARGET-cross/bin/$MUSL_TARGET-strip" "/usr/bin/actual-strip"; \
    else \
        echo "skipping toolchain as we are native" && \
        ln -s /usr/bin/strip /usr/bin/actual-strip; \
    fi

WORKDIR /build

COPY . .

RUN source $HOME/.cargo/env && \
    cargo build --release \
        --target="$RUST_TARGET" && \
    cp target/$RUST_TARGET/release/aochatproxy /aochatproxy && \
    actual-strip /aochatproxy

FROM docker.io/${FINAL_TARGET}/alpine:edge

WORKDIR /aochatproxy

COPY --from=builder /aochatproxy /usr/bin/aochatproxy

CMD /usr/bin/aochatproxy
