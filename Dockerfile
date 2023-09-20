FROM rust:1.72.1

WORKDIR /app
COPY . .

RUN cargo build --release

CMD ./target/release/query_api