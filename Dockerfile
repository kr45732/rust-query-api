FROM rust:1.31

WORKDIR /app
COPY . .

RUN cargo build --release

CMD ./target/release/query_api