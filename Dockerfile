FROM rust:1.59.0

WORKDIR /app
COPY . .

RUN cargo build --release

CMD ./target/release/query_api