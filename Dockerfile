FROM ubuntu:focal

RUN apt-get update && apt-get install -y dumb-init
RUN apt-get install -y openssl

ENTRYPOINT ["/usr/bin/dumb-init", "--", "/usr/bin/waterwheel"]

WORKDIR /root

COPY target/release/waterwheel /usr/bin/

CMD ["--help"]
