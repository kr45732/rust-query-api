FROM rust:1.68.0

WORKDIR /app
COPY . .

RUN cargo build --release

CMD ./target/release/query_api