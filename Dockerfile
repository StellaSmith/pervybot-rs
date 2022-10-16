# https://hub.docker.com/r/tnk4on/yt-dlp
FROM docker.io/tnk4on/yt-dlp
LABEL Name=pervy_jail Version=0.0.1
RUN mkdir /opt/pervybot/
WORKDIR /opt/pervybot/
COPY /{path to binary} ./
CMD ./{the binary}