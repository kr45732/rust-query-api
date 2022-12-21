FROM rust:1.66.0

WORKDIR /app
COPY . .

RUN cargo build --release

CMD ./target/release/query_api