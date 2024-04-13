FROM rust:alpine as base

FROM base as builder

ARG UID=10001
ARG GID=10001
ARG UNAME=e2l

RUN apk update && apk add --no-cache libc-dev make protobuf-dev openssl-dev cmake && \
    addgroup --gid ${GID} ${UNAME} && \
    adduser --home /home/e2l --uid ${UID} -G ${UNAME} --disabled-password e2l && \
    mkdir /home/e2l/e2l-parser && \
    chown -R ${UNAME}:${UNAME} /home/e2l 

WORKDIR /home/e2l/e2l-parser

COPY .cargo/ .cargo
COPY protos/ protos/
COPY src/ src/
COPY build.rs build.rs
COPY Cargo.* ./

RUN RUSTFLAGS='-C target-feature=-crt-static' cargo build --release

FROM base

COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /home/e2l/e2l-parser

COPY --from=builder /home/e2l/e2l-parser/target/release/e2l-parser ./

USER e2l:e2l
EXPOSE 1680/udp

CMD ["./e2l-parser"]
