FROM rust:1.31

WORKDIR /rust-query-api
COPY . .

RUN cargo run --release