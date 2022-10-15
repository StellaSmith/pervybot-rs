# Base image can be found here: https://hub.docker.com/r/jrottenberg/ffmpeg/
# There are options for selecting a different distro.
FROM jrottenberg/ffmpeg:3.3
LABEL Name=horny_jail Version=0.0.1
ADD https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp /bin/
WORKDIR /home/root/
COPY /{path to binary} ./
CMD ./{the binary}