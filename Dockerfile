FROM docker.io/library/rust:1-slim as build
# RUN rustup target add x86_64-unknown-linux-musl
WORKDIR /usr/src/pervybot-rs
COPY . .
RUN cargo build --release

# https://hub.docker.com/r/tnk4on/yt-dlp
FROM docker.io/tnk4on/yt-dlp
USER root:root
RUN mkdir /opt/pervybot/
WORKDIR /opt/pervybot/
COPY --from=build /usr/src/pervybot-rs/target/release/pervybot-rs .
CMD ./pervybot-rs