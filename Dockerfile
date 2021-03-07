FROM ubuntu:latest

RUN apt-get update && apt-get install -y dumb-init
RUN apt-get install -y openssl

ENTRYPOINT ["/usr/bin/dumb-init", "--", "/usr/bin/waterwheel"]

WORKDIR /root

COPY ui/dist/ /root/ui/dist/
COPY target/debug/waterwheel /usr/bin/

CMD ["--help"]
