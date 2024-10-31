FROM rust:latest as builder

ARG GITHUB_PAT
ENV GH_PAT=$GITHUB_PAT

RUN apt-get update -y && apt-get install -y libfontconfig1-dev libfreetype6-dev libexpat1-dev
WORKDIR /usr/src/rowifi
COPY . .
RUN git config --global url."https://${GH_PAT}:@github.com".insteadOf "https://github.com"
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true
RUN cargo build --release

FROM debian:bookworm
RUN apt-get update && apt-get install -y libfontconfig1-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/rowifi/target/release/rowifi /usr/local/bin/rowifi
CMD ["rowifi"]