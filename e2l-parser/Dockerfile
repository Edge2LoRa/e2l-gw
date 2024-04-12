FROM rust:alpine as base

FROM base as builder

RUN apk update && apk add --no-cache libc-dev make protobuf-dev openssl-dev cmake

# Create appuser
ENV USER=e2l
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"

WORKDIR /e2l-gw-parser

COPY ./ .

RUN RUSTFLAGS='-C target-feature=-crt-static' cargo build --release


FROM base


COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group


WORKDIR /e2l-gw-parser

COPY --from=builder /e2l-gw-parser/target/release/e2l-gw-parser ./

USER e2l:e2l
EXPOSE 1680/udp

CMD ["./e2l-gw-parser"]
