FROM docker.io/rust:slim-bullseye as builder
WORKDIR /usr/src/scraper-api
RUN apt update && apt install -y libssl-dev
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

ENV DEPLOYMENT_URL 0.0.0.0:8000
ENV OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu
ENV OPENSSL_INCLUDE_DIR=/usr/include
COPY Cargo.toml .
COPY ./src/ ./src
RUN cargo build --release

FROM docker.io/debian:bullseye-slim
RUN apt update && apt install -y ca-certificates
WORKDIR /usr/local/bin/
COPY --from=builder /usr/src/scraper-api/target/release/scraper-api .

CMD ["scraper-api"]
EXPOSE 8000
