FROM rust:1.31

WORKDIR /app
COPY . .

RUN ls /app
RUN cargo build --release

CMD ./target/release/query_api