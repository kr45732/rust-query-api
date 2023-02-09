FROM rust:1.67.1

WORKDIR /app
COPY . .

RUN cargo build --release

CMD ./target/release/query_api