FROM rust:1.77.2-alpine3.19 as builder

ARG UID=10001
ARG GID=10001
ARG UNAME=e2l
ARG APP_NAME=e2l-parser

RUN apk update && apk add --no-cache libc-dev make protobuf-dev openssl-dev cmake && \
    addgroup --gid ${GID} ${UNAME} && \
    adduser --home /home/${UNAME} --uid ${UID} -G ${UNAME} --disabled-password ${UNAME} && \
    mkdir /home/${UNAME}/${APP_NAME} && \
    chown -R ${UNAME}:${UNAME} /home/${UNAME} 

WORKDIR /home/${UNAME}/${APP_NAME}
USER ${UNAME}:${UNAME}

COPY .cargo/ .cargo
COPY protos/ protos/
COPY src/ src/
COPY build.rs build.rs
COPY Cargo.* ./

RUN RUSTFLAGS='-C target-feature=-crt-static' cargo build --release

FROM alpine:3.19.1

ARG UID=10001
ARG GID=10001
ARG UNAME=e2l
ARG APP_NAME=e2l-parser

COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

RUN apk update && apk add --no-cache libc-dev protobuf-dev openssl-dev && \
    mkdir -p /home/${UNAME}/${APP_NAME} && \
    chown -R ${UNAME}:${UNAME} /home/${UNAME}
WORKDIR /home/${UNAME}/${APP_NAME}
USER ${UNAME}:${UNAME}

COPY --from=builder /home/e2l/e2l-parser/target/release/e2l-parser ./

EXPOSE 1680/udp

CMD ["./e2l-parser"]
