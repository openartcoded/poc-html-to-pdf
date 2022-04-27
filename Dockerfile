FROM rust:1.60.0 as builder
WORKDIR /app
RUN cargo new poc_htmltopdf
WORKDIR /app/poc_htmltopdf

COPY ./Cargo.toml ./Cargo.lock ./

RUN cargo build --release
RUN rm -rf ./src

COPY ./src/ ./src

RUN rm ./target/release/deps/poc_htmltopdf*

RUN cargo build --release

FROM debian:11-slim
ENV RUST_LOG=info

RUN apt-get update && apt-get install -y \
      chromium \
      --no-install-recommends \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY ./pages ./pages

COPY --from=builder  /app/poc_htmltopdf/target/release/poc_htmltopdf .
CMD [ "./poc_htmltopdf" ]
