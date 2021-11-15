FROM rust:1.56.1

WORKDIR /app
COPY . .

RUN cargo build --release

CMD ./target/release/query_api